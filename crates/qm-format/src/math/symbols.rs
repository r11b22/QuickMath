//! The math vocabulary: keywords (`alpha`, `sum`, `sqrt`, …) and operator
//! spellings (`<=`, `->`, …) together with their Unicode and LaTeX forms.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::OnceLock;

use crate::ast::{AccentKind, Atom, AtomClass, AtomStyle, SpaceWidth};
use crate::math::latex::escape_text;

/// A fixed symbol from the tables below.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Symbol {
    pub text: &'static str,
    pub latex: &'static str,
    pub class: AtomClass,
    pub style: AtomStyle,
}

impl Symbol {
    pub fn to_atom(self) -> Atom {
        Atom {
            text: self.text.to_string(),
            latex: self.latex.to_string(),
            class: self.class,
            style: self.style,
        }
    }
}

/// Alphabet transformations applied by `cal(…)`, `bb(…)` and `bbb(…)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Font {
    Calligraphic,
    Bold,
    Blackboard,
}

/// What a bare word (or `\word` command) means inside math.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Keyword {
    Symbol(Symbol),
    Sqrt,
    Root,
    Fence {
        open: &'static str,
        close: &'static str,
    },
    Accent(AccentKind),
    Font(Font),
    Matrix {
        open: &'static str,
        close: &'static str,
    },
    Space {
        width: SpaceWidth,
        count: usize,
    },
    /// `a + b over c + d`: a fraction of everything around it, bounded by
    /// relations, punctuation and the enclosing group.
    Over,
    /// `x sq`, `x cb`, `A inv`: a superscript attached to the previous term.
    Postfix(Symbol),
    /// Prose inside math such as `if` or `otherwise`, shown as upright text.
    Prose,
}

const fn symbol(
    text: &'static str,
    latex: &'static str,
    class: AtomClass,
    style: AtomStyle,
) -> Symbol {
    Symbol {
        text,
        latex,
        class,
        style,
    }
}

const fn ordinary(text: &'static str, latex: &'static str) -> Symbol {
    symbol(text, latex, AtomClass::Ordinary, AtomStyle::Upright)
}

const fn greek(text: &'static str, latex: &'static str) -> Symbol {
    symbol(text, latex, AtomClass::Ordinary, AtomStyle::Italic)
}

const fn relation(text: &'static str, latex: &'static str) -> Symbol {
    symbol(text, latex, AtomClass::Relation, AtomStyle::Upright)
}

const fn binary(text: &'static str, latex: &'static str) -> Symbol {
    symbol(text, latex, AtomClass::Binary, AtomStyle::Upright)
}

const fn punctuation(text: &'static str, latex: &'static str) -> Symbol {
    symbol(text, latex, AtomClass::Punctuation, AtomStyle::Upright)
}

const fn large(text: &'static str, latex: &'static str, limits: bool) -> Symbol {
    symbol(
        text,
        latex,
        AtomClass::LargeOperator { limits },
        AtomStyle::Upright,
    )
}

/// Function names (`sin`, `det`) are operator atoms like TeX's `\mathop`, so
/// renderers space them from their argument; their scripts never go above.
const fn function(text: &'static str, latex: &'static str) -> Symbol {
    large(text, latex, false)
}

const fn sym(symbol: Symbol) -> Keyword {
    Keyword::Symbol(symbol)
}

pub(crate) const INFINITY: Symbol = ordinary("∞", "\\infty");
pub(crate) const INFIMUM: Symbol = large("inf", "\\inf", true);
pub(crate) const PRIME: Symbol = ordinary("′", "\\prime");
/// `*` used as a script (`z^*`) means the conjugate star, not a centred dot.
pub(crate) const ASTERISK: Symbol = ordinary("∗", "*");
const TWO: Symbol = ordinary("2", "2");
const THREE: Symbol = ordinary("3", "3");
const MINUS_ONE: Symbol = ordinary("−1", "-1");

