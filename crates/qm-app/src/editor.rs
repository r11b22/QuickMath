//! The source editor: a highlighted, monospace `TextEdit` with a line-number gutter.

use egui::text::{CCursor, CCursorRange};
use egui::widgets::text_edit::{TextEditOutput, TextEditState};
use egui::{Align, Align2, FontId, Id, Key, Modifiers};
use qm_render::Theme;

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
        let mut is_edited = handle_indent_keys(ui.ctx(), id, &mut note.text);
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

        let output = egui::ScrollArea::vertical()
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
            })
            .inner;

        if is_edited || output.response.response.changed() {
            note.mark_edited(now);
        }
        if let Some(range) = output.cursor_range {
            note.cursor = text_metrics::position_of_char(&note.text, range.primary.index.0);
        }
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
    let mut state = TextEditState::load(ctx, id).unwrap_or_default();
    let range = match state.cursor.char_range() {
        Some(range) => {
            let sorted = range.as_sorted_char_range();
            sorted.start.0..sorted.end.0
        }
        None => {
            let end = text.chars().count();
            end..end
        }
    };
    let cursor = text_metrics::replace_char_range(text, range, insertion);
    state
        .cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(cursor))));
    state.store(ctx, id);
    true
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
