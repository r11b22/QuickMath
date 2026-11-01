//! The source editor: a highlighted, monospace `TextEdit` with a line-number
//! gutter and suggestions for math words.

use egui::text::{CCursor, CCursorRange};
use egui::widgets::text_edit::{TextEditOutput, TextEditState};
use egui::{Align, Align2, EventFilter, FontId, Id, Key, Modifiers};
use qm_render::Theme;

use crate::completion_popup::CompletionPopup;
use crate::highlight::HighlightCache;
use crate::note::OpenNote;
use crate::text_metrics;

const EDITOR_FONT_SIZE: f32 = 14.0;
const INDENT: &str = "  ";
const GUTTER_PADDING: f32 = 14.0;
const TEXT_MARGIN_LEFT: f32 = 12.0;
const TEXT_MARGIN_RIGHT: i8 = 16;
const TEXT_MARGIN_VERTICAL: i8 = 12;
const MIN_GUTTER_DIGITS: usize = 2;

/// Editor widget state that outlives a single frame.
#[derive(Default)]
pub struct EditorView {
    highlight: HighlightCache,
    completion: CompletionPopup,
}

/// The editor's widget id. Salted with the note path so every note gets its
/// own undo history and a switch never "undoes" into another note's text.
fn editor_id(note: &OpenNote) -> Id {
    Id::new("qm_editor").with(note.path())
}

impl EditorView {
    /// Draws the editor filling `ui` and applies any pending [`crate::note::EditorRequests`].
    pub fn show(&mut self, ui: &mut egui::Ui, note: &mut OpenNote, theme: &Theme) {
        let id = editor_id(note);
        let now = ui.input(|input| input.time);
        let font_id = FontId::monospace(EDITOR_FONT_SIZE);
        let mut is_edited = self.accept_completion(ui.ctx(), id, &mut note.text);
        // Runs after completion, which takes Tab while suggestions are shown.
        is_edited |= handle_indent_keys(ui.ctx(), id, &mut note.text);
        if let Some(insertion) = note.requests.insertion.take() {
            insert_at_cursor(ui.ctx(), id, &mut note.text, &insertion);
            ui.memory_mut(|memory| memory.request_focus(id));
            is_edited = true;
        }
        let jump_to_line = note.requests.jump_to_line.take();
        let gutter = Gutter {
            width: gutter_width(ui, &font_id, &note.text),
            current_line: note.cursor.line,
        };

        let scroll = egui::ScrollArea::vertical()
            .id_salt(id.with("scroll"))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let output =
                    self.show_text_edit(ui, id, &mut note.text, theme, &font_id, gutter.width);
                gutter.paint(ui, &output, theme, &font_id);
                if let Some(line) = jump_to_line {
                    move_cursor_to_line(ui, id, &note.text, &output, line);
                }
                output
            });
        let output = scroll.inner;

        if is_edited || output.response.response.changed() {
            note.mark_edited(now);
        }
        if let Some(range) = output.cursor_range {
            note.cursor = text_metrics::position_of_char(&note.text, range.primary.index.0);
        }
        if ui.memory(|memory| memory.has_focus(id)) {
            let cursor = output.cursor_range.and_then(cursor_without_selection);
            self.completion.update(&note.text, cursor);
            let filter = editor_event_filter(self.completion.is_visible());
            ui.memory_mut(|memory| memory.set_focus_lock_filter(id, filter));
            let popup_id = id.with("completion");
            self.completion
                .show(ui.ctx(), popup_id, &output, scroll.inner_rect, theme);
        }
    }

    /// Applies a suggestion accepted with Tab. Returns whether the text changed.
    fn accept_completion(&mut self, ctx: &egui::Context, id: Id, text: &mut String) -> bool {
        if !ctx.memory(|memory| memory.has_focus(id)) {
            return false;
        }
        // Refresh first: the text may have changed since the last frame
        // (another note opened, an insertion from the help panel).
        let cursor = TextEditState::load(ctx, id)
            .and_then(|state| state.cursor.char_range())
            .and_then(cursor_without_selection);
        self.completion.update(text, cursor);
        let Some(acceptance) = self.completion.handle_keys(ctx) else {
            return false;
        };
        replace_and_place_cursor(ctx, id, text, acceptance.char_range, acceptance.spelling);
        true
    }

    fn show_text_edit(
        &mut self,
        ui: &mut egui::Ui,
        id: Id,
        text: &mut String,
        theme: &Theme,
        font_id: &FontId,
        gutter_width: f32,
    ) -> TextEditOutput {
        let highlight = &mut self.highlight;
        let mut layouter = |ui: &egui::Ui, buffer: &dyn egui::TextBuffer, wrap_width: f32| {
            let mut job = highlight.layout_job(buffer.as_str(), theme, font_id);
            job.wrap.max_width = wrap_width;
            ui.fonts_mut(|fonts| fonts.layout_job(job))
        };
        let margin = egui::Margin {
            left: (gutter_width + TEXT_MARGIN_LEFT).round() as i8,
            right: TEXT_MARGIN_RIGHT,
            top: TEXT_MARGIN_VERTICAL,
            bottom: TEXT_MARGIN_VERTICAL,
        };
        egui::TextEdit::multiline(text)
            .id(id)
            .font(font_id.clone())
            .lock_focus(true)
            .desired_width(f32::INFINITY)
            .min_size(ui.available_size())
            // A custom frame replaces `.margin()` for placement, so the
            // margin (which keeps text clear of the gutter) must live on the frame.
            .frame(egui::Frame::NONE.inner_margin(margin))
            .margin(margin)
            .layouter(&mut layouter)
            .show(ui)
    }
}