/// Word → meaning. Earlier entries win when several words share a Unicode
/// text, which decides the LaTeX used for directly typed Unicode symbols.
static KEYWORDS: &[(&str, Keyword)] = &[
    // Greek, lowercase (italic like TeX).
    ("alpha", sym(greek("α", "\\alpha"))),
    ("beta", sym(greek("β", "\\beta"))),
    ("gamma", sym(greek("γ", "\\gamma"))),
    ("delta", sym(greek("δ", "\\delta"))),
    ("eps", sym(greek("ε", "\\varepsilon"))),
    ("epsilon", sym(greek("ε", "\\varepsilon"))),
    ("varepsilon", sym(greek("ε", "\\varepsilon"))),
    ("zeta", sym(greek("ζ", "\\zeta"))),
    ("eta", sym(greek("η", "\\eta"))),
    ("theta", sym(greek("θ", "\\theta"))),
    ("vartheta", sym(greek("ϑ", "\\vartheta"))),
    ("iota", sym(greek("ι", "\\iota"))),
    ("kappa", sym(greek("κ", "\\kappa"))),
    ("lambda", sym(greek("λ", "\\lambda"))),
    ("lamda", sym(greek("λ", "\\lambda"))),
    ("mu", sym(greek("μ", "\\mu"))),
    ("nu", sym(greek("ν", "\\nu"))),
    ("xi", sym(greek("ξ", "\\xi"))),
    ("pi", sym(greek("π", "\\pi"))),
    ("varpi", sym(greek("ϖ", "\\varpi"))),
    ("rho", sym(greek("ρ", "\\rho"))),
    ("varrho", sym(greek("ϱ", "\\varrho"))),
    ("sigma", sym(greek("σ", "\\sigma"))),
    ("varsigma", sym(greek("ς", "\\varsigma"))),
    ("tau", sym(greek("τ", "\\tau"))),
    ("upsilon", sym(greek("υ", "\\upsilon"))),
    ("phi", sym(greek("φ", "\\varphi"))),
    ("varphi", sym(greek("φ", "\\varphi"))),
    ("chi", sym(greek("χ", "\\chi"))),
    ("psi", sym(greek("ψ", "\\psi"))),
    ("omega", sym(greek("ω", "\\omega"))),
    // Greek, uppercase (upright like TeX).
    ("Gamma", sym(ordinary("Γ", "\\Gamma"))),
    ("Delta", sym(ordinary("Δ", "\\Delta"))),
    ("Theta", sym(ordinary("Θ", "\\Theta"))),
    ("Lambda", sym(ordinary("Λ", "\\Lambda"))),
    ("Xi", sym(ordinary("Ξ", "\\Xi"))),
    ("Pi", sym(ordinary("Π", "\\Pi"))),
    ("Sigma", sym(ordinary("Σ", "\\Sigma"))),
    ("Upsilon", sym(ordinary("Υ", "\\Upsilon"))),
    ("Phi", sym(ordinary("Φ", "\\Phi"))),
    ("Psi", sym(ordinary("Ψ", "\\Psi"))),
    ("Omega", sym(ordinary("Ω", "\\Omega"))),
    // Letter-like symbols. `inf` is special-cased by the parser (∞ vs infimum).
    ("oo", sym(INFINITY)),
    ("inf", sym(INFINITY)),
    ("infty", sym(INFINITY)),
    ("del", sym(ordinary("∂", "\\partial"))),
    ("partial", sym(ordinary("∂", "\\partial"))),
    ("grad", sym(ordinary("∇", "\\nabla"))),
    ("nabla", sym(ordinary("∇", "\\nabla"))),
    ("RR", sym(ordinary("ℝ", "\\mathbb{R}"))),
    ("NN", sym(ordinary("ℕ", "\\mathbb{N}"))),
    ("ZZ", sym(ordinary("ℤ", "\\mathbb{Z}"))),
    ("QQ", sym(ordinary("ℚ", "\\mathbb{Q}"))),
    ("CC", sym(ordinary("ℂ", "\\mathbb{C}"))),
    ("forall", sym(ordinary("∀", "\\forall"))),
    ("exists", sym(ordinary("∃", "\\exists"))),
    ("nexists", sym(ordinary("∄", "\\nexists"))),
    ("not", sym(ordinary("¬", "\\neg"))),
    ("neg", sym(ordinary("¬", "\\neg"))),
    ("emptyset", sym(ordinary("∅", "\\emptyset"))),
    ("empty", sym(ordinary("∅", "\\emptyset"))),
    ("aleph", sym(ordinary("ℵ", "\\aleph"))),
    ("hbar", sym(ordinary("ℏ", "\\hbar"))),
    ("ell", sym(ordinary("ℓ", "\\ell"))),
    ("angle", sym(ordinary("∠", "\\angle"))),
    ("triangle", sym(ordinary("△", "\\triangle"))),
    ("qed", sym(ordinary("∎", "\\blacksquare"))),
    ("ldots", sym(ordinary("…", "\\ldots"))),
    ("dots", sym(ordinary("…", "\\ldots"))),
    ("cdots", sym(ordinary("⋯", "\\cdots"))),
    ("vdots", sym(ordinary("⋮", "\\vdots"))),
    ("ddots", sym(ordinary("⋱", "\\ddots"))),
    // Relations and arrows.
    ("in", sym(relation("∈", "\\in"))),
    ("notin", sym(relation("∉", "\\notin"))),
    ("sub", sym(relation("⊂", "\\subset"))),
    ("subset", sym(relation("⊂", "\\subset"))),
    ("sube", sym(relation("⊆", "\\subseteq"))),
    ("subseteq", sym(relation("⊆", "\\subseteq"))),
    ("supset", sym(relation("⊃", "\\supset"))),
    ("supe", sym(relation("⊇", "\\supseteq"))),
    ("supseteq", sym(relation("⊇", "\\supseteq"))),
    ("leq", sym(relation("≤", "\\leq"))),
    ("le", sym(relation("≤", "\\leq"))),
    ("geq", sym(relation("≥", "\\geq"))),
    ("ge", sym(relation("≥", "\\geq"))),
    ("neq", sym(relation("≠", "\\neq"))),
    ("ne", sym(relation("≠", "\\neq"))),
    ("approx", sym(relation("≈", "\\approx"))),
    ("equiv", sym(relation("≡", "\\equiv"))),
    ("sim", sym(relation("∼", "\\sim"))),
    ("simeq", sym(relation("≃", "\\simeq"))),
    ("cong", sym(relation("≅", "\\cong"))),
    ("prop", sym(relation("∝", "\\propto"))),
    ("propto", sym(relation("∝", "\\propto"))),
    ("perp", sym(relation("⊥", "\\perp"))),
    ("parallel", sym(relation("∥", "\\parallel"))),
    ("to", sym(relation("→", "\\to"))),
    ("mapsto", sym(relation("↦", "\\mapsto"))),
    ("implies", sym(relation("⇒", "\\implies"))),
    ("iff", sym(relation("⇔", "\\iff"))),
    ("ll", sym(relation("≪", "\\ll"))),
    ("gg", sym(relation("≫", "\\gg"))),
    ("mid", sym(relation("∣", "\\mid"))),
    ("therefore", sym(relation("∴", "\\therefore"))),
    ("because", sym(relation("∵", "\\because"))),
    // Binary operators.
    ("xx", sym(binary("×", "\\times"))),
    ("times", sym(binary("×", "\\times"))),
    ("cdot", sym(binary("⋅", "\\cdot"))),
    ("div", sym(binary("÷", "\\div"))),
    ("pm", sym(binary("±", "\\pm"))),
    ("mp", sym(binary("∓", "\\mp"))),
    ("cup", sym(binary("∪", "\\cup"))),
    ("cap", sym(binary("∩", "\\cap"))),
    ("and", sym(binary("∧", "\\wedge"))),
    ("wedge", sym(binary("∧", "\\wedge"))),
    ("or", sym(binary("∨", "\\vee"))),
    ("vee", sym(binary("∨", "\\vee"))),
    ("setminus", sym(binary("∖", "\\setminus"))),
    ("circ", sym(binary("∘", "\\circ"))),
    ("ox", sym(binary("⊗", "\\otimes"))),
    ("otimes", sym(binary("⊗", "\\otimes"))),
    ("oplus", sym(binary("⊕", "\\oplus"))),
    ("star", sym(binary("⋆", "\\star"))),
    ("mod", sym(binary("mod", "\\bmod"))),
    // Large operators.
    ("sum", sym(large("∑", "\\sum", true))),
    ("prod", sym(large("∏", "\\prod", true))),
    ("bigcup", sym(large("⋃", "\\bigcup", true))),
    ("bigcap", sym(large("⋂", "\\bigcap", true))),
    ("int", sym(large("∫", "\\int", false))),
    ("iint", sym(large("∬", "\\iint", false))),
    ("iiint", sym(large("∭", "\\iiint", false))),
    ("oint", sym(large("∮", "\\oint", false))),
    ("lim", sym(large("lim", "\\lim", true))),
    ("limsup", sym(large("lim sup", "\\limsup", true))),
    ("liminf", sym(large("lim inf", "\\liminf", true))),
    ("max", sym(large("max", "\\max", true))),
    ("min", sym(large("min", "\\min", true))),
    ("sup", sym(large("sup", "\\sup", true))),
    ("supremum", sym(large("sup", "\\sup", true))),
    ("infimum", sym(INFIMUM)),
    (
        "argmax",
        sym(large("arg max", "\\operatorname*{arg\\,max}", true)),
    ),
    (
        "argmin",
        sym(large("arg min", "\\operatorname*{arg\\,min}", true)),
    ),
    // Upright function names.
    ("sin", sym(function("sin", "\\sin"))),
    ("cos", sym(function("cos", "\\cos"))),
    ("tan", sym(function("tan", "\\tan"))),
    ("sec", sym(function("sec", "\\sec"))),
    ("csc", sym(function("csc", "\\csc"))),
    ("cot", sym(function("cot", "\\cot"))),
    ("arcsin", sym(function("arcsin", "\\arcsin"))),
    ("arccos", sym(function("arccos", "\\arccos"))),
    ("arctan", sym(function("arctan", "\\arctan"))),
    ("sinh", sym(function("sinh", "\\sinh"))),
    ("cosh", sym(function("cosh", "\\cosh"))),
    ("tanh", sym(function("tanh", "\\tanh"))),
    ("coth", sym(function("coth", "\\coth"))),
    ("log", sym(function("log", "\\log"))),
    ("ln", sym(function("ln", "\\ln"))),
    ("lg", sym(function("lg", "\\lg"))),
    ("exp", sym(function("exp", "\\exp"))),
    ("det", sym(function("det", "\\det"))),
    ("dim", sym(function("dim", "\\dim"))),
    ("ker", sym(function("ker", "\\ker"))),
    ("gcd", sym(function("gcd", "\\gcd"))),
    ("deg", sym(function("deg", "\\deg"))),
    ("arg", sym(function("arg", "\\arg"))),
    ("hom", sym(function("hom", "\\hom"))),
    ("Pr", sym(function("Pr", "\\Pr"))),
    ("lcm", sym(function("lcm", "\\operatorname{lcm}"))),
    ("sgn", sym(function("sgn", "\\operatorname{sgn}"))),
    ("tr", sym(function("tr", "\\operatorname{tr}"))),
    ("rank", sym(function("rank", "\\operatorname{rank}"))),
    ("span", sym(function("span", "\\operatorname{span}"))),
    ("Re", sym(function("Re", "\\operatorname{Re}"))),
    ("Im", sym(function("Im", "\\operatorname{Im}"))),
    // Structures that take an argument.
    ("sqrt", Keyword::Sqrt),
    ("root", Keyword::Root),
    (
        "abs",
        Keyword::Fence {
            open: "|",
            close: "|",
        },
    ),
    (
        "norm",
        Keyword::Fence {
            open: "‖",
            close: "‖",
        },
    ),
    (
        "floor",
        Keyword::Fence {
            open: "⌊",
            close: "⌋",
        },
    ),
    (
        "ceil",
        Keyword::Fence {
            open: "⌈",
            close: "⌉",
        },
    ),
    (
        "ang",
        Keyword::Fence {
            open: "⟨",
            close: "⟩",
        },
    ),
    ("hat", Keyword::Accent(AccentKind::Hat)),
    ("bar", Keyword::Accent(AccentKind::Bar)),
    ("vec", Keyword::Accent(AccentKind::Vector)),
    ("dot", Keyword::Accent(AccentKind::Dot)),
    ("ddot", Keyword::Accent(AccentKind::DoubleDot)),
    ("tilde", Keyword::Accent(AccentKind::Tilde)),
    ("cal", Keyword::Font(Font::Calligraphic)),
    ("bb", Keyword::Font(Font::Bold)),
    ("bbb", Keyword::Font(Font::Blackboard)),
    (
        "cases",
        Keyword::Matrix {
            open: "{",
            close: "",
        },
    ),
    (
        "mat",
        Keyword::Matrix {
            open: "(",
            close: ")",
        },
    ),
    (
        "pmat",
        Keyword::Matrix {
            open: "(",
            close: ")",
        },
    ),
    (
        "bmat",
        Keyword::Matrix {
            open: "[",
            close: "]",
        },
    ),
    (
        "vmat",
        Keyword::Matrix {
            open: "|",
            close: "|",
        },
    ),
    (
        "quad",
        Keyword::Space {
            width: SpaceWidth::Quad,
            count: 1,
        },
    ),
    (
        "qquad",
        Keyword::Space {
            width: SpaceWidth::Quad,
            count: 2,
        },
    ),
    // Pen-order helpers.
    ("over", Keyword::Over),
    ("sq", Keyword::Postfix(TWO)),
    ("cb", Keyword::Postfix(THREE)),
    ("inv", Keyword::Postfix(MINUS_ONE)),
    ("if", Keyword::Prose),
    ("else", Keyword::Prose),
    ("otherwise", Keyword::Prose),
    ("for", Keyword::Prose),
    ("when", Keyword::Prose),
    ("where", Keyword::Prose),
];

