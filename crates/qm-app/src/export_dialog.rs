//! The export dialog and the background thread that runs exports.

use std::path::{Path, PathBuf};
use std::thread::JoinHandle;

use egui::RichText;
use qm_core::{ExportFormat, Project};
use qm_format::Document;
use qm_render::Theme;

const FORMATS: [(ExportFormat, &str, &str); 3] = [
    (
        ExportFormat::Obsidian,
        "Obsidian Markdown",
        "Markdown with $…$ math, callouts and wikilinks",
    ),
    (
        ExportFormat::Latex,
        "LaTeX",
        "A .tex source file to compile or hand in",
    ),
    (
        ExportFormat::Pdf,
        "PDF",
        "A typeset document, ready to print",
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExportScope {
    CurrentNote,
    WholeProject,
}

/// What can be exported right now.
pub struct ExportSource<'a> {
    pub project: &'a Project,
    /// The open note's parsed document and title, if a note is open.
    pub note: Option<(&'a Document, String)>,
}

/// The result of a finished export, for the notification area.
#[derive(Debug)]
pub enum ExportOutcome {
    Succeeded(PathBuf),
    Failed(String),
}

struct ExportJob {
    handle: JoinHandle<qm_core::Result<()>>,
    destination: PathBuf,
}

pub struct ExportDialog {
    is_open: bool,
    format: ExportFormat,
    scope: ExportScope,
    job: Option<ExportJob>,
}

impl Default for ExportDialog {
    fn default() -> Self {
        ExportDialog {
            is_open: false,
            format: ExportFormat::Pdf,
            scope: ExportScope::CurrentNote,
            job: None,
        }
    }
}

impl ExportDialog {
    pub fn open(&mut self) {
        self.is_open = true;
    }

    /// Draws the dialog when open. May start an export.
    pub fn show(&mut self, ctx: &egui::Context, source: &ExportSource, theme: &Theme) {
        if !self.is_open {
            return;
        }
        if source.note.is_none() {
            self.scope = ExportScope::WholeProject;
        }
        let modal = egui::Modal::new(egui::Id::new("export_dialog")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.heading(RichText::new("Export").color(theme.text));
            ui.add_space(8.0);
            self.format_choice(ui, theme);
            ui.add_space(8.0);
            self.scope_choice(ui, source);
            ui.add_space(12.0);
            ui.separator();
            self.footer(ui, source, theme);
        });
        if modal.should_close() {
            self.is_open = false;
        }
    }

    fn format_choice(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        ui.label(RichText::new("Format").strong());
        for (format, name, description) in FORMATS {
            ui.radio_value(&mut self.format, format, name);
            ui.indent(name, |ui| {
                ui.label(RichText::new(description).color(theme.text_faint).small())
            });
        }
    }

    fn scope_choice(&mut self, ui: &mut egui::Ui, source: &ExportSource) {
        ui.label(RichText::new("What to export").strong());
        ui.add_enabled_ui(source.note.is_some(), |ui| {
            let label = match &source.note {
                Some((_, title)) => format!("Current note ({title})"),
                None => "Current note (none open)".to_string(),
            };
            ui.radio_value(&mut self.scope, ExportScope::CurrentNote, label);
        });
        ui.radio_value(
            &mut self.scope,
            ExportScope::WholeProject,
            format!("Whole project ({})", source.project.name()),
        );
    }

    fn footer(&mut self, ui: &mut egui::Ui, source: &ExportSource, theme: &Theme) {
        ui.horizontal(|ui| {
            if self.is_running() {
                ui.spinner();
                ui.label(RichText::new("Exporting…").color(theme.text_muted));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let export =
                    egui::Button::new(RichText::new("Export…").color(theme.on_accent).strong())
                        .fill(theme.accent);
                if ui.add_enabled(!self.is_running(), export).clicked() {
                    self.start(source);
                }
                if ui.button("Close").clicked() {
                    self.is_open = false;
                }
            });
        });
    }

    pub fn is_running(&self) -> bool {
        self.job.is_some()
    }

    /// Asks for a destination and starts the export on a background thread.
    fn start(&mut self, source: &ExportSource) {
        let format = self.format;
        let job = match (self.scope, &source.note) {
            (ExportScope::CurrentNote, Some((document, title))) => {
                let Some(destination) = pick_note_destination(source.project.root(), title, format)
                else {
                    return;
                };
                let document = (*document).clone();
                let target = destination.clone();
                let handle =
                    std::thread::spawn(move || qm_core::export_note(&document, format, &target));
                ExportJob {
                    handle,
                    destination,
                }
            }
            _ => {
                let Some(destination) = pick_project_destination(source.project.root()) else {
                    return;
                };
                let project = source.project.clone();
                let target = destination.clone();
                let handle =
                    std::thread::spawn(move || qm_core::export_project(&project, format, &target));
                ExportJob {
                    handle,
                    destination,
                }
            }
        };
        self.job = Some(job);
    }

    /// Collects a finished export. Call every frame; keeps the UI repainting while one runs.
    pub fn poll(&mut self, ctx: &egui::Context) -> Option<ExportOutcome> {
        let job = self.job.as_ref()?;
        if !job.handle.is_finished() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return None;
        }
        let job = self.job.take()?;
        let outcome = match job.handle.join() {
            Ok(Ok(())) => ExportOutcome::Succeeded(job.destination),
            Ok(Err(error)) => ExportOutcome::Failed(error.to_string()),
            Err(_) => ExportOutcome::Failed("the exporter crashed unexpectedly".to_string()),
        };
        if matches!(outcome, ExportOutcome::Succeeded(_)) {
            self.is_open = false;
        }
        Some(outcome)
    }
}