/// Tab inserts two spaces, Shift+Tab removes up to two. Returns whether the text changed.
fn handle_indent_keys(ctx: &egui::Context, id: Id, text: &mut String) -> bool {
    if !ctx.memory(|memory| memory.has_focus(id)) {
        return false;
    }
    if ctx.input_mut(|input| input.consume_key(Modifiers::SHIFT, Key::Tab)) {
        return outdent_at_cursor(ctx, id, text);
    }
    if ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Tab)) {
        return insert_at_cursor(ctx, id, text, INDENT);
    }
    false
}

/// Replaces the selection (or inserts at the cursor, or at the end when the
/// editor never had a cursor) and places the cursor after the insertion.
fn insert_at_cursor(ctx: &egui::Context, id: Id, text: &mut String, insertion: &str) -> bool {
    let selection = TextEditState::load(ctx, id).and_then(|state| state.cursor.char_range());
    let range = match selection {
        Some(range) => {
            let sorted = range.as_sorted_char_range();
            sorted.start.0..sorted.end.0
        }
        None => {
            let end = text.chars().count();
            end..end
        }
    };
    replace_and_place_cursor(ctx, id, text, range, insertion);
    true
}

/// Replaces `char_range` with `insertion` and puts the cursor after it.
fn replace_and_place_cursor(
    ctx: &egui::Context,
    id: Id,
    text: &mut String,
    char_range: std::ops::Range<usize>,
    insertion: &str,
) {
    let mut state = TextEditState::load(ctx, id).unwrap_or_default();
    let cursor = text_metrics::replace_char_range(text, char_range, insertion);
    state
        .cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(cursor))));
    state.store(ctx, id);
}

/// The keys the focused editor keeps for itself instead of egui's focus
/// navigation. Matches what `TextEdit` with `lock_focus(true)` sets, plus
/// Escape while suggestions are shown, so Escape hides them rather than
/// leaving the editor. Applies from the next frame on.
fn editor_event_filter(is_completion_visible: bool) -> EventFilter {
    EventFilter {
        tab: true,
        horizontal_arrows: true,
        vertical_arrows: true,
        escape: is_completion_visible,
    }
}

/// The cursor's character offset, or `None` while text is selected.
fn cursor_without_selection(range: CCursorRange) -> Option<usize> {
    (range.primary == range.secondary).then_some(range.primary.index.0)
}

fn outdent_at_cursor(ctx: &egui::Context, id: Id, text: &mut String) -> bool {
    let Some(mut state) = TextEditState::load(ctx, id) else {
        return false;
    };
    let Some(range) = state.cursor.char_range() else {
        return false;
    };
    let before = text.len();
    let cursor = text_metrics::outdent_line(text, range.primary.index.0, INDENT.len());
    state
        .cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(cursor))));
    state.store(ctx, id);
    text.len() != before
}

fn move_cursor_to_line(ui: &egui::Ui, id: Id, text: &str, output: &TextEditOutput, line: usize) {
    let cursor = CCursor::new(text_metrics::char_offset_of_line(text, line));
    let mut state = output.state.clone();
    state.cursor.set_char_range(Some(CCursorRange::one(cursor)));
    state.store(ui.ctx(), id);
    ui.memory_mut(|memory| memory.request_focus(id));
    let cursor_rect = output
        .galley
        .pos_from_cursor(cursor)
        .translate(output.galley_pos.to_vec2());
    ui.scroll_to_rect(cursor_rect, Some(Align::Center));
    ui.ctx().request_repaint();
}

fn gutter_width(ui: &egui::Ui, font_id: &FontId, text: &str) -> f32 {
    let line_count = text.matches('\n').count() + 1;
    let digits = line_count.to_string().len().max(MIN_GUTTER_DIGITS);
    let digit_width = ui.fonts_mut(|fonts| fonts.glyph_width(font_id, '0'));
    digits as f32 * digit_width + 2.0 * GUTTER_PADDING
}

