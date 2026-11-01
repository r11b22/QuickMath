//! Misspelled words in the editor: which words they are, the wavy line
//! under them, and the right-click menu with corrections.

use std::hash::{Hash, Hasher};
use std::ops::Range;

use egui::text::CCursor;
use egui::widgets::text_edit::TextEditOutput;
use egui::{Color32, Popup, RichText, SetOpenCommand, Shape, Stroke};
use qm_core::SpellChecker;
use qm_render::Theme;

use crate::spelling::SpellCheck;

const WAVE_AMPLITUDE: f32 = 1.2;
const WAVE_HALF_LENGTH: f32 = 2.0;
/// Space between the text's row bottom and the middle of the wave.
const WAVE_LIFT: f32 = 1.5;
const WAVE_STROKE_WIDTH: f32 = 1.0;

/// What the user picked in the right-click menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MisspellingChoice {
    /// Replace the characters in `char_range` with `replacement`.
    Replace {
        char_range: Range<usize>,
        replacement: String,
    },
    AddToDictionary(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Misspelling {
    /// Character range (not bytes), as egui's text cursor counts.
    char_range: Range<usize>,
    word: String,
}

struct OpenMenu {
    misspelling: Misspelling,
    suggestions: Vec<String>,
}

#[derive(Default)]
pub struct Misspellings {
    words: Vec<Misspelling>,
    /// Hash of the text and spell checker revision `words` was found for.
    computed_for: Option<u64>,
    menu: Option<OpenMenu>,
}

impl Misspellings {
    /// Re-checks `text` if it or the spell checker changed since the last call.
    pub fn update(&mut self, text: &str, spell_check: Option<SpellCheck>) {
        let key = state_hash(text, spell_check.map(|spell_check| spell_check.revision));
        if self.computed_for == Some(key) {
            return;
        }
        self.computed_for = Some(key);
        self.words = spell_check
            .map(|spell_check| find_misspellings(text, spell_check.checker))
            .unwrap_or_default();
    }

    /// Draws a wavy line under each visible misspelled word, except the word
    /// at `cursor` (a character offset), which may still be being typed.
    pub fn paint(
        &self,
        ui: &egui::Ui,
        output: &TextEditOutput,
        cursor: Option<usize>,
        color: Color32,
    ) {
        let stroke = Stroke::new(WAVE_STROKE_WIDTH, color);
        let visible = ui.clip_rect();
        for misspelling in &self.words {
            if cursor.is_some_and(|cursor| touches(&misspelling.char_range, cursor)) {
                continue;
            }
            let start = output
                .galley
                .pos_from_cursor(CCursor::new(misspelling.char_range.start));
            let end = output
                .galley
                .pos_from_cursor(CCursor::new(misspelling.char_range.end));
            // Only a word longer than the editor is wide wraps onto a second
            // row; such words are left without a line rather than drawing
            // one across the gap.
            let is_on_one_row = start.min.y == end.min.y;
            let rect = start.union(end).translate(output.galley_pos.to_vec2());
            if is_on_one_row && visible.intersects(rect) {
                ui.painter()
                    .add(wavy_line(rect.x_range(), rect.bottom() - WAVE_LIFT, stroke));
            }
        }
    }

    /// Opens the menu when a misspelled word is right-clicked and draws it
    /// while open. Returns the choice once the user picks one.
    pub fn show_menu(
        &mut self,
        output: &TextEditOutput,
        spell_check: Option<SpellCheck>,
        theme: &Theme,
    ) -> Option<MisspellingChoice> {
        let response = &output.response.response;
        if response.secondary_clicked() {
            self.menu = response.interact_pointer_pos().zip(spell_check).and_then(
                |(pointer, spell_check)| {
                    let cursor = output.galley.cursor_from_pos(pointer - output.galley_pos);
                    let misspelling = self.misspelling_at(cursor.index.0)?.clone();
                    Some(OpenMenu {
                        suggestions: spell_check.checker.suggestions(&misspelling.word),
                        misspelling,
                    })
                },
            );
        }
        // The text changed under an open menu: its word may be gone or moved.
        if self
            .menu
            .as_ref()
            .is_some_and(|menu| !self.words.contains(&menu.misspelling))
        {
            self.menu = None;
        }
        // Like egui's own context menu, but only opened over a misspelled word.
        let open_command = if response.secondary_clicked() {
            Some(SetOpenCommand::Bool(self.menu.is_some()))
        } else if response.clicked() {
            Some(SetOpenCommand::Bool(false))
        } else {
            None
        };
        let menu = self.menu.as_ref()?;
        Popup::menu(response)
            .open_memory(open_command)
            .at_pointer_fixed()
            .show(|ui| show_menu_contents(ui, menu, theme))
            .and_then(|shown| shown.inner)
    }

    /// The misspelled word that contains or ends at character offset `index`.
    fn misspelling_at(&self, index: usize) -> Option<&Misspelling> {
        self.words
            .iter()
            .find(|misspelling| touches(&misspelling.char_range, index))
    }
}

fn show_menu_contents(
    ui: &mut egui::Ui,
    menu: &OpenMenu,
    theme: &Theme,
) -> Option<MisspellingChoice> {
    let mut choice = None;
    for suggestion in &menu.suggestions {
        if ui.button(suggestion).clicked() {
            choice = Some(MisspellingChoice::Replace {
                char_range: menu.misspelling.char_range.clone(),
                replacement: suggestion.clone(),
            });
        }
    }
    if menu.suggestions.is_empty() {
        ui.label(
            RichText::new("No suggestions")
                .italics()
                .color(theme.text_faint),
        );
    }
    ui.separator();
    let word = &menu.misspelling.word;
    if ui.button(format!("Add \"{word}\" to dictionary")).clicked() {
        choice = Some(MisspellingChoice::AddToDictionary(word.clone()));
    }
    choice
}

/// The words of `text` that `checker` rejects, with character ranges.
fn find_misspellings(text: &str, checker: &SpellChecker) -> Vec<Misspelling> {
    let mut misspellings = Vec::new();
    // Counts characters incrementally: the word ranges come in source order.
    let mut char_index = 0;
    let mut byte_index = 0;
    for range in qm_format::spell_check_words(text) {
        let word = &text[range.clone()];
        if checker.is_correct(word) {
            continue;
        }
        char_index += text[byte_index..range.start].chars().count();
        let length = word.chars().count();
        misspellings.push(Misspelling {
            char_range: char_index..char_index + length,
            word: word.to_string(),
        });
        char_index += length;
        byte_index = range.end;
    }
    misspellings
}

/// Whether the cursor at `index` is inside `range` or right at either end of it.
fn touches(range: &Range<usize>, index: usize) -> bool {
    range.start <= index && index <= range.end
}

/// A zigzag from `x_range.min` to `x_range.max` around height `y`.
fn wavy_line(x_range: egui::Rangef, y: f32, stroke: Stroke) -> Shape {
    let mut points = Vec::new();
    let mut x = x_range.min;
    let mut is_up = true;
    while x < x_range.max {
        let offset = if is_up {
            -WAVE_AMPLITUDE
        } else {
            WAVE_AMPLITUDE
        };
        points.push(egui::pos2(x, y + offset));
        x += WAVE_HALF_LENGTH;
        is_up = !is_up;
    }
    points.push(egui::pos2(x_range.max, y));
    Shape::line(points, stroke)
}

fn state_hash(text: &str, revision: Option<u64>) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    revision.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use qm_core::{Language, WordList};

    fn english_checker() -> SpellChecker {
        let english = Language::available_in(&[])
            .into_iter()
            .find(|language| language.code() == "en_US")
            .expect("English is built in");
        let (checker, _) = SpellChecker::new(vec![english.load().unwrap()], &WordList::default());
        checker
    }

    #[test]
    fn misspellings_use_character_ranges() {
        let checker = english_checker();
        let found = find_misspellings(";;αβ;; theorm and lemmma", &checker);
        assert_eq!(
            found,
            [
                Misspelling {
                    char_range: 7..13,
                    word: "theorm".into()
                },
                Misspelling {
                    char_range: 18..24,
                    word: "lemmma".into()
                },
            ]
        );
    }

    #[test]
    fn update_rechecks_only_when_text_or_checker_changes() {
        let mut checker = english_checker();
        let mut misspellings = Misspellings::default();

        misspellings.update(
            "Qwertian space",
            Some(SpellCheck {
                checker: &checker,
                revision: 1,
            }),
        );
        assert_eq!(misspellings.words.len(), 1);

        checker.accept("Qwertian").unwrap();
        misspellings.update(
            "Qwertian space",
            Some(SpellCheck {
                checker: &checker,
                revision: 1,
            }),
        );
        assert_eq!(misspellings.words.len(), 1, "same revision: cached");
        misspellings.update(
            "Qwertian space",
            Some(SpellCheck {
                checker: &checker,
                revision: 2,
            }),
        );
        assert!(misspellings.words.is_empty());

        misspellings.update("Qwertian spaec", None);
        assert!(misspellings.words.is_empty(), "spell checking off");
    }

    #[test]
    fn a_word_counts_as_under_the_cursor_up_to_both_ends() {
        let misspellings = Misspellings {
            words: vec![Misspelling {
                char_range: 4..8,
                word: "spac".into(),
            }],
            ..Misspellings::default()
        };
        assert!(misspellings.misspelling_at(3).is_none());
        assert!(misspellings.misspelling_at(4).is_some());
        assert!(misspellings.misspelling_at(8).is_some());
        assert!(misspellings.misspelling_at(9).is_none());
    }

    #[test]
    fn wavy_line_spans_the_word() {
        let Shape::Path(path) = wavy_line(egui::Rangef::new(10.0, 20.0), 5.0, Stroke::NONE) else {
            panic!("expected a path");
        };
        assert_eq!(path.points.first().map(|p| p.x), Some(10.0));
        assert_eq!(path.points.last().map(|p| p.x), Some(20.0));
    }
}
