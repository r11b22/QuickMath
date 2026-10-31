//! QuickMath backend: projects (Obsidian-style vault folders), note storage,
//! app settings and exporters.
//!
//! The GUI talks to the file system and exporters only through this crate.

use std::fmt;
use std::path::{Path, PathBuf};

use qm_format::Document;

/// Name of the hidden folder that marks a directory as a QuickMath project
/// (like Obsidian's `.obsidian/`).
pub const PROJECT_DIR_NAME: &str = ".quickmath";

#[derive(Debug)]
pub enum CoreError {
    Io { path: PathBuf, source: std::io::Error },
    InvalidName(String),
    AlreadyExists(PathBuf),
    OutsideProject(PathBuf),
    Export(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::Io { path, source } => write!(f, "{}: {source}", path.display()),
            CoreError::InvalidName(name) => write!(f, "invalid name: {name:?}"),
            CoreError::AlreadyExists(path) => write!(f, "{} already exists", path.display()),
            CoreError::OutsideProject(path) => write!(f, "{} is outside the project", path.display()),
            CoreError::Export(message) => write!(f, "export failed: {message}"),
        }
    }
}

impl std::error::Error for CoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CoreError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, CoreError>;

/// A node in the project's file tree. Hidden entries (starting with `.`) are omitted.
#[derive(Debug, Clone, PartialEq)]
pub struct FileNode {
    pub name: String,
    pub path: PathBuf,
    pub kind: FileKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileKind {
    /// Children sorted folders first, then case-insensitively by name.
    Folder(Vec<FileNode>),
    Note,
    /// Any non-`.qm` file (images, PDFs, …).
    Other,
}

/// Per-project UI state persisted in `.quickmath/workspace.json`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WorkspaceState {
    pub open_note: Option<PathBuf>,
    pub expanded_folders: Vec<PathBuf>,
}

/// An opened project folder. All paths it hands out are absolute.
#[derive(Debug, Clone)]
pub struct Project {
    root: PathBuf,
}

impl Project {
    /// Opens `root` as a project, creating `.quickmath/` if it is missing.
    pub fn open(root: &Path) -> Result<Project> {
        // CONTRACT STUB: replaced by the backend agent.
        Ok(Project { root: root.to_path_buf() })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Display name of the project (the folder name).
    pub fn name(&self) -> String {
        self.root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    }

    /// Scans the project folder. The returned node is the root folder.
    pub fn file_tree(&self) -> Result<FileNode> {
        unimplemented!("CONTRACT STUB: backend agent")
    }

    /// Creates an empty note `name` (`.qm` is appended if missing) in `folder`.
    pub fn create_note(&self, folder: &Path, name: &str) -> Result<PathBuf> {
        let _ = (folder, name);
        unimplemented!("CONTRACT STUB: backend agent")
    }

    pub fn create_folder(&self, parent: &Path, name: &str) -> Result<PathBuf> {
        let _ = (parent, name);
        unimplemented!("CONTRACT STUB: backend agent")
    }

    /// Renames a note or folder in place, returning its new path.
    pub fn rename(&self, path: &Path, new_name: &str) -> Result<PathBuf> {
        let _ = (path, new_name);
        unimplemented!("CONTRACT STUB: backend agent")
    }

    /// Moves a note or folder into `.quickmath/trash/` instead of deleting it.
    pub fn trash(&self, path: &Path) -> Result<()> {
        let _ = path;
        unimplemented!("CONTRACT STUB: backend agent")
    }

    pub fn read_note(&self, path: &Path) -> Result<String> {
        let _ = path;
        unimplemented!("CONTRACT STUB: backend agent")
    }

    /// Writes atomically (temp file + rename) so a crash never truncates a note.
    pub fn write_note(&self, path: &Path, contents: &str) -> Result<()> {
        let _ = (path, contents);
        unimplemented!("CONTRACT STUB: backend agent")
    }

    /// Resolves a `[[wikilink]]` target the way Obsidian does: by note name
    /// anywhere in the project, or by relative path.
    pub fn resolve_wikilink(&self, target: &str) -> Option<PathBuf> {
        let _ = target;
        unimplemented!("CONTRACT STUB: backend agent")
    }

    pub fn load_workspace(&self) -> WorkspaceState {
        unimplemented!("CONTRACT STUB: backend agent")
    }

    pub fn save_workspace(&self, state: &WorkspaceState) -> Result<()> {
        let _ = state;
        unimplemented!("CONTRACT STUB: backend agent")
    }
}

/// App-wide settings stored in the user's config directory.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AppSettings {
    /// Most recent first, without duplicates.
    pub recent_projects: Vec<PathBuf>,
    pub dark_mode: bool,
}

impl AppSettings {
    /// Loads settings, falling back to defaults when the file is missing or unreadable.
    pub fn load() -> AppSettings {
        unimplemented!("CONTRACT STUB: backend agent")
    }

    pub fn save(&self) -> Result<()> {
        unimplemented!("CONTRACT STUB: backend agent")
    }

    /// Moves `root` to the front of the recent-projects list.
    pub fn remember_project(&mut self, root: &Path) {
        let _ = root;
        unimplemented!("CONTRACT STUB: backend agent")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Obsidian-flavoured Markdown (`$…$` MathJax, callouts, wikilinks).
    Obsidian,
    Latex,
    Pdf,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Obsidian => "md",
            ExportFormat::Latex => "tex",
            ExportFormat::Pdf => "pdf",
        }
    }
}

/// Exports one parsed note to `destination` (a file path).
pub fn export_note(document: &Document, format: ExportFormat, destination: &Path) -> Result<()> {
    let _ = (document, format, destination);
    unimplemented!("CONTRACT STUB: backend agent")
}

/// Exports every note in the project into `destination_folder`, mirroring the
/// folder structure. For Obsidian this produces a ready-to-open vault.
pub fn export_project(project: &Project, format: ExportFormat, destination_folder: &Path) -> Result<()> {
    let _ = (project, format, destination_folder);
    unimplemented!("CONTRACT STUB: backend agent")
}
