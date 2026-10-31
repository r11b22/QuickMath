//! Per-project UI state, persisted in `.quickmath/workspace.json`.

use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::paths::{resolve_inside, to_slash_path};
use crate::Result;

/// Per-project UI state persisted in `.quickmath/workspace.json`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WorkspaceState {
    pub open_note: Option<PathBuf>,
    pub expanded_folders: Vec<PathBuf>,
}

/// On-disk form: `/`-separated paths relative to the project root, so a
/// project folder can be moved or synced between machines.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct WorkspaceFile {
    open_note: Option<String>,
    expanded_folders: Vec<String>,
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
        Ok(WorkspaceFile {
            open_note: state.open_note.as_deref().map(relative).transpose()?,
            expanded_folders: state
                .expanded_folders
                .iter()
                .map(|path| relative(path))
                .collect::<Result<_>>()?,
        })
    }

    /// Converts back to absolute paths, dropping entries that no longer exist
    /// (or never were valid, e.g. after hand-editing the file).
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
        WorkspaceState {
            open_note,
            expanded_folders,
        }
    }
}

fn existing_entry(root: &Path, relative: &str) -> Option<PathBuf> {
    let relative = Path::new(relative);
    let is_plain_relative = relative
        .components()
        .all(|component| matches!(component, Component::Normal(_)));
    if !is_plain_relative {
        return None;
    }
    let path = root.join(relative);
    let resolved = resolve_inside(root, &path).ok()?;
    resolved.exists().then_some(resolved)
}
