//! The bottom status bar: save state, cursor position, word count, file path.

use std::path::Path;

use egui::{Align, Layout, RichText};
use qm_render::Theme;

use crate::note::OpenNote;

/// egui's `small()` text is too small to read at a glance in the status bar.
const STATUS_FONT_SIZE: f32 = 12.5;

/// Draws the status bar for `note` (or an empty bar when no note is open).
pub fn show_status_bar(
    ui: &mut egui::Ui,
    note: Option<&OpenNote>,
    project_root: &Path,
    theme: &Theme,
) {
    ui.horizontal_centered(|ui| {
        let Some(note) = note else {
            ui.label(
                RichText::new("No note open")
                    .color(theme.text_faint)
                    .size(STATUS_FONT_SIZE),
            );
            return;
        };
        let (state_label, state_color) = if note.is_dirty() {
            ("● Unsaved changes", theme.text_muted)
        } else {
            ("✔ Saved", theme.accent_text)
        };
        ui.label(
            RichText::new(state_label)
                .color(state_color)
                .size(STATUS_FONT_SIZE),
        );
        ui.separator();
        let cursor = note.cursor;
        ui.label(
            RichText::new(format!("Ln {}, Col {}", cursor.line + 1, cursor.column + 1))
                .color(theme.text_muted)
                .size(STATUS_FONT_SIZE),
        );
        ui.separator();
        ui.label(
            RichText::new(word_count_label(note.word_count()))
                .color(theme.text_muted)
                .size(STATUS_FONT_SIZE),
        );

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let relative = note
                .path()
                .strip_prefix(project_root)
                .unwrap_or(note.path());
            ui.label(
                RichText::new(relative.display().to_string())
                    .color(theme.text_faint)
                    .size(STATUS_FONT_SIZE),
            );
        });
    });
}

fn word_count_label(count: usize) -> String {
    if count == 1 {
        "1 word".to_string()
    } else {
        format!("{count} words")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_count_label_pluralises() {
        assert_eq!(word_count_label(0), "0 words");
        assert_eq!(word_count_label(1), "1 word");
        assert_eq!(word_count_label(42), "42 words");
    }
}