/// Operator spellings produced by the lexer, longest spellings first within
/// each shared prefix (the lexer relies on [`MULTI_CHAR_OPERATORS`]).
static OPERATORS: &[(&str, Symbol)] = &[
    ("<=>", relation("⇔", "\\Leftrightarrow")),
    ("|->", relation("↦", "\\mapsto")),
    ("...", ordinary("…", "\\ldots")),
    ("!in", relation("∉", "\\notin")),
    ("->", relation("→", "\\to")),
    ("<-", relation("←", "\\leftarrow")),
    ("=>", relation("⇒", "\\Rightarrow")),
    ("<=", relation("≤", "\\leq")),
    (">=", relation("≥", "\\geq")),
    ("!=", relation("≠", "\\neq")),
    ("~=", relation("≈", "\\approx")),
    ("==", relation("≡", "\\equiv")),
    ("+-", binary("±", "\\pm")),
    ("-+", binary("∓", "\\mp")),
    ("//", ordinary("/", "/")),
    ("/=", relation("≠", "\\neq")),
    ("**", binary("∗", "\\ast")),
    (":=", relation("≔", ":=")),
    ("<<", relation("≪", "\\ll")),
    (">>", relation("≫", "\\gg")),
    ("||", ordinary("‖", "\\|")),
    ("+", binary("+", "+")),
    ("-", binary("−", "-")),
    ("*", binary("⋅", "\\cdot")),
    ("=", relation("=", "=")),
    ("<", relation("<", "<")),
    (">", relation(">", ">")),
    ("~", relation("∼", "\\sim")),
    (":", punctuation(":", ":")),
    (",", punctuation(",", ",")),
    (";", punctuation(";", ";")),
    ("!", ordinary("!", "!")),
    ("|", ordinary("|", "|")),
    (".", ordinary(".", ".")),
    ("?", ordinary("?", "?")),
    ("@", binary("∘", "\\circ")),
    ("&", ordinary("&", "\\&")),
    ("%", ordinary("%", "\\%")),
    ("#", ordinary("#", "\\#")),
    ("$", ordinary("$", "\\$")),
];

