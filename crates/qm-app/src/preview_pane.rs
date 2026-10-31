//! The live preview pane and its synchronisation with the editor cursor.

use qm_render::{PreviewOptions, Theme};

use crate::note::OpenNote;

/// Remembers which cursor line the preview last scrolled to, so it only
/// scrolls when the cursor actually moves to another line.
#[derive(Debug, Default)]
pub struct PreviewSync {
    synced_line: Option<usize>,
    requested_line: Option<usize>,
}

impl PreviewSync {
    /// Scrolls the preview to `line` on its next frame, independent of the cursor
    /// (used by the outline when the editor is hidden).
    pub fn request_scroll(&mut self, line: usize) {
        self.requested_line = Some(line);
    }

    /// Returns the line to scroll the preview to this frame: an explicit
    /// request, or the cursor line when the cursor moved lines.
    fn line_to_scroll_to(&mut self, cursor_line: usize) -> Option<usize> {
        if let Some(line) = self.requested_line.take() {
            return Some(line);
        }
        if self.synced_line == Some(cursor_line) {
            return None;
        }
        self.synced_line = Some(cursor_line);
        Some(cursor_line)
    }
}

/// Draws the preview. Double-clicks jump the editor; returns the target of a
/// clicked wikilink for the workspace to open.
pub fn show_preview(
    ui: &mut egui::Ui,
    note: &mut OpenNote,
    sync: &mut PreviewSync,
    theme: &Theme,
) -> Option<String> {
    let options = PreviewOptions {
        scroll_to_line: sync.line_to_scroll_to(note.cursor.line),
    };
    let response = qm_render::show_document(ui, note.document(), theme, &options);
    if let Some(line) = response.clicked_line {
        note.requests.jump_to_line = Some(line);
        // The block is already in view; don't scroll the preview when the cursor lands there.
        sync.synced_line = Some(line);
    }
    response.clicked_wikilink
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrolls_once_per_cursor_line_change() {
        let mut sync = PreviewSync::default();
        assert_eq!(sync.line_to_scroll_to(0), Some(0));
        assert_eq!(sync.line_to_scroll_to(0), None);
        assert_eq!(sync.line_to_scroll_to(7), Some(7));
        assert_eq!(sync.line_to_scroll_to(7), None);
    }

    #[test]
    fn explicit_request_wins_once() {
        let mut sync = PreviewSync::default();
        sync.line_to_scroll_to(3);
        sync.request_scroll(12);
        assert_eq!(sync.line_to_scroll_to(3), Some(12));
        assert_eq!(sync.line_to_scroll_to(3), None);
    }
}