fn pick_note_destination(
    project_root: &Path,
    title: &str,
    format: ExportFormat,
) -> Option<PathBuf> {
    let extension = format.extension();
    let path = rfd::FileDialog::new()
        .set_title("Export note")
        .set_directory(project_root)
        .set_file_name(format!("{title}.{extension}"))
        .add_filter(format_name(format), &[extension])
        .save_file()?;
    Some(with_extension_if_missing(path, extension))
}

fn pick_project_destination(project_root: &Path) -> Option<PathBuf> {
    let start = project_root.parent().unwrap_or(project_root);
    rfd::FileDialog::new()
        .set_title("Export project into folder")
        .set_directory(start)
        .pick_folder()
}

fn format_name(format: ExportFormat) -> &'static str {
    FORMATS
        .iter()
        .find(|(candidate, _, _)| *candidate == format)
        .map_or("Export", |(_, name, _)| name)
}

/// Some platform dialogs return the typed name without the filter's extension.
fn with_extension_if_missing(path: PathBuf, extension: &str) -> PathBuf {
    if path
        .extension()
        .is_some_and(|existing| existing.eq_ignore_ascii_case(extension))
    {
        path
    } else {
        let mut name = path.into_os_string();
        name.push(".");
        name.push(extension);
        PathBuf::from(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_missing_extension() {
        assert_eq!(
            with_extension_if_missing(PathBuf::from("/tmp/notes"), "pdf"),
            PathBuf::from("/tmp/notes.pdf")
        );
        assert_eq!(
            with_extension_if_missing(PathBuf::from("/tmp/v1.2"), "tex"),
            PathBuf::from("/tmp/v1.2.tex")
        );
    }

    #[test]
    fn keeps_matching_extension_case_insensitively() {
        assert_eq!(
            with_extension_if_missing(PathBuf::from("/tmp/a.PDF"), "pdf"),
            PathBuf::from("/tmp/a.PDF")
        );
    }

    #[test]
    fn every_format_has_a_display_name() {
        for format in [
            ExportFormat::Obsidian,
            ExportFormat::Latex,
            ExportFormat::Pdf,
        ] {
            assert_ne!(format_name(format), "Export");
        }
    }
}