/// Multi-character operator spellings, longest first, for the lexer.
pub(crate) const MULTI_CHAR_OPERATORS: [&str; 20] = [
    "<=>", "|->", "...", "->", "<-", "=>", "<=", ">=", "!=", "~=", "==", "+-", "-+", "//", "/=",
    "**", ":=", "<<", ">>", "||",
];

/// Single characters the lexer turns into operator tokens.
pub(crate) const SINGLE_CHAR_OPERATORS: &str = "+-*=<>~:,;!|.?@&%#$";

/// Whether an operator takes spoken limits (`sum i=1 to n`, `lim x to 0`).
/// Function names such as `sin` are operators too but never do.
pub(crate) fn takes_word_limits(operator: &Atom) -> bool {
    match operator.class {
        AtomClass::LargeOperator { limits } => limits || !starts_with_letter(operator),
        _ => false,
    }
}

fn starts_with_letter(atom: &Atom) -> bool {
    atom.text.starts_with(|c: char| c.is_ascii_alphabetic())
}

/// Whether a large operator reads `to` as an arrow (`lim x to 0`) rather than
/// as "up to" (`sum i=1 to n`). Word-like operators (lim, max, sup, …) do.
pub(crate) fn reads_to_as_arrow(operator: &Atom) -> bool {
    starts_with_letter(operator)
}

