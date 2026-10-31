//! Live rendering of QuickMath documents and math inside egui.
//!
//! Layout is a TeX-inspired box model ([`math`]) plus a paragraph flow with
//! its own line breaking ([`document`]); both produce theme-independent
//! drawing lists ([`paint`]) that are cached between frames and painted with
//! egui's painter.

mod document;
pub mod fonts;
mod math;
mod measure;
mod paint;
pub mod theme;

pub use theme::{SyntaxColors, Theme};

use qm_format::{Document, MathNode};

/// Registers the math and text fonts the renderer needs. Call once at startup;
/// the app may add its own fonts to the same definitions afterwards.
///
/// Adds the named families in [`fonts`] (`qm-math`, `qm-serif`, …); the
/// app's `Proportional` and `Monospace` families are left unchanged.
pub fn install_fonts(fonts: &mut egui::FontDefinitions) {
    fonts::install_fonts(fonts);
}

/// Options for one preview frame.
#[derive(Debug, Clone, Default)]
pub struct PreviewOptions {
    /// When set, scroll so the block starting at or before this source line is visible.
    ///
    /// The preview scrolls every frame this is set, so only set it when the
    /// editor cursor actually moved (otherwise the user cannot scroll freely).
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
pub fn show_document(
    ui: &mut egui::Ui,
    document: &Document,
    theme: &Theme,
    options: &PreviewOptions,
) -> PreviewResponse {
    document::show_document(ui, document, theme, options)
}

/// Draws a single math expression (e.g. for a symbol cheat sheet or tooltip).
///
/// The math is sized relative to the ui's body text; hovering an error shows
/// its message.
pub fn show_math(
    ui: &mut egui::Ui,
    math: &[MathNode],
    is_display: bool,
    theme: &Theme,
) -> egui::Response {
    document::show_math(ui, math, is_display, theme)
}
