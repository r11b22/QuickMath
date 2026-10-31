//! Exporters from a parsed [`Document`] to Obsidian Markdown, LaTeX and PDF.
//!
//! The string renderers are public so the GUI can, for example, copy a note
//! as LaTeX without going through a file.

#[cfg(test)]
mod fixtures;
pub mod latex;
pub mod obsidian;
pub mod pdf;
mod typst_markup;
mod typst_math;

use std::fs;
use std::path::{Path, PathBuf};

use qm_format::{math_to_latex, parse_document, DisplayMath, Document};

use crate::fs_util::{create_dir_all, write_atomically};
use crate::project::{has_note_extension, FileKind, FileNode, Project};
use crate::{CoreError, Result};

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

    /// Renders `document` into the bytes of a file in this format.
    pub fn render(self, document: &Document) -> Result<Vec<u8>> {
        match self {
            ExportFormat::Obsidian => Ok(obsidian::to_markdown(document).into_bytes()),
            ExportFormat::Latex => Ok(latex::to_latex(document).into_bytes()),
            ExportFormat::Pdf => pdf::to_pdf(document),
        }
    }
}

/// Exports one parsed note to `destination` (a file path). Missing parent
/// folders are created and the file is written atomically.
pub fn export_note(document: &Document, format: ExportFormat, destination: &Path) -> Result<()> {
    let bytes = format.render(document)?;
    if let Some(folder) = destination.parent() {
        create_dir_all(folder)?;
    }
    write_atomically(destination, &bytes)
}

/// Exports every note in the project into `destination_folder`, mirroring the
/// folder structure. For Obsidian this produces a ready-to-open vault.
///
/// Notes become `<name>.<extension>`. For Obsidian, all other files (images,
/// PDFs, …) are copied too, so embeds and wikilinks keep working. The
/// destination may lie inside the project (it is then skipped while
/// exporting), but it must not be the project folder or one of its parents,
/// which could overwrite the project's own files.
pub fn export_project(
    project: &Project,
    format: ExportFormat,
    destination_folder: &Path,
) -> Result<()> {
    create_dir_all(destination_folder)?;
    let destination = destination_folder
        .canonicalize()
        .map_err(|error| CoreError::io(destination_folder, error))?;
    if project.root().starts_with(&destination) {
        return Err(CoreError::Export(format!(
            "cannot export into {}: it contains the project itself",
            destination.display()
        )));
    }

    let exporter = ProjectExporter {
        root: project.root(),
        destination: &destination,
        format,
    };
    exporter.export_folder(&project.file_tree()?)
}

/// The LaTeX source of each display-math row, cells joined with ` & `.
/// Shared by the Markdown and LaTeX exporters. Returns no rows when nothing
/// would be visible, and `{}` for empty rows: a blank line inside display
/// math is a LaTeX error.
fn latex_rows(math: &DisplayMath) -> Vec<String> {
    let rows: Vec<String> = math
        .rows
        .iter()
        .map(|row| {
            row.cells
                .iter()
                .map(|cell| math_to_latex(cell).trim().to_string())
                .collect::<Vec<_>>()
                .join(" & ")
                .trim()
                .to_string()
        })
        .collect();
    if rows
        .iter()
        .all(|row| row.replace('&', "").trim().is_empty())
    {
        return Vec::new();
    }
    rows.into_iter()
        .map(|row| {
            if row.is_empty() {
                "{}".to_string()
            } else {
                row
            }
        })
        .collect()
}

/// The text shown for a wikilink outside Obsidian: its label, or the target
/// without `.qm` and with `#heading` shown as `note > heading`.
fn wikilink_text(target: &str, label: Option<&str>) -> String {
    if let Some(label) = label {
        return label.to_string();
    }
    let (path, anchor) = match target.split_once('#') {
        Some((path, anchor)) => (path, Some(anchor)),
        None => (target, None),
    };
    let path = strip_note_extension(path);
    match anchor {
        Some(anchor) => format!("{path} > {anchor}"),
        None => path.to_string(),
    }
}

/// `week1/intro.qm` → `week1/intro`; other names are returned unchanged.
fn strip_note_extension(path: &str) -> &str {
    if has_note_extension(Path::new(path)) {
        &path[..path.len() - ".qm".len()]
    } else {
        path
    }
}

/// Whether any row has more than one cell, i.e. uses alignment markers.
fn is_aligned(math: &DisplayMath) -> bool {
    math.rows.iter().any(|row| row.cells.len() > 1)
}

struct ProjectExporter<'a> {
    root: &'a Path,
    destination: &'a Path,
    format: ExportFormat,
}

impl ProjectExporter<'_> {
    fn export_folder(&self, folder: &FileNode) -> Result<()> {
        let FileKind::Folder(children) = &folder.kind else {
            return Ok(());
        };
        create_dir_all(&self.mirrored_path(&folder.path))?;
        for child in children {
            if child.path == self.destination {
                continue;
            }
            match child.kind {
                FileKind::Folder(_) => self.export_folder(child)?,
                FileKind::Note => self.export_note_file(&child.path)?,
                FileKind::Other if self.format == ExportFormat::Obsidian => {
                    self.copy_file(&child.path)?
                }
                FileKind::Other => {}
            }
        }
        Ok(())
    }

    fn export_note_file(&self, note: &Path) -> Result<()> {
        let source = fs::read_to_string(note).map_err(|error| CoreError::io(note, error))?;
        let document = parse_document(&source);
        let destination = self
            .mirrored_path(note)
            .with_extension(self.format.extension());
        export_note(&document, self.format, &destination).map_err(|error| match error {
            CoreError::Export(message) => {
                CoreError::Export(format!("{}: {message}", self.relative(note).display()))
            }
            other => other,
        })
    }

    fn copy_file(&self, file: &Path) -> Result<()> {
        let destination = self.mirrored_path(file);
        fs::copy(file, &destination)
            .map(drop)
            .map_err(|error| CoreError::io(file, error))
    }

    fn relative<'p>(&self, path: &'p Path) -> &'p Path {
        path.strip_prefix(self.root).unwrap_or(path)
    }

    fn mirrored_path(&self, path: &Path) -> PathBuf {
        self.destination.join(self.relative(path))
    }
}