/// FNV-1a: keyword lookups happen for every word on every keystroke, and the
/// default SipHash showed up at ~8% of document parsing time in profiles.
/// Keys are fixed, so hash-flooding resistance is not needed.
#[derive(Default)]
struct FnvHasher(u64);

impl Hasher for FnvHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
        const PRIME: u64 = 0x0100_0000_01b3;
        let mut hash = if self.0 == 0 { OFFSET_BASIS } else { self.0 };
        for byte in bytes {
            hash = (hash ^ u64::from(*byte)).wrapping_mul(PRIME);
        }
        self.0 = hash;
    }
}

type FastMap<V> = HashMap<&'static str, V, BuildHasherDefault<FnvHasher>>;

fn keyword_table() -> &'static FastMap<Keyword> {
    static TABLE: OnceLock<FastMap<Keyword>> = OnceLock::new();
    TABLE.get_or_init(|| KEYWORDS.iter().copied().collect())
}

fn symbols_by_text() -> &'static FastMap<Symbol> {
    static TABLE: OnceLock<FastMap<Symbol>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = FastMap::default();
        let keyword_symbols = KEYWORDS.iter().filter_map(|(_, keyword)| match keyword {
            Keyword::Symbol(symbol) => Some(*symbol),
            _ => None,
        });
        for symbol in OPERATORS
            .iter()
            .map(|(_, symbol)| *symbol)
            .chain(keyword_symbols)
        {
            table.entry(symbol.text).or_insert(symbol);
        }
        table
    })
}

