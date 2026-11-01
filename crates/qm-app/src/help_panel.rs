//! The help drawer: the notation cheat sheet and every symbol from the
//! syntax file (click a row or symbol to insert it), and keyboard shortcuts.

use egui::{RichText, Sense};
use qm_format::{CheatSheetKind, MathList};
use qm_render::Theme;

const SHORTCUTS: [(&str, &str); 6] = [
    ("Ctrl+S", "Save now (notes also autosave after 1 s)"),
    ("Tab / Shift+Tab", "Indent / outdent by two spaces"),
    (
        "Tab in math",
        "Complete the suggested word (Up/Down choose, Esc hides)",
    ),
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

/// A section of the syntax file, e.g. "Integrals".
struct SymbolGroup {
    title: &'static str,
    symbols: Vec<SymbolEntry>,
}

/// One word or operator with its example parsed once.
struct SymbolEntry {
    spelling: &'static str,
    /// Inserted on click, e.g. `abs x` for `abs`.
    example: &'static str,
    math: MathList,
}

#[derive(Default)]
pub struct HelpPanel {
    rows: Option<Vec<CheatSheetRow>>,
    symbol_groups: Option<Vec<SymbolGroup>>,
}

impl HelpPanel {
    /// Draws the panel contents; returns notation the user clicked to insert.
    pub fn show(&mut self, ui: &mut egui::Ui, theme: &Theme) -> Option<&'static str> {
        let rows = self.rows.get_or_insert_with(load_rows);
        let symbol_groups = self.symbol_groups.get_or_insert_with(load_symbol_groups);
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
                if let Some(example) = show_symbol_groups(ui, symbol_groups, theme) {
                    inserted = Some(example);
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

fn load_symbol_groups() -> Vec<SymbolGroup> {
    qm_format::vocabulary()
        .iter()
        .map(|section| SymbolGroup {
            title: section.title,
            symbols: section
                .entries
                .iter()
                .map(|entry| SymbolEntry {
                    spelling: entry.spelling,
                    example: entry.example,
                    math: qm_format::parse_math(entry.example),
                })
                .collect(),
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
    make_clickable(ui, inner.response, theme)
}

/// Lists every symbol from the syntax file in collapsible sections; returns
/// the example of the symbol the user clicked.
fn show_symbol_groups(
    ui: &mut egui::Ui,
    groups: &[SymbolGroup],
    theme: &Theme,
) -> Option<&'static str> {
    ui.heading(RichText::new("Symbols").color(theme.text));
    ui.label(
        RichText::new("Every word and operator defined in syntax.toml.")
            .color(theme.text_faint)
            .small(),
    );
    let mut inserted = None;
    for group in groups {
        egui::CollapsingHeader::new(RichText::new(group.title).color(theme.text_muted))
            .id_salt(group.title)
            .show(ui, |ui| {
                for symbol in &group.symbols {
                    if show_symbol_entry(ui, symbol, theme).clicked() {
                        inserted = Some(symbol.example);
                    }
                }
            });
    }
    inserted
}

fn show_symbol_entry(ui: &mut egui::Ui, symbol: &SymbolEntry, theme: &Theme) -> egui::Response {
    let frame = egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(8, 2))
        .corner_radius(crate::theme_style::CORNER_RADIUS);
    let inner = frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(symbol.spelling)
                    .monospace()
                    .color(theme.syntax.math),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                qm_render::show_math(ui, &symbol.math, false, theme);
            });
        });
    });
    make_clickable(ui, inner.response, theme)
}

/// Outlines `response` on hover and marks it as inserting at the cursor.
fn make_clickable(ui: &egui::Ui, response: egui::Response, theme: &Theme) -> egui::Response {
    let response = response.interact(Sense::click());
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

    #[test]
    fn lists_symbols_from_the_syntax_file_with_their_examples() {
        let groups = load_symbol_groups();

        let integrals = groups
            .iter()
            .find(|group| group.title == "Integrals")
            .unwrap();
        let fences = groups.iter().find(|group| group.title == "Fences").unwrap();
        let double_integral = integrals
            .symbols
            .iter()
            .find(|symbol| symbol.spelling == "integral2")
            .unwrap();
        let abs = fences
            .symbols
            .iter()
            .find(|symbol| symbol.spelling == "abs")
            .unwrap();

        assert_eq!(double_integral.example, "integral2");
        assert!(!double_integral.math.is_empty());
        assert_eq!(abs.example, "abs x");
    }
}
