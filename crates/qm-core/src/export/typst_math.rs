//! Conversion of QuickMath math syntax trees to Typst math markup.
//!
//! Typst math reads multi-letter words as variable names and gives ASCII
//! punctuation structural meaning (`/` makes fractions, `,` separates
//! arguments, `"` starts text, …). To stay robust for every tree the parser
//! may produce, this module:
//!
//! - emits every node as a self-contained, space-separated token;
//! - backslash-escapes all ASCII punctuation in atoms;
//! - quotes multi-character atoms as strings (`op("sin")` for operators);
//! - uses explicit function calls (`frac`, `attach`, `lr`, `mat`, …) instead
//!   of Typst's shorthand syntax;
//! - replaces empty arguments with `#none`, always followed by a space so a
//!   `;` row separator is never parsed as the end of the `#` expression.

use qm_format::{AccentKind, Atom, AtomClass, AtomStyle, MathList, MathNode, MathRow, SpaceWidth};

use unicode_math_class::MathClass;

use super::typst_markup::string_literal;

const MINUS_SIGN: char = '\u{2212}';

/// Typst source for a sequence of math nodes (empty if nothing is visible).
pub(crate) fn math_list(list: &[MathNode]) -> String {
    list.iter()
        .map(node)
        .filter(|markup| !markup.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Typst source for display-math rows: cells joined by `&`, rows by `\`.
pub(crate) fn math_rows(rows: &[MathRow]) -> String {
    rows.iter()
        .map(|row| {
            row.cells
                .iter()
                .map(|cell| math_list(cell))
                .collect::<Vec<_>>()
                .join(" & ")
        })
        .collect::<Vec<_>>()
        .join(" \\\n")
}

/// Like [`math_list`], but never empty so it can be used as a function argument.
fn argument(list: &[MathNode]) -> String {
    non_empty(math_list(list))
}

fn non_empty(markup: String) -> String {
    if markup.trim().is_empty() {
        "#none".to_string()
    } else {
        markup
    }
}

fn node(node: &MathNode) -> String {
    match node {
        MathNode::Atom(atom) => self::atom(atom),
        MathNode::Text(text) if text.is_empty() => String::new(),
        MathNode::Text(text) => string_literal(text),
        MathNode::Fraction {
            numerator,
            denominator,
        } => format!("frac({} , {})", argument(numerator), argument(denominator)),
        MathNode::Scripts { base, sub, sup } => scripts(base, sub.as_deref(), sup.as_deref()),
        MathNode::Root { index, radicand } => match index {
            Some(index) => format!("root({} , {})", argument(index), argument(radicand)),
            None => format!("sqrt({})", argument(radicand)),
        },
        MathNode::Fenced { open, close, body } => fenced(open, close, body),
        MathNode::Matrix { open, close, rows } => matrix(open, close, rows),
        MathNode::Accent { accent, body } => {
            format!("{}({})", accent_function(*accent), argument(body))
        }
        MathNode::Group(list) => math_list(list),
        MathNode::Space(width) => space(*width).to_string(),
        MathNode::Error { source, .. } => {
            format!("#text(fill: rgb(\"#c62828\"), {})", string_literal(source))
        }
    }
}

fn atom(atom: &Atom) -> String {
    let text = atom.text.as_str();
    let mut characters = text.chars();
    let (Some(first), None) = (characters.next(), characters.next()) else {
        return multi_character_atom(atom);
    };
    if matches!(atom.class, AtomClass::LargeOperator { .. }) && first.is_ascii_alphanumeric() {
        return multi_character_atom(atom);
    }
    let symbol = single_character(first);
    match atom.style {
        // Like LaTeX's `\mathbf`: bold letters are upright.
        AtomStyle::Bold if first.is_alphabetic() => format!("bold(upright({symbol}))"),
        AtomStyle::Bold => format!("bold({symbol})"),
        AtomStyle::Upright if first.is_alphabetic() => format!("upright({symbol})"),
        AtomStyle::Upright | AtomStyle::Italic => symbol,
    }
}

/// Emits one character so Typst reads it literally: ASCII punctuation is
/// escaped, everything else (letters, digits, Unicode symbols) is a valid
/// single-character token as is.
fn single_character(character: char) -> String {
    if character == '-' {
        // An escaped hyphen would print as a hyphen, not a minus sign.
        return MINUS_SIGN.to_string();
    }
    if character.is_ascii_punctuation() {
        format!("\\{character}")
    } else {
        character.to_string()
    }
}

fn multi_character_atom(atom: &Atom) -> String {
    let text = atom.text.as_str();
    if text.is_empty() {
        return String::new();
    }
    let is_number = text.chars().all(|c| c.is_ascii_digit() || c == '.')
        && text.starts_with(|c: char| c.is_ascii_digit())
        && text.ends_with(|c: char| c.is_ascii_digit());
    if is_number {
        return match atom.style {
            AtomStyle::Bold => format!("bold({text})"),
            _ => text.to_string(),
        };
    }

    let literal = string_literal(text);
    let is_word = text.chars().all(char::is_alphabetic);
    let styled = match (atom.class, atom.style) {
        (AtomClass::LargeOperator { limits }, _) => format!("op({literal}, limits: #{limits})"),
        (AtomClass::Ordinary, AtomStyle::Upright) if is_word => format!("op({literal})"),
        (_, AtomStyle::Italic) => format!("italic({literal})"),
        (_, AtomStyle::Bold) => format!("bold({literal})"),
        (_, AtomStyle::Upright) => literal,
    };
    match class_name(atom.class) {
        Some(class) => format!("class(\"{class}\", {styled})"),
        None => styled,
    }
}

/// The Typst math class to force on a quoted atom (strings default to "normal").
fn class_name(class: AtomClass) -> Option<&'static str> {
    match class {
        AtomClass::Binary => Some("binary"),
        AtomClass::Relation => Some("relation"),
        AtomClass::Open => Some("opening"),
        AtomClass::Close => Some("closing"),
        AtomClass::Punctuation => Some("punctuation"),
        AtomClass::Ordinary | AtomClass::LargeOperator { .. } => None,
    }
}

fn scripts(base: &MathNode, sub: Option<&[MathNode]>, sup: Option<&[MathNode]>) -> String {
    let mut markup = format!("attach({}", script_base(base));
    if let Some(sub) = sub {
        markup.push_str(&format!(" , b: {}", argument(sub)));
    }
    if let Some(sup) = sup {
        markup.push_str(&format!(" , t: {}", argument(sup)));
    }
    markup.push(')');
    markup
}

/// Large operators get their script placement made explicit: limits above
/// and below in display style (like TeX), or always to the side.
fn script_base(base: &MathNode) -> String {
    let rendered = non_empty(node(base));
    match base {
        MathNode::Atom(Atom {
            class: AtomClass::LargeOperator { limits },
            text,
            ..
        }) if text.chars().count() == 1 => {
            if *limits {
                format!("limits({rendered}, inline: #false)")
            } else {
                format!("scripts({rendered})")
            }
        }
        _ => rendered,
    }
}

/// `lr` stretches the first and last item if they are delimiters. For a
/// one-sided fence the missing side gets an invisible zero-width space, so a
/// delimiter at the edge of the body (e.g. the `)` in `{ f(x)`) is not
/// stretched by mistake.
fn fenced(open: &str, close: &str, body: &[MathNode]) -> String {
    let body = math_list(body);
    let side = |delimiter: &str| {
        let mut characters = delimiter.chars();
        match (characters.next(), characters.next()) {
            (None, _) => "zws".to_string(),
            (Some(character), None) => single_character(character),
            _ => string_literal(delimiter),
        }
    };
    if open.is_empty() && close.is_empty() {
        return body;
    }
    let body = if body.is_empty() {
        "zws".to_string()
    } else {
        body
    };
    format!("lr({} {body} {})", side(open), side(close))
}

fn matrix(open: &str, close: &str, rows: &[Vec<MathList>]) -> String {
    let delimiters = format!("delim: #({}, {})", delimiter(open), delimiter(close));

    // Exactly one delimiter (e.g. a piecewise definition) reads best as
    // `cases`, which left-aligns cells; `&` then aligns the columns.
    let is_one_sided = open.is_empty() != close.is_empty();
    if is_one_sided {
        let mut arguments = vec![delimiters];
        if open.is_empty() {
            arguments.push("reverse: #true".to_string());
        }
        arguments.extend(rows.iter().map(|row| non_empty(cells(row).join(" & "))));
        return format!("cases({})", arguments.join(" , "));
    }

    let rows: Vec<String> = rows
        .iter()
        .map(|row| non_empty(cells(row).join(" , ")))
        .collect();
    if rows.is_empty() {
        return format!("mat({delimiters})");
    }
    format!("mat({delimiters} , {})", rows.join(" ; "))
}

/// A `mat`/`cases` delimiter argument. Typst only accepts characters whose
/// Unicode math class is a delimiter; anything else is dropped.
fn delimiter(text: &str) -> String {
    let mut characters = text.chars();
    match (characters.next(), characters.next()) {
        (Some(character), None) if is_delimiter(character) => {
            string_literal(&character.to_string())
        }
        _ => "none".to_string(),
    }
}

fn is_delimiter(character: char) -> bool {
    matches!(
        unicode_math_class::class(character),
        Some(MathClass::Opening | MathClass::Closing | MathClass::Fence)
    )
}

fn cells(row: &[MathList]) -> Vec<String> {
    row.iter().map(|cell| argument(cell)).collect()
}

fn accent_function(accent: AccentKind) -> &'static str {
    match accent {
        AccentKind::Hat => "hat",
        AccentKind::Bar => "macron",
        AccentKind::Vector => "arrow",
        AccentKind::Dot => "dot",
        AccentKind::DoubleDot => "dot.double",
        AccentKind::Tilde => "tilde",
    }
}

fn space(width: SpaceWidth) -> &'static str {
    match width {
        SpaceWidth::Thin => "thin",
        SpaceWidth::Medium => "med",
        SpaceWidth::Thick => "thick",
        SpaceWidth::Quad => "quad",
    }
}
