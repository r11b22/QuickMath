//! Suggestions for the math word being typed: a list under the cursor fed by
//! `qm_format::math_completions`. Tab accepts, Up/Down choose and Escape
//! hides. Enter and Space are never taken, so typing on never picks one.

use std::hash::{Hash, Hasher};
use std::ops::Range;

use egui::text::CCursor;
use egui::widgets::text_edit::TextEditOutput;
use egui::{Id, InputState, Key, Modifiers, RichText};
use qm_format::MathList;
use qm_render::Theme;

use crate::text_metrics;

const POPUP_WIDTH: f32 = 280.0;
const POPUP_GAP: f32 = 2.0;

/// A suggestion the user accepted: replace `char_range` of the text with `spelling`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Acceptance {
    pub char_range: Range<usize>,
    pub spelling: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    Next,
    Previous,
    Accept,
    Dismiss,
}

struct Suggestions {
    /// Character range of the partly typed word.
    char_range: Range<usize>,
    rows: Vec<Row>,
}

struct Row {
    spelling: &'static str,
    section: &'static str,
    /// The syntax file's example for the word, parsed once for rendering.
    math: MathList,
}

#[derive(Default)]
pub struct CompletionPopup {
    suggestions: Option<Suggestions>,
    selected: usize,
    /// Word hidden with Escape; suggestions return once the word changes.
    dismissed: Option<Range<usize>>,
    /// Hash of the text and cursor the suggestions were computed for.
    computed_for: Option<u64>,
}

impl CompletionPopup {
    /// Recomputes the suggestions if the text or cursor changed. `cursor` is
    /// a character offset, or `None` when there is a selection or no cursor.
    pub fn update(&mut self, text: &str, cursor: Option<usize>) {
        let key = state_hash(text, cursor);
        if self.computed_for == Some(key) {
            return;
        }
        self.computed_for = Some(key);
        self.selected = 0;
        self.suggestions = cursor.and_then(|cursor| suggestions_at(text, cursor));
        let word = self.suggestions.as_ref().map(|s| &s.char_range);
        if self.dismissed.as_ref() != word {
            self.dismissed = None;
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible_suggestions().is_some()
    }

    /// Consumes the popup's keys while it is visible and returns the
    /// suggestion accepted with Tab. Call before the text edit sees the keys.
    pub fn handle_keys(&mut self, ctx: &egui::Context) -> Option<Acceptance> {
        self.visible_suggestions()?;
        let command = ctx.input_mut(read_command)?;
        self.apply(command)
    }

    /// Draws the list just below the start of the typed word, if that word
    /// is inside `visible_rect` (the editor's scroll viewport).
    pub fn show(
        &self,
        ctx: &egui::Context,
        id: Id,
        output: &TextEditOutput,
        visible_rect: egui::Rect,
        theme: &Theme,
    ) {
        let Some(suggestions) = self.visible_suggestions() else {
            return;
        };
        let word_rect = output
            .galley
            .pos_from_cursor(CCursor::new(suggestions.char_range.start))
            .translate(output.galley_pos.to_vec2());
        if !visible_rect.intersects(word_rect) {
            return;
        }
        egui::Area::new(id)
            .order(egui::Order::Foreground)
            .fixed_pos(word_rect.left_bottom() + egui::vec2(0.0, POPUP_GAP))
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_width(POPUP_WIDTH);
                    for (index, row) in suggestions.rows.iter().enumerate() {
                        show_row(ui, row, index == self.selected, theme);
                    }
                });
            });
    }

    fn visible_suggestions(&self) -> Option<&Suggestions> {
        self.suggestions
            .as_ref()
            .filter(|suggestions| self.dismissed.as_ref() != Some(&suggestions.char_range))
    }

    fn apply(&mut self, command: Command) -> Option<Acceptance> {
        let suggestions = self.visible_suggestions()?;
        let count = suggestions.rows.len();
        let char_range = suggestions.char_range.clone();
        let spelling = suggestions.rows[self.selected].spelling;
        match command {
            Command::Next => self.selected = (self.selected + 1) % count,
            Command::Previous => self.selected = (self.selected + count - 1) % count,
            Command::Dismiss => self.dismissed = Some(char_range),
            Command::Accept => {
                self.suggestions = None;
                return Some(Acceptance {
                    char_range,
                    spelling,
                });
            }
        }
        None
    }
}

