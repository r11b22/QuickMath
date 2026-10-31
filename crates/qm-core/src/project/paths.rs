//! Name validation and containment checks for paths inside a project.

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

use qm_format::FILE_EXTENSION;

use crate::{CoreError, Result};

/// Rejects names that are empty, hidden, contain path separators or NUL, or
/// would otherwise escape the folder they are created in.
pub(crate) fn validate_name(name: &str) -> Result<()> {
    let is_invalid =
        name.trim().is_empty() || name.starts_with('.') || name.contains(['/', '\\', '\0']);
    if is_invalid {
        return Err(CoreError::InvalidName(name.to_string()));
    }
    Ok(())
}

/// Appends `.qm` to `name` unless it already ends with it.
pub(crate) fn with_note_extension(name: &str) -> String {
    if has_note_extension(Path::new(name)) {
        name.to_string()
    } else {
        format!("{name}.{FILE_EXTENSION}")
    }
}

/// Whether `path` names a `.qm` note (by extension, case-insensitively).
pub(crate) fn has_note_extension(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case(FILE_EXTENSION))
}

/// Whether a directory entry name is hidden (starts with `.`).
pub(crate) fn is_hidden(name: &OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}

/// Resolves `path` (absolute, or relative to `root`) to an absolute path and
/// verifies it lies inside `root`, which must already be canonical.
///
/// The path may not exist yet: the deepest existing ancestor is canonicalized
/// (resolving symlinks and `..`) and the remaining components are appended,
/// which must be plain names. This way neither `..` tricks nor symlinks
/// pointing elsewhere can escape the project.
pub(crate) fn resolve_inside(root: &Path, path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let outside = || CoreError::OutsideProject(path.to_path_buf());

    let (existing, missing) = split_existing_prefix(&absolute).ok_or_else(outside)?;
    let mut resolved = existing
        .canonicalize()
        .map_err(|error| CoreError::io(&existing, error))?;
    for component in missing.components() {
        match component {
            Component::Normal(name) => resolved.push(name),
            Component::CurDir => {}
            _ => return Err(outside()),
        }
    }

    if resolved.starts_with(root) {
        Ok(resolved)
    } else {
        Err(outside())
    }
}

/// Splits `path` into its deepest existing ancestor and the (relative)
/// remainder below it.
fn split_existing_prefix(path: &Path) -> Option<(PathBuf, PathBuf)> {
    let mut existing = path;
    while existing.symlink_metadata().is_err() {
        existing = existing.parent()?;
    }
    let missing = path.strip_prefix(existing).ok()?.to_path_buf();
    Some((existing.to_path_buf(), missing))
}

/// The `/`-separated form of a path relative to the project root, as stored
/// in `workspace.json` and compared by wikilink resolution.
pub(crate) fn to_slash_path(relative: &Path) -> String {
    relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_name_rejects_invalid_names() {
        for name in ["", "   ", "a/b", "a\\b", "..", ".", ".hidden", "nul\0byte"] {
            assert!(validate_name(name).is_err(), "{name:?} should be rejected");
        }
    }

    #[test]
    fn validate_name_accepts_ordinary_and_unicode_names() {
        for name in ["Lecture 1", "lineare-algebra.qm", "Übung α", "a..b"] {
            assert!(validate_name(name).is_ok(), "{name:?} should be accepted");
        }
    }

    #[test]
    fn with_note_extension_appends_only_when_missing() {
        assert_eq!(with_note_extension("notes"), "notes.qm");
        assert_eq!(with_note_extension("notes.qm"), "notes.qm");
        assert_eq!(with_note_extension("notes.QM"), "notes.QM");
        assert_eq!(with_note_extension("v1.2"), "v1.2.qm");
    }
}
