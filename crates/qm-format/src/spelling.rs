//! The prose words that spell checking looks at. Uses the highlighter, so
//! words are checked exactly where the parser reads them as prose and never
//! in math, code, wikilinks or front matter.

use std::ops::Range;

use crate::highlight::highlight;
use crate::TokenKind;

/// Shorter words are mostly variables in prose ("let f be").
const MIN_WORD_CHARS: usize = 2;
/// Marks a web address, whose parts are not words.
const URL_SCHEME_SEPARATOR: &str = "://";

pub(crate) fn spell_check_words(source: &str) -> Vec<Range<usize>> {
    highlight(source)
        .into_iter()
        .filter(|span| is_prose(span.kind))
        .flat_map(|span| words_in_span(source, span.range))
        .collect()
}

fn is_prose(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Text | TokenKind::Heading | TokenKind::Emphasis | TokenKind::Strong
    )
}

fn words_in_span(source: &str, span: Range<usize>) -> Vec<Range<usize>> {
    let mut words = Vec::new();
    let mut offset = span.start;
    for chunk in source[span].split_inclusive(char::is_whitespace) {
        if !chunk.contains(URL_SCHEME_SEPARATOR) {
            words.extend(
                word_candidates(chunk)
                    .into_iter()
                    .filter(|word| is_worth_checking(&chunk[word.clone()]))
                    .map(|word| offset + word.start..offset + word.end),
            );
        }
        offset += chunk.len();
    }
    words
}

/// Runs of letters, digits and `_`, joined across single apostrophes and
/// hyphens (`don't`, `well-known`), but not across leading or trailing ones.
fn word_candidates(text: &str) -> Vec<Range<usize>> {
    let mut words = Vec::new();
    let mut start = None;
    let mut chars = text.char_indices().peekable();
    while let Some((index, c)) = chars.next() {
        let is_word_char = c.is_alphanumeric() || c == '_';
        let joins_letters = is_joiner(c)
            && start.is_some()
            && chars.peek().is_some_and(|(_, next)| next.is_alphanumeric());
        if is_word_char || joins_letters {
            start.get_or_insert(index);
        } else if let Some(word_start) = start.take() {
            words.push(word_start..index);
        }
    }
    if let Some(word_start) = start {
        words.push(word_start..text.len());
    }
    words
}

fn is_joiner(c: char) -> bool {
    matches!(c, '\'' | '’' | '-')
}

/// Leaves out what is not a dictionary word: single letters, names with
/// digits or `_` (`x2`, `f_n`) and abbreviations in capitals (`PDE`).
fn is_worth_checking(word: &str) -> bool {
    let has_digit_or_underscore = word.chars().any(|c| c.is_numeric() || c == '_');
    let is_abbreviation = word
        .chars()
        .filter(|c| c.is_alphabetic())
        .all(char::is_uppercase);
    word.chars().count() >= MIN_WORD_CHARS && !has_digit_or_underscore && !is_abbreviation
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidates(text: &str) -> Vec<&str> {
        word_candidates(text)
            .into_iter()
            .map(|range| &text[range])
            .collect()
    }

    #[test]
    fn candidates_join_inner_apostrophes_and_hyphens_only() {
        assert_eq!(candidates("don't well-known"), ["don't", "well-known"]);
        assert_eq!(
            candidates("'quoted' -dash- a--b"),
            ["quoted", "dash", "a", "b"]
        );
        assert_eq!(candidates("l’homme"), ["l’homme"]);
    }

    #[test]
    fn variables_numbers_and_abbreviations_are_not_checked() {
        assert!(is_worth_checking("germs"));
        assert!(is_worth_checking("Riemann"));
        assert!(!is_worth_checking("f"));
        assert!(!is_worth_checking("x2"));
        assert!(!is_worth_checking("f_n"));
        assert!(!is_worth_checking("PDE"));
    }
}