fn read_command(input: &mut InputState) -> Option<Command> {
    let bindings = [
        (Key::ArrowDown, Command::Next),
        (Key::ArrowUp, Command::Previous),
        (Key::Tab, Command::Accept),
        (Key::Escape, Command::Dismiss),
    ];
    bindings
        .into_iter()
        .find(|(key, _)| input.consume_key(Modifiers::NONE, *key))
        .map(|(_, command)| command)
}

fn suggestions_at(text: &str, cursor: usize) -> Option<Suggestions> {
    let byte_cursor = text_metrics::byte_offset_of_char(text, cursor);
    let completions = qm_format::math_completions(text, byte_cursor)?;
    let start = text[..completions.range.start].chars().count();
    // The typed word is ASCII letters, so bytes and characters agree.
    let char_range = start..start + completions.range.len();
    let rows = completions
        .candidates
        .iter()
        .map(|candidate| Row {
            spelling: candidate.entry.spelling,
            section: candidate.section,
            math: qm_format::parse_math(candidate.entry.example),
        })
        .collect();
    Some(Suggestions { char_range, rows })
}

fn state_hash(text: &str, cursor: Option<usize>) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    cursor.hash(&mut hasher);
    hasher.finish()
}

fn show_row(ui: &mut egui::Ui, row: &Row, is_selected: bool, theme: &Theme) {
    let fill = if is_selected {
        theme.accent_soft
    } else {
        egui::Color32::TRANSPARENT
    };
    egui::Frame::new()
        .fill(fill)
        .inner_margin(egui::Margin::symmetric(8, 2))
        .corner_radius(crate::theme_style::CORNER_RADIUS)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(row.spelling)
                        .monospace()
                        .color(theme.syntax.math),
                );
                qm_render::show_math(ui, &row.math, false, theme);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(row.section).color(theme.text_faint).small());
                });
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn popup_for(text: &str) -> CompletionPopup {
        let mut popup = CompletionPopup::default();
        popup.update(text, Some(text.chars().count()));
        popup
    }

    fn selected_spelling(popup: &CompletionPopup) -> &'static str {
        popup.visible_suggestions().unwrap().rows[popup.selected].spelling
    }

    #[test]
    fn word_range_is_in_characters() {
        let popup = popup_for("αβ ;;alp");
        assert_eq!(popup.visible_suggestions().unwrap().char_range, 5..8);
    }

    #[test]
    fn nothing_is_suggested_for_a_selection_or_outside_math() {
        let mut popup = CompletionPopup::default();
        popup.update(";;alp", None);
        assert!(popup.visible_suggestions().is_none());
        assert!(popup_for("alp").visible_suggestions().is_none());
    }

    #[test]
    fn tab_accepts_the_first_suggestion_by_default() {
        let mut popup = popup_for(";;alp");
        let acceptance = popup.apply(Command::Accept);
        assert_eq!(
            acceptance,
            Some(Acceptance {
                char_range: 2..5,
                spelling: "alpha",
            })
        );
        assert!(popup.visible_suggestions().is_none());
    }

    #[test]
    fn arrows_move_the_selection_and_wrap() {
        let mut popup = popup_for(";;sq");
        assert_eq!(selected_spelling(&popup), "sq");
        popup.apply(Command::Next);
        assert_eq!(selected_spelling(&popup), "sqrt");
        popup.apply(Command::Next);
        assert_eq!(selected_spelling(&popup), "sq");
        popup.apply(Command::Previous);
        assert_eq!(selected_spelling(&popup), "sqrt");
    }

    #[test]
    fn selection_survives_frames_without_changes() {
        let mut popup = popup_for(";;sq");
        popup.apply(Command::Next);
        popup.update(";;sq", Some(4));
        assert_eq!(selected_spelling(&popup), "sqrt");
    }

    #[test]
    fn escape_hides_until_the_word_changes() {
        let mut popup = popup_for(";;al");
        assert_eq!(popup.apply(Command::Dismiss), None);
        assert!(popup.visible_suggestions().is_none());
        popup.update(";;al ", Some(5));
        popup.update(";;al", Some(4));
        assert!(popup.visible_suggestions().is_some());
    }

    #[test]
    fn escape_keeps_the_list_hidden_while_the_word_stays() {
        let mut popup = popup_for(";;al");
        popup.apply(Command::Dismiss);
        popup.update(";;al;;", Some(4));
        assert!(popup.visible_suggestions().is_none());
    }

    #[test]
    fn hidden_popup_ignores_commands() {
        let mut popup = popup_for("plain text");
        assert_eq!(popup.apply(Command::Accept), None);
    }
}
