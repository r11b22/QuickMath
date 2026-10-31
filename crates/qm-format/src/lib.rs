//! The QuickMath (`.qm`) note format.
//!
//! See `docs/FORMAT.md` for the language specification.

pub mod ast;

pub use ast::*;

/// File extension for QuickMath notes, without the dot.
pub const FILE_EXTENSION: &str = "qm";

/// Parses a whole note. Never fails: unparseable math becomes
/// [`MathNode::Error`] so the live preview can keep rendering while typing.
pub fn parse_document(source: &str) -> Document {
    // CONTRACT STUB: replaced by the format agent.
    Document {
        front_matter: FrontMatter::default(),
        blocks: vec![Block { kind: BlockKind::Paragraph(vec![Inline::Text(source.to_string())]), line: 0 }],
    }
}

/// Parses a math expression as written between `$` delimiters.
pub fn parse_math(source: &str) -> MathList {
    // CONTRACT STUB: replaced by the format agent.
    vec![MathNode::Text(source.to_string())]
}

/// Converts math to LaTeX math-mode source (without surrounding `$`).
pub fn math_to_latex(math: &[MathNode]) -> String {
    // CONTRACT STUB: replaced by the format agent.
    let _ = math;
    String::new()
}

/// Syntax-highlighting category for a byte range of the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Text,
    Heading,
    Emphasis,
    Strong,
    Code,
    /// The `$` / `$$` delimiters and other structural markup.
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
    // CONTRACT STUB: replaced by the format agent.
    vec![HighlightSpan { range: 0..source.len(), kind: TokenKind::Text }]
}