/// Looks up a math keyword such as `alpha`, `sum` or `sqrt`.
pub(crate) fn keyword(word: &str) -> Option<Keyword> {
    keyword_table().get(word).copied()
}

/// Whether `word` has a meaning of its own (used by the highlighter).
pub(crate) fn is_keyword(word: &str) -> bool {
    keyword_table().contains_key(word)
}

/// The atom for an operator token produced by the lexer.
pub(crate) fn operator_atom(spelling: &str) -> Atom {
    match OPERATORS
        .iter()
        .find(|(candidate, _)| *candidate == spelling)
    {
        Some((_, symbol)) => symbol.to_atom(),
        None => unknown_char_atom(spelling),
    }
}

/// The Unicode text of an operator token, or `""` if unknown.
pub(crate) fn operator_text(spelling: &str) -> &'static str {
    OPERATORS
        .iter()
        .find(|(candidate, _)| *candidate == spelling)
        .map_or("", |(_, symbol)| symbol.text)
}

/// The class an operator token will get, without building the atom.
pub(crate) fn operator_class(spelling: &str) -> AtomClass {
    OPERATORS
        .iter()
        .find(|(candidate, _)| *candidate == spelling)
        .map_or(AtomClass::Ordinary, |(_, symbol)| symbol.class)
}