/// The line-number column painted into the editor's left margin, so it
/// scrolls together with the text.
struct Gutter {
    width: f32,
    current_line: usize,
}

impl Gutter {
    fn paint(&self, ui: &egui::Ui, output: &TextEditOutput, theme: &Theme, font_id: &FontId) {
        let editor_rect = output.response.response.rect;
        let gutter_rect = egui::Rect::from_min_size(
            editor_rect.min,
            egui::vec2(self.width, editor_rect.height()),
        );
        let painter = ui.painter();
        painter.rect_filled(gutter_rect, 0.0, theme.surface);
        painter.vline(
            gutter_rect.right(),
            gutter_rect.y_range(),
            egui::Stroke::new(1.0, theme.border),
        );

        let visible = ui.clip_rect();
        let number_x = gutter_rect.right() - GUTTER_PADDING;
        // Wrapped rows continue the previous line, so only rows that start a
        // source line get a number.
        let mut line = 0;
        let mut starts_line = true;
        for row in &output.galley.rows {
            let row_top = output.galley_pos.y + row.pos.y;
            let is_visible =
                row_top + row.rect().height() >= visible.top() && row_top <= visible.bottom();
            if starts_line && is_visible {
                let color = if line == self.current_line {
                    theme.text_muted
                } else {
                    theme.text_faint
                };
                let label = (line + 1).to_string();
                painter.text(
                    egui::pos2(number_x, row_top),
                    Align2::RIGHT_TOP,
                    label,
                    font_id.clone(),
                    color,
                );
            }
            if row.ends_with_newline {
                line += 1;
            }
            starts_line = row.ends_with_newline;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, RawInput};
    use qm_core::Project;

    /// Drives the real editor widget headlessly, one frame per call.
    struct Harness {
        ctx: egui::Context,
        editor: EditorView,
        note: OpenNote,
        folder: std::path::PathBuf,
    }

    impl Harness {
        fn new(name: &str) -> Harness {
            let folder =
                std::env::temp_dir().join(format!("qm-editor-test-{}-{name}", std::process::id()));
            std::fs::create_dir_all(&folder).unwrap();
            let project = Project::open(&folder).unwrap();
            let path = project.create_note(&folder, "note").unwrap();
            let note = OpenNote::load(&project, &path).unwrap();
            let ctx = egui::Context::default();
            let mut fonts = egui::FontDefinitions::default();
            qm_render::install_fonts(&mut fonts);
            ctx.set_fonts(fonts);
            ctx.memory_mut(|memory| memory.request_focus(editor_id(&note)));
            let mut harness = Harness {
                ctx,
                editor: EditorView::default(),
                note,
                folder,
            };
            harness.frame(Vec::new());
            harness
        }

        fn frame(&mut self, events: Vec<Event>) {
            let input = RawInput {
                events,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                ..RawInput::default()
            };
            let theme = Theme::light();
            let Harness {
                ctx, editor, note, ..
            } = self;
            let _ = ctx.run_ui(input, |ui| editor.show(ui, note, &theme));
        }

        fn type_text(&mut self, text: &str) {
            self.frame(vec![Event::Text(text.to_string())]);
        }

        fn press(&mut self, key: Key) {
            self.frame(vec![Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }]);
        }
    }

    impl Drop for Harness {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.folder);
        }
    }

    #[test]
    fn tab_completes_a_math_word() {
        let mut harness = Harness::new("complete");
        harness.type_text(";;alp");
        harness.press(Key::Tab);
        assert_eq!(harness.note.text, ";;alpha");
        harness.type_text(" x");
        assert_eq!(harness.note.text, ";;alpha x");
    }

    #[test]
    fn arrow_down_picks_the_next_suggestion() {
        let mut harness = Harness::new("arrow");
        harness.type_text(";;sq");
        harness.press(Key::ArrowDown);
        harness.press(Key::Tab);
        assert_eq!(harness.note.text, ";;sqrt");
    }

    #[test]
    fn tab_indents_when_nothing_is_suggested() {
        let mut harness = Harness::new("indent");
        harness.type_text("alp");
        harness.press(Key::Tab);
        assert_eq!(harness.note.text, "alp  ");
    }

    #[test]
    fn escape_hides_suggestions_so_tab_indents() {
        let mut harness = Harness::new("escape");
        harness.type_text(";;alp");
        harness.press(Key::Escape);
        harness.press(Key::Tab);
        assert_eq!(harness.note.text, ";;alp  ");
    }

    #[test]
    fn escape_without_suggestions_still_leaves_the_editor() {
        let mut harness = Harness::new("leave");
        harness.type_text("alp");
        harness.press(Key::Escape);
        harness.press(Key::Tab);
        assert_eq!(harness.note.text, "alp");
    }
}
