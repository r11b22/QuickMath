//! Live rendering of QuickMath documents and math inside egui.

pub mod theme;

pub use theme::{SyntaxColors, Theme};

use qm_format::{Document, MathNode};

/// Registers the math and text fonts the renderer needs. Call once at startup;
/// the app may add its own fonts to the same definitions afterwards.
pub fn install_fonts(fonts: &mut egui::FontDefinitions) {
    // CONTRACT STUB: replaced by the renderer agent.
    let _ = fonts;
}

/// Options for one preview frame.
#[derive(Debug, Clone, Default)]
pub struct PreviewOptions {
    /// When set, scroll so the block starting at or before this source line is visible.
    pub scroll_to_line: Option<usize>,
}

/// What the user did in the preview this frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PreviewResponse {
    /// Source line of a block the user double-clicked (jump the editor there).
    pub clicked_line: Option<usize>,
    /// Target of a wikilink the user clicked.
    pub clicked_wikilink: Option<String>,
}

/// Draws the whole document as a scrollable, page-like preview filling `ui`.
pub fn show_document(ui: &mut egui::Ui, document: &Document, theme: &Theme, options: &PreviewOptions) -> PreviewResponse {
    // CONTRACT STUB: replaced by the renderer agent.
    let _ = (document, options, theme);
    ui.label("preview");
    PreviewResponse::default()
}

/// Draws a single math expression (e.g. for a symbol cheat sheet or tooltip).
pub fn show_math(ui: &mut egui::Ui, math: &[MathNode], is_display: bool, theme: &Theme) -> egui::Response {
    // CONTRACT STUB: replaced by the renderer agent.
    let _ = (math, is_display, theme);
    ui.label("math")
}
