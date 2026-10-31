//! The QuickMath (`.qm`) note format.
//!
//! See `docs/FORMAT.md` for the language specification.

pub mod ast;

mod block;
mod cheat_sheet;
mod front_matter;
mod highlight;
mod inline;
mod line_syntax;
mod lines;
mod math;

pub use ast::*;

/// File extension for QuickMath notes, without the dot.
pub const FILE_EXTENSION: &str = "qm";

/// Parses a whole note. Never fails: unparseable math becomes
/// [`MathNode::Error`] so the live preview can keep rendering while typing.
pub fn parse_document(source: &str) -> Document {
    let lines = lines::split_lines(source);
    let (front_matter, front_matter_lines) = front_matter::parse_front_matter(&lines);
    let blocks = block::parse_blocks(&lines[front_matter_lines..], 0);
    Document {
        front_matter,
        blocks,
    }
}

/// Parses a math expression as written between `;;` delimiters.
pub fn parse_math(source: &str) -> MathList {
    math::parse_math(source)
}

/// Converts math to LaTeX math-mode source (without surrounding `$`).
pub fn math_to_latex(math: &[MathNode]) -> String {
    math::math_to_latex(math)
}

/// Converts display math to LaTeX math-mode source (without `\[ \]`).
///
/// A single unaligned row is plain math; several rows, or a row with
/// alignment cells, become `\begin{aligned} … \end{aligned}` with `&` at
/// the cell boundaries and `\\` between rows.
pub fn display_math_to_latex(display: &DisplayMath) -> String {
    math::display_math_to_latex(display)
}

/// How the help panel should show a [`CheatSheetEntry`] input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheatSheetKind {
    /// Document-level syntax (headings, callouts, `;;` lines): show as text.
    Markup,
    /// Exactly what goes between `;;` delimiters: render `parse_math(input)`.
    Math,
}

/// One example for the editor's help panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheatSheetEntry {
    pub input: &'static str,
    pub description: &'static str,
    pub kind: CheatSheetKind,
}

/// Example inputs with short descriptions for the editor's help panel.
pub fn cheat_sheet() -> &'static [CheatSheetEntry] {
    cheat_sheet::CHEAT_SHEET
}

/// Syntax-highlighting category for a byte range of the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Text,
    Heading,
    Emphasis,
    Strong,
    Code,
    /// The `;;` delimiters and other structural markup.
    Markup,
    MathIdentifier,
    MathNumber,
    MathOperator,
    /// Keywords such as `sum`, `sqrt`, `alpha`, `RR`.
    MathKeyword,
    MathText,
    CalloutMarker,
    WikiLink,
    FrontMatter,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    pub range: std::ops::Range<usize>,
    pub kind: TokenKind,
}

/// Splits `source` into contiguous, non-overlapping spans that cover every
/// byte, for the editor's syntax highlighting.
pub fn highlight(source: &str) -> Vec<HighlightSpan> {
    highlight::highlight(source)
}
