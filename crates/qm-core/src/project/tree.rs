//! Scanning a project folder into a [`FileNode`] tree.

use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

use super::paths::{has_note_extension, is_hidden};
use crate::{is_slide_deck, CoreError, Result};

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

impl FileNode {
    /// Calls `visit` for this node and every descendant, parents before children.
    pub fn walk(&self, visit: &mut impl FnMut(&FileNode)) {
        visit(self);
        if let FileKind::Folder(children) = &self.kind {
            for child in children {
                child.walk(visit);
            }
        }
    }

    /// Every slide deck (PDF file) at or below this node, in tree order.
    pub fn slide_decks(&self) -> Vec<PathBuf> {
        let mut decks = Vec::new();
        self.walk(&mut |node| {
            if node.kind == FileKind::Other && is_slide_deck(&node.path) {
                decks.push(node.path.clone());
            }
        });
        decks
    }
}

/// Scans the folder at `path`. Hidden entries (including `.quickmath/`) are
/// skipped, and symlinked folders are not descended into so a symlink loop
/// cannot make the scan run forever.
pub(crate) fn scan_folder(path: &Path) -> Result<FileNode> {
    let mut children = Vec::new();
    let entries = fs::read_dir(path).map_err(|error| CoreError::io(path, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| CoreError::io(path, error))?;
        if is_hidden(&entry.file_name()) {
            continue;
        }
        if let Some(child) = scan_entry(&entry)? {
            children.push(child);
        }
    }
    children.sort_by(compare_nodes);
    Ok(FileNode {
        name: display_name(path),
        path: path.to_path_buf(),
        kind: FileKind::Folder(children),
    })
}

fn scan_entry(entry: &fs::DirEntry) -> Result<Option<FileNode>> {
    let path = entry.path();
    let file_type = entry
        .file_type()
        .map_err(|error| CoreError::io(&path, error))?;
    if file_type.is_dir() {
        return scan_folder(&path).map(Some);
    }
    if file_type.is_symlink() && path.is_dir() {
        return Ok(None);
    }
    let kind = if has_note_extension(&path) {
        FileKind::Note
    } else {
        FileKind::Other
    };
    Ok(Some(FileNode {
        name: display_name(&path),
        path,
        kind,
    }))
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn compare_nodes(a: &FileNode, b: &FileNode) -> Ordering {
    let is_folder = |node: &FileNode| matches!(node.kind, FileKind::Folder(_));
    is_folder(b)
        .cmp(&is_folder(a))
        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        .then_with(|| a.name.cmp(&b.name))
}
