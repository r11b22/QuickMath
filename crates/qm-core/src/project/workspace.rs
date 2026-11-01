//! Per-project UI state, persisted in `.quickmath/workspace.json`.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::paths::{resolve_inside, to_slash_path};
use crate::Result;

/// Per-project UI state persisted in `.quickmath/workspace.json`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WorkspaceState {
    pub open_note: Option<PathBuf>,
    pub expanded_folders: Vec<PathBuf>,
    /// The slide deck linked to each note, by note path.
    pub note_slides: BTreeMap<PathBuf, NoteSlides>,
}

/// The slide deck linked to a note and the page last viewed (zero-based).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteSlides {
    pub deck: PathBuf,
    pub page: usize,
}

/// On-disk form: `/`-separated paths relative to the project root, so a
/// project folder can be moved or synced between machines.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct WorkspaceFile {
    open_note: Option<String>,
    expanded_folders: Vec<String>,
    /// Keyed by note path.
    note_slides: BTreeMap<String, NoteSlidesFile>,
}

#[derive(Debug, Serialize, Deserialize)]
struct NoteSlidesFile {
    deck: String,
    page: usize,
}

impl WorkspaceFile {
    /// Converts absolute (or root-relative) paths to the on-disk form.
    /// Fails with [`crate::CoreError::OutsideProject`] for paths outside `root`.
    pub(crate) fn from_state(root: &Path, state: &WorkspaceState) -> Result<WorkspaceFile> {
        let relative = |path: &Path| -> Result<String> {
            let resolved = resolve_inside(root, path)?;
            let relative = resolved.strip_prefix(root).unwrap_or(Path::new(""));
            Ok(to_slash_path(relative))
        };
        let note_slides = state
            .note_slides
            .iter()
            .map(|(note, slides)| {
                let entry = NoteSlidesFile {
                    deck: relative(&slides.deck)?,
                    page: slides.page,
                };
                Ok((relative(note)?, entry))
            })
            .collect::<Result<_>>()?;
        Ok(WorkspaceFile {
            open_note: state.open_note.as_deref().map(relative).transpose()?,
            expanded_folders: state
                .expanded_folders
                .iter()
                .map(|path| relative(path))
                .collect::<Result<_>>()?,
            note_slides,
        })
    }

    /// Converts back to absolute paths, dropping entries that no longer exist
    /// (or never were valid, e.g. after hand-editing the file).
    ///
    /// A slides link is kept while its note exists, even if the deck is gone,
    /// so the app can report the missing deck when the note is opened.
    pub(crate) fn into_state(self, root: &Path) -> WorkspaceState {
        let open_note = self
            .open_note
            .and_then(|path| existing_entry(root, &path))
            .filter(|path| path.is_file());
        let expanded_folders = self
            .expanded_folders
            .iter()
            .filter_map(|path| existing_entry(root, path))
            .filter(|path| path.is_dir())
            .collect();
        let note_slides = self
            .note_slides
            .into_iter()
            .filter_map(|(note, slides)| {
                let note = existing_entry(root, &note).filter(|path| path.is_file())?;
                let deck = entry_inside(root, &slides.deck)?;
                let page = slides.page;
                Some((note, NoteSlides { deck, page }))
            })
            .collect();
        WorkspaceState {
            open_note,
            expanded_folders,
            note_slides,
        }
    }
}

fn existing_entry(root: &Path, relative: &str) -> Option<PathBuf> {
    entry_inside(root, relative).filter(|path| path.exists())
}

/// `relative` resolved below `root`, whether or not it exists. `None` for
/// paths that are not plain relative paths or that escape the root.
fn entry_inside(root: &Path, relative: &str) -> Option<PathBuf> {
    let relative = Path::new(relative);
    let is_plain_relative = relative
        .components()
        .all(|component| matches!(component, Component::Normal(_)));
    if !is_plain_relative {
        return None;
    }
    resolve_inside(root, &root.join(relative)).ok()
}
