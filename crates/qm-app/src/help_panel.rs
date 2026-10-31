//! The help drawer: the notation cheat sheet (click a row to insert it) and
//! keyboard shortcuts.

use egui::{RichText, Sense};
use qm_format::{CheatSheetKind, MathList};
use qm_render::Theme;

const SHORTCUTS: [(&str, &str); 5] = [
    ("Ctrl+S", "Save now (notes also autosave after 1 s)"),
    ("Tab / Shift+Tab", "Indent / outdent by two spaces"),
    ("Double-click preview", "Jump the editor to that block"),
    (
        "Click [[link]] in preview",
        "Open (or create) the linked note",
    ),
    ("Click a row below", "Insert the notation at the cursor"),
];

/// One cheat-sheet row with its math parsed once.
struct CheatSheetRow {
    input: &'static str,
    description: &'static str,
    /// Rendered preview for math entries; `None` for document markup.
    math: Option<MathList>,
}

#[derive(Default)]
pub struct HelpPanel {
    rows: Option<Vec<CheatSheetRow>>,
}

impl HelpPanel {
    /// Draws the panel contents; returns notation the user clicked to insert.
    pub fn show(&mut self, ui: &mut egui::Ui, theme: &Theme) -> Option<&'static str> {
        let rows = self.rows.get_or_insert_with(load_rows);
        let mut inserted = None;
        ui.heading(RichText::new("Notation").color(theme.text));
        ui.label(
            RichText::new("Write math between ;; … ;; (inline) or start a line with ;; (display).")
                .color(theme.text_faint)
                .small(),
        );
        ui.add_space(4.0);
        egui::ScrollArea::vertical()
            .id_salt("help_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for row in rows.iter() {
                    if show_row(ui, row, theme).clicked() {
                        inserted = Some(row.input);
                    }
                }
                ui.add_space(12.0);
                show_shortcuts(ui, theme);
            });
        inserted
    }
}

fn load_rows() -> Vec<CheatSheetRow> {
    qm_format::cheat_sheet()
        .iter()
        .map(|entry| CheatSheetRow {
            input: entry.input,
            description: entry.description,
            math: match entry.kind {
                CheatSheetKind::Math => Some(qm_format::parse_math(entry.input)),
                CheatSheetKind::Markup => None,
            },
        })
        .collect()
}

fn show_row(ui: &mut egui::Ui, row: &CheatSheetRow, theme: &Theme) -> egui::Response {
    let frame = egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(8, 6))
        .corner_radius(crate::theme_style::CORNER_RADIUS);
    let inner = frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(row.input)
                    .monospace()
                    .color(theme.syntax.math),
            );
            if let Some(math) = &row.math {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    qm_render::show_math(ui, math, false, theme);
                });
            }
        });
        ui.label(
            RichText::new(row.description)
                .color(theme.text_faint)
                .small(),
        );
    });
    let response = inner.response.interact(Sense::click());
    if response.hovered() {
        ui.painter().rect_stroke(
            response.rect,
            crate::theme_style::CORNER_RADIUS,
            egui::Stroke::new(1.0, theme.accent),
            egui::StrokeKind::Inside,
        );
    }
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("Insert at cursor")
}

fn show_shortcuts(ui: &mut egui::Ui, theme: &Theme) {
    ui.heading(RichText::new("Shortcuts").color(theme.text));
    egui::Grid::new("shortcuts_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            for (keys, action) in SHORTCUTS {
                ui.label(RichText::new(keys).monospace().color(theme.text_muted));
                ui.label(RichText::new(action).color(theme.text_faint).small());
                ui.end_row();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_math_entries_and_leaves_markup_as_text() {
        let rows = load_rows();

        let heading = rows.iter().find(|row| row.input.starts_with('#')).unwrap();
        let fraction = rows.iter().find(|row| row.input == "a+b/c+d").unwrap();

        assert!(heading.math.is_none());
        assert!(fraction.math.is_some());
    }
}
