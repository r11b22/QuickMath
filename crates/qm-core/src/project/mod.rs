//! Projects: any folder opened as an Obsidian-style vault of `.qm` notes.

mod paths;
mod tree;
mod wikilink;
mod workspace;

use std::fs;
use std::path::{Path, PathBuf};

use crate::fs_util::{create_dir_all, write_atomically};
use crate::{CoreError, Result, SlideDeck, PROJECT_DIR_NAME};

pub(crate) use paths::has_note_extension;
pub use tree::{FileKind, FileNode};
use workspace::WorkspaceFile;
pub use workspace::{NoteSlides, WorkspaceState};

const TRASH_DIR_NAME: &str = "trash";
const WORKSPACE_FILE_NAME: &str = "workspace.json";

/// An opened project folder. All paths it hands out are absolute.
///
/// Every method that takes a path accepts it either absolute or relative to
/// the project root, and refuses (with [`CoreError::OutsideProject`]) paths
/// that resolve outside the root or into the hidden `.quickmath/` folder.
#[derive(Debug, Clone)]
pub struct Project {
    /// Canonical (symlink-free) path, so containment checks are reliable.
    root: PathBuf,
}

impl Project {
    /// Opens `root` as a project, creating `.quickmath/` (and its `trash/`)
    /// if it is missing.
    ///
    /// Fails if `root` does not exist or is not a folder.
    pub fn open(root: &Path) -> Result<Project> {
        let root = root
            .canonicalize()
            .map_err(|error| CoreError::io(root, error))?;
        if !root.is_dir() {
            let error = std::io::Error::other("not a folder");
            return Err(CoreError::io(&root, error));
        }
        let project = Project { root };
        create_dir_all(&project.trash_dir())?;
        Ok(project)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Display name of the project (the folder name).
    pub fn name(&self) -> String {
        self.root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// Scans the project folder. The returned node is the root folder.
    pub fn file_tree(&self) -> Result<FileNode> {
        tree::scan_folder(&self.root)
    }

    /// Creates an empty note `name` (`.qm` is appended if missing) in `folder`.
    ///
    /// Fails with [`CoreError::InvalidName`] for empty, hidden or path-like
    /// names and with [`CoreError::AlreadyExists`] if the file exists.
    pub fn create_note(&self, folder: &Path, name: &str) -> Result<PathBuf> {
        paths::validate_name(name)?;
        let folder = self.resolve_existing_folder(folder)?;
        let path = folder.join(paths::with_note_extension(name));
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| already_exists_or_io(&path, error))?;
        Ok(path)
    }

    /// Creates the folder `name` inside `parent`.
    pub fn create_folder(&self, parent: &Path, name: &str) -> Result<PathBuf> {
        paths::validate_name(name)?;
        let parent = self.resolve_existing_folder(parent)?;
        let path = parent.join(name);
        fs::create_dir(&path).map_err(|error| already_exists_or_io(&path, error))?;
        Ok(path)
    }

    /// Renames a note or folder in place, returning its new path.
    ///
    /// When renaming a note, `.qm` is appended to `new_name` if missing so
    /// the note keeps being recognised. Never overwrites an existing entry.
    pub fn rename(&self, path: &Path, new_name: &str) -> Result<PathBuf> {
        paths::validate_name(new_name)?;
        let path = self.resolve_entry(path)?;
        let new_file_name = if path.is_file() && has_note_extension(&path) {
            paths::with_note_extension(new_name)
        } else {
            new_name.to_string()
        };
        let new_path = path.with_file_name(new_file_name);
        if new_path == path {
            return Ok(path);
        }
        if new_path.symlink_metadata().is_ok() {
            return Err(CoreError::AlreadyExists(new_path));
        }
        fs::rename(&path, &new_path).map_err(|error| CoreError::io(&path, error))?;
        Ok(new_path)
    }

    /// Moves a note or folder into `.quickmath/trash/` instead of deleting it.
    /// If the trash already holds an entry with that name, a numbered suffix
    /// is added (`notes 2.qm`, `notes 3.qm`, …).
    pub fn trash(&self, path: &Path) -> Result<()> {
        let path = self.resolve_entry(path)?;
        let trash_dir = self.trash_dir();
        create_dir_all(&trash_dir)?;
        let destination = free_path_in(&trash_dir, &path);
        fs::rename(&path, &destination).map_err(|error| CoreError::io(&path, error))
    }

    pub fn read_note(&self, path: &Path) -> Result<String> {
        let path = self.resolve(path)?;
        fs::read_to_string(&path).map_err(|error| CoreError::io(&path, error))
    }

    /// Writes atomically (temp file + rename) so a crash never truncates a note.
    /// Creates the note if it does not exist yet; its folder must exist.
    pub fn write_note(&self, path: &Path, contents: &str) -> Result<()> {
        let path = self.resolve(path)?;
        write_atomically(&path, contents.as_bytes())
    }

    /// Opens the slide deck (a PDF file) at `path` for rendering.
    ///
    /// Fails with [`CoreError::Io`] if the file cannot be read and with
    /// [`CoreError::Slides`] if it is not a readable PDF.
    pub fn open_slides(&self, path: &Path) -> Result<SlideDeck> {
        let path = self.resolve(path)?;
        let bytes = fs::read(&path).map_err(|error| CoreError::io(&path, error))?;
        SlideDeck::from_bytes(&path, bytes)
    }

    /// Resolves a `[[wikilink]]` target the way Obsidian does: by note name
    /// anywhere in the project, or by relative path.
    ///
    /// `[[name]]` matches notes whose file stem equals `name`; `[[dir/name]]`
    /// matches notes whose relative path ends with `dir/name`. Matching is
    /// case-insensitive, `.qm` and `#heading` suffixes are ignored, and the
    /// shortest matching path wins.
    pub fn resolve_wikilink(&self, target: &str) -> Option<PathBuf> {
        let tree = self.file_tree().ok()?;
        wikilink::resolve(&self.root, &tree, target)
    }

    /// Loads the saved UI state. Missing or corrupt state yields the default,
    /// and entries whose files no longer exist are dropped.
    pub fn load_workspace(&self) -> WorkspaceState {
        let file = fs::read_to_string(self.workspace_path())
            .ok()
            .and_then(|json| serde_json::from_str::<WorkspaceFile>(&json).ok())
            .unwrap_or_default();
        file.into_state(&self.root)
    }

    /// Saves the UI state with paths stored relative to the project root.
    pub fn save_workspace(&self, state: &WorkspaceState) -> Result<()> {
        let file = WorkspaceFile::from_state(&self.root, state)?;
        let json = serde_json::to_string_pretty(&file).expect("strings and lists always serialize");
        let path = self.workspace_path();
        create_dir_all(&self.project_dir())?;
        write_atomically(&path, json.as_bytes())
    }

    fn project_dir(&self) -> PathBuf {
        self.root.join(PROJECT_DIR_NAME)
    }

    fn trash_dir(&self) -> PathBuf {
        self.project_dir().join(TRASH_DIR_NAME)
    }

    fn workspace_path(&self) -> PathBuf {
        self.project_dir().join(WORKSPACE_FILE_NAME)
    }

    /// Resolves a user-facing path: inside the root, but not inside `.quickmath/`.
    fn resolve(&self, path: &Path) -> Result<PathBuf> {
        let resolved = paths::resolve_inside(&self.root, path)?;
        if resolved.starts_with(self.project_dir()) {
            return Err(CoreError::OutsideProject(path.to_path_buf()));
        }
        Ok(resolved)
    }

    fn resolve_existing_folder(&self, folder: &Path) -> Result<PathBuf> {
        let folder = self.resolve(folder)?;
        if !folder.is_dir() {
            let error = std::io::Error::new(std::io::ErrorKind::NotFound, "folder not found");
            return Err(CoreError::io(&folder, error));
        }
        Ok(folder)
    }

    /// Resolves an existing note or folder that may be renamed or trashed:
    /// anything in the project except the root itself.
    fn resolve_entry(&self, path: &Path) -> Result<PathBuf> {
        let resolved = self.resolve(path)?;
        if resolved == self.root {
            return Err(CoreError::InvalidName(
                "the project folder itself cannot be renamed or trashed".to_string(),
            ));
        }
        resolved
            .symlink_metadata()
            .map_err(|error| CoreError::io(&resolved, error))?;
        Ok(resolved)
    }
}

fn already_exists_or_io(path: &Path, error: std::io::Error) -> CoreError {
    if error.kind() == std::io::ErrorKind::AlreadyExists {
        CoreError::AlreadyExists(path.to_path_buf())
    } else {
        CoreError::io(path, error)
    }
}

/// A path in `folder` named like `original` that does not exist yet:
/// `name.ext`, then `name 2.ext`, `name 3.ext`, …
fn free_path_in(folder: &Path, original: &Path) -> PathBuf {
    let file_name = original
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let candidate = folder.join(&file_name);
    if candidate.symlink_metadata().is_err() {
        return candidate;
    }
    let (stem, extension) = split_file_name(&file_name, original.is_dir());
    (2u64..)
        .map(|number| folder.join(format!("{stem} {number}{extension}")))
        .find(|candidate| candidate.symlink_metadata().is_err())
        .expect("an unused numbered name always exists")
}

/// Splits `notes.qm` into `("notes", ".qm")`. Folders keep their whole name.
fn split_file_name(file_name: &str, is_folder: bool) -> (&str, &str) {
    match file_name.rfind('.') {
        Some(dot) if !is_folder && dot > 0 => file_name.split_at(dot),
        _ => (file_name, ""),
    }
}
