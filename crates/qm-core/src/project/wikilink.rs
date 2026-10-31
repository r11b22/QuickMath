//! Obsidian-style `[[wikilink]]` resolution.

use std::path::{Path, PathBuf};

use super::paths::{has_note_extension, to_slash_path};
use super::tree::{FileKind, FileNode};

/// Strips the parts of a wikilink target that do not name a file: a
/// `#heading` / `#^block` anchor, a `|label`, surrounding whitespace, a
/// leading `/` and an optional `.qm` suffix. Backslashes are treated as `/`.
pub(crate) fn normalize_target(target: &str) -> String {
    let without_label = target.split('|').next().unwrap_or_default();
    let without_anchor = without_label.split('#').next().unwrap_or_default();
    let normalized = without_anchor.trim().replace('\\', "/");
    let normalized = normalized.trim_start_matches('/');
    match normalized.len().checked_sub(3) {
        Some(stem_len) if has_note_extension(Path::new(normalized)) => {
            normalized[..stem_len].to_string()
        }
        _ => normalized.to_string(),
    }
}

/// Finds the note a wikilink points to, using Obsidian's rules: a bare name
/// matches any note with that file stem, a name with `/` matches the end of a
/// note's relative path. Matching ignores case; among several candidates the
/// one with the shortest path wins (ties broken alphabetically).
pub(crate) fn resolve(root: &Path, tree: &FileNode, target: &str) -> Option<PathBuf> {
    let wanted = normalize_target(target).to_lowercase();
    if wanted.is_empty() {
        return None;
    }
    let wanted_suffix = format!("/{wanted}");

    let mut candidates = Vec::new();
    tree.walk(&mut |node| {
        if node.kind != FileKind::Note {
            return;
        }
        let Ok(relative) = node.path.strip_prefix(root) else {
            return;
        };
        let key = to_slash_path(&relative.with_extension("")).to_lowercase();
        let matches = if wanted.contains('/') {
            key == wanted || key.ends_with(&wanted_suffix)
        } else {
            key.rsplit('/').next() == Some(wanted.as_str())
        };
        if matches {
            candidates.push((relative.components().count(), key, node.path.clone()));
        }
    });
    candidates.sort();
    candidates.into_iter().next().map(|(_, _, path)| path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_target_strips_anchor_label_and_extension() {
        assert_eq!(normalize_target("Topology#Open sets"), "Topology");
        assert_eq!(normalize_target(" week1/intro.qm "), "week1/intro");
        assert_eq!(normalize_target("a|label"), "a");
        assert_eq!(normalize_target("/abs\\path"), "abs/path");
        assert_eq!(normalize_target("#only-heading"), "");
        assert_eq!(normalize_target("Shouting.QM"), "Shouting");
    }
}