/// The atom for a character typed directly, e.g. `α` or `≤`.
pub(crate) fn atom_for_char(c: char) -> Atom {
    let mut buffer = [0u8; 4];
    let text: &str = c.encode_utf8(&mut buffer);
    if let Some(symbol) = symbols_by_text().get(text) {
        return symbol.to_atom();
    }
    if c.is_ascii_alphabetic() {
        return Atom {
            text: text.to_string(),
            latex: text.to_string(),
            class: AtomClass::Ordinary,
            style: AtomStyle::Italic,
        };
    }
    unknown_char_atom(text)
}

fn unknown_char_atom(text: &str) -> Atom {
    let is_safe_in_math = text
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "()[]/'+-=<>!?.,;:|*".contains(c));
    let latex = if is_safe_in_math {
        text.to_string()
    } else {
        format!("\\text{{{}}}", escape_text(text))
    };
    Atom {
        text: text.to_string(),
        latex,
        class: AtomClass::Ordinary,
        style: AtomStyle::Upright,
    }
}

/// The atom for a backslash escape such as `\$` or `\{`.
pub(crate) fn escaped_atom(c: char) -> Atom {
    let (text, latex, class) = match c {
        '{' => ("{", "\\{", AtomClass::Open),
        '}' => ("}", "\\}", AtomClass::Close),
        '$' => ("$", "\\$", AtomClass::Ordinary),
        '%' => ("%", "\\%", AtomClass::Ordinary),
        '&' => ("&", "\\&", AtomClass::Ordinary),
        '#' => ("#", "\\#", AtomClass::Ordinary),
        '_' => ("_", "\\_", AtomClass::Ordinary),
        '\\' => ("\\", "\\backslash", AtomClass::Ordinary),
        '|' => ("‖", "\\|", AtomClass::Ordinary),
        _ => return atom_for_char(c),
    };
    Atom {
        text: text.to_string(),
        latex: latex.to_string(),
        class,
        style: AtomStyle::Upright,
    }
}

/// Re-styles a single atom for `cal(…)`, `bb(…)` or `bbb(…)`.
pub(crate) fn apply_font_to_atom(font: Font, atom: Atom) -> Atom {
    let letter = single_ascii_alphanumeric(&atom.text);
    match (font, letter) {
        (Font::Bold, Some(c)) => Atom {
            latex: format!("\\mathbf{{{c}}}"),
            style: AtomStyle::Bold,
            ..atom
        },
        (Font::Bold, None) => Atom {
            latex: format!("\\boldsymbol{{{}}}", atom.latex),
            style: AtomStyle::Bold,
            ..atom
        },
        (Font::Calligraphic, Some(c)) if c.is_ascii_alphabetic() => {
            styled_letter(c, script_letter(c), "mathcal")
        }
        (Font::Blackboard, Some(c)) => styled_letter(c, double_struck(c), "mathbb"),
        _ => atom,
    }
}

