//! Syntax trees produced by the QuickMath parser.
//!
//! These types are the contract between the parser, the live renderer and the
//! exporters. Change them only in a backwards-compatible way, and update every
//! consumer when you do.

/// A parsed `.qm` note.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Document {
    pub front_matter: FrontMatter,
    pub blocks: Vec<Block>,
}

/// Metadata from the optional `---` header at the top of a note.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FrontMatter {
    pub title: Option<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub course: Option<String>,
    pub tags: Vec<String>,
}

/// A block-level element together with where it starts in the source.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub kind: BlockKind,
    /// Zero-based source line the block starts on; used to sync the editor
    /// cursor with the preview.
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockKind {
    /// `level` is 1..=6.
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    Paragraph(Vec<Inline>),
    DisplayMath(DisplayMath),
    /// Each item is a list of blocks so items can nest lists and math.
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
    },
    /// Lecture-style boxes such as definitions, theorems and proofs.
    Callout {
        kind: CalloutKind,
        title: Vec<Inline>,
        body: Vec<Block>,
    },
    Code {
        language: Option<String>,
        code: String,
    },
    Quote(Vec<Block>),
    Rule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CalloutKind {
    Definition,
    Theorem,
    Lemma,
    Corollary,
    Proof,
    Example,
    Note,
    Warning,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Text(String),
    Strong(Vec<Inline>),
    Emphasis(Vec<Inline>),
    Code(String),
    Math(MathList),
    /// `[[target]]` or `[[target|label]]`, linking to another note in the project.
    WikiLink {
        target: String,
        label: Option<String>,
    },
    LineBreak,
}

/// Display math; rows stack vertically and their cells align column-wise.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DisplayMath {
    pub rows: Vec<MathRow>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MathRow {
    /// Cells separated by the alignment marker; a single cell means no alignment.
    pub cells: Vec<MathList>,
}

pub type MathList = Vec<MathNode>;

#[derive(Debug, Clone, PartialEq)]
pub enum MathNode {
    Atom(Atom),
    /// Upright prose inside math, e.g. `"otherwise"`.
    Text(String),
    Fraction {
        numerator: MathList,
        denominator: MathList,
    },
    /// A base with an optional subscript and/or superscript. When the base is a
    /// large operator with `limits: true`, display mode places the scripts
    /// above and below instead of to the right.
    Scripts {
        base: Box<MathNode>,
        sub: Option<MathList>,
        sup: Option<MathList>,
    },
    Root {
        index: Option<MathList>,
        radicand: MathList,
    },
    /// Delimiters that stretch to the body height. An empty string means "no
    /// delimiter on this side".
    Fenced {
        open: String,
        close: String,
        body: MathList,
    },
    /// Rows of cells; `open`/`close` follow the same rules as `Fenced`.
    /// Piecewise definitions use `open: "{"` and `close: ""`.
    Matrix {
        open: String,
        close: String,
        rows: Vec<Vec<MathList>>,
    },
    Accent {
        accent: AccentKind,
        body: MathList,
    },
    /// Invisible grouping, e.g. the parentheses dropped from `(a+b)/c`.
    Group(MathList),
    Space(SpaceWidth),
    /// Source that could not be parsed; shown verbatim and highlighted.
    Error {
        source: String,
        message: String,
    },
}

/// A single symbol: a variable, number, operator, relation, Greek letter, …
#[derive(Debug, Clone, PartialEq)]
pub struct Atom {
    /// What to display, as Unicode (e.g. `"α"`, `"≤"`, `"ℝ"`, `"sin"`, `"42"`).
    pub text: String,
    /// The equivalent LaTeX (e.g. `"\\alpha"`, `"\\leq"`, `"\\mathbb{R}"`).
    pub latex: String,
    pub class: AtomClass,
    pub style: AtomStyle,
}

/// Spacing class, following TeX's atom types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomClass {
    Ordinary,
    /// `sum`, `int`, `lim`, … `limits` says whether scripts go above/below in display mode.
    LargeOperator {
        limits: bool,
    },
    Binary,
    Relation,
    Open,
    Close,
    Punctuation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomStyle {
    /// Single-letter variables.
    Italic,
    /// Numbers, operators, function names (`sin`), already-styled Unicode (`ℝ`).
    Upright,
    Bold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccentKind {
    Hat,
    Bar,
    Vector,
    Dot,
    DoubleDot,
    Tilde,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpaceWidth {
    Thin,
    Medium,
    Thick,
    Quad,
}
