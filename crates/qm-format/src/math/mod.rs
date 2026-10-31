//! Math notation: lexer, parser, symbol table and LaTeX output.

mod latex;
mod lexer;
mod parser;
mod symbols;

pub(crate) use latex::{display_math_to_latex, math_to_latex};
pub(crate) use parser::parse_math;

use crate::ast::{AtomClass, MathList, MathNode, MathRow};
use crate::highlight::SpanSink;
use crate::TokenKind as Highlight;
use lexer::{lex, TokenKind};

/// Parses one display-math row. Rows align at their first relation (`=`,
/// `<`, `->`, …) unless the row marks its alignment points with `&`.
pub(crate) fn parse_display_row(source: &str) -> MathRow {
    if has_unquoted_ampersand(source) {
        let cells = split_unquoted_ampersands(source)
            .into_iter()
            .map(parse_math)
            .collect();
        return MathRow { cells };
    }
    MathRow {
        cells: split_at_first_relation(parse_math(source)),
    }
}

fn split_at_first_relation(mut list: MathList) -> Vec<MathList> {
    let is_relation =
        |node: &MathNode| matches!(node, MathNode::Atom(atom) if atom.class == AtomClass::Relation);
    match list.iter().position(is_relation) {
        Some(position) => {
            let right = list.split_off(position);
            vec![list, right]
        }
        None => vec![list],
    }
}

fn has_unquoted_ampersand(source: &str) -> bool {
    split_unquoted_ampersands(source).len() > 1
}

/// Splits at `&` outside `"…"` and `` `…` `` text.
fn split_unquoted_ampersands(source: &str) -> Vec<&str> {
    let mut cells = Vec::new();
    let mut cell_start = 0;
    let mut open_quote: Option<char> = None;
    for (index, c) in source.char_indices() {
        match (open_quote, c) {
            (None, '"' | '`') => open_quote = Some(c),
            (Some(quote), _) if c == quote => open_quote = None,
            (None, '&') => {
                cells.push(&source[cell_start..index]);
                cell_start = index + 1;
            }
            _ => {}
        }
    }
    cells.push(&source[cell_start..]);
    cells
}

/// Highlights math source located at byte `offset` of the document.
pub(crate) fn highlight_math(source: &str, offset: usize, sink: &mut SpanSink) {
    for token in lex(source) {
        let text = &source[token.start..token.end];
        sink.push(
            offset + token.start..offset + token.end,
            highlight_kind(token.kind, text),
        );
    }
}

fn highlight_kind(kind: TokenKind, text: &str) -> Highlight {
    match kind {
        TokenKind::Number => Highlight::MathNumber,
        TokenKind::Word if symbols::is_keyword(text) => Highlight::MathKeyword,
        TokenKind::Word | TokenKind::Other => Highlight::MathIdentifier,
        TokenKind::Command if symbols::is_keyword(&text[1..]) => Highlight::MathKeyword,
        TokenKind::Command | TokenKind::LoneBackslash => Highlight::Error,
        TokenKind::Quote { .. } => Highlight::MathText,
        TokenKind::Operator if text == "&" => Highlight::Markup,
        TokenKind::Operator
        | TokenKind::Escape
        | TokenKind::Open
        | TokenKind::Close
        | TokenKind::Underscore
        | TokenKind::Caret
        | TokenKind::Slash
        | TokenKind::Prime => Highlight::MathOperator,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_aligns_at_first_relation() {
        let row = parse_display_row("f(x) = x + 1 = y");
        assert_eq!(row.cells.len(), 2);
        assert_eq!(math_to_latex(&row.cells[1]), "=x+1=y");
    }

    #[test]
    fn row_starting_with_relation_has_empty_left_cell() {
        let row = parse_display_row("= 2x");
        assert!(row.cells[0].is_empty());
    }

    #[test]
    fn explicit_ampersand_overrides_automatic_alignment() {
        let row = parse_display_row("a & = b & c");
        assert_eq!(row.cells.len(), 3);
    }

    #[test]
    fn ampersand_inside_quotes_does_not_split() {
        assert_eq!(parse_display_row("x \"a & b\"").cells.len(), 1);
    }
}