fn styled_letter(c: char, styled: char, command: &str) -> Atom {
    Atom {
        text: styled.to_string(),
        latex: format!("\\{command}{{{c}}}"),
        class: AtomClass::Ordinary,
        style: AtomStyle::Upright,
    }
}

fn single_ascii_alphanumeric(text: &str) -> Option<char> {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphanumeric() => Some(c),
        _ => None,
    }
}

/// Unicode "Mathematical Script" letters; some live in the Letterlike block.
fn script_letter(c: char) -> char {
    let exception = match c {
        'B' => Some('ℬ'),
        'E' => Some('ℰ'),
        'F' => Some('ℱ'),
        'H' => Some('ℋ'),
        'I' => Some('ℐ'),
        'L' => Some('ℒ'),
        'M' => Some('ℳ'),
        'R' => Some('ℛ'),
        'e' => Some('ℯ'),
        'g' => Some('ℊ'),
        'o' => Some('ℴ'),
        _ => None,
    };
    exception.unwrap_or_else(|| offset_letter(c, 0x1D49C, 0x1D4B6, None))
}

/// Unicode "Mathematical Double-Struck" letters and digits.
fn double_struck(c: char) -> char {
    let exception = match c {
        'C' => Some('ℂ'),
        'H' => Some('ℍ'),
        'N' => Some('ℕ'),
        'P' => Some('ℙ'),
        'Q' => Some('ℚ'),
        'R' => Some('ℝ'),
        'Z' => Some('ℤ'),
        _ => None,
    };
    exception.unwrap_or_else(|| offset_letter(c, 0x1D538, 0x1D552, Some(0x1D7D8)))
}

fn offset_letter(c: char, upper_base: u32, lower_base: u32, digit_base: Option<u32>) -> char {
    let code = match c {
        'A'..='Z' => upper_base + (c as u32 - 'A' as u32),
        'a'..='z' => lower_base + (c as u32 - 'a' as u32),
        '0'..='9' => match digit_base {
            Some(base) => base + (c as u32 - '0' as u32),
            None => return c,
        },
        _ => return c,
    };
    char::from_u32(code).unwrap_or(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_multi_char_operator_has_a_symbol() {
        for spelling in MULTI_CHAR_OPERATORS {
            assert!(OPERATORS.iter().any(|(s, _)| *s == spelling), "{spelling}");
        }
    }

    #[test]
    fn every_single_char_operator_has_a_symbol() {
        for c in SINGLE_CHAR_OPERATORS.chars() {
            let spelling = c.to_string();
            assert!(OPERATORS.iter().any(|(s, _)| *s == spelling), "{spelling}");
        }
    }

    #[test]
    fn multi_char_operators_are_ordered_longest_first_per_prefix() {
        for (i, earlier) in MULTI_CHAR_OPERATORS.iter().enumerate() {
            for later in &MULTI_CHAR_OPERATORS[i + 1..] {
                assert!(
                    !later.starts_with(earlier),
                    "{later} is shadowed by {earlier}"
                );
            }
        }
    }

    #[test]
    fn keywords_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for (word, _) in KEYWORDS {
            assert!(seen.insert(word), "duplicate keyword {word}");
        }
    }

    #[test]
    fn script_and_double_struck_letters_use_letterlike_exceptions() {
        assert_eq!(script_letter('A'), '𝒜');
        assert_eq!(script_letter('B'), 'ℬ');
        assert_eq!(double_struck('R'), 'ℝ');
        assert_eq!(double_struck('A'), '𝔸');
        assert_eq!(double_struck('1'), '𝟙');
    }

    #[test]
    fn typed_unicode_symbol_maps_back_to_latex() {
        assert_eq!(atom_for_char('α').latex, "\\alpha");
        assert_eq!(atom_for_char('≤').latex, "\\leq");
        assert_eq!(atom_for_char('ℝ').latex, "\\mathbb{R}");
    }

    #[test]
    fn unknown_unicode_char_is_wrapped_in_text() {
        assert_eq!(atom_for_char('é').latex, "\\text{é}");
    }
}
