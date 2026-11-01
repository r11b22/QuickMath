//! Suggestions for the math word being typed, drawn from the syntax file's
//! vocabulary. Uses the highlighter to decide whether the cursor is in math,
//! so suggestions appear exactly where the parser reads words as math.

use std::ops::Range;

use crate::highlight::highlight;
use crate::math::{keyword_symbol_text, vocabulary};
use crate::{MathCompletion, MathCompletions, TokenKind};

/// Single letters are usually variables, so suggesting on them would be noise.
const MIN_QUERY_LEN: usize = 2;
/// Below this, gapped matches are mostly noise (`sq` would match `simeq`).
const MIN_SUBSEQUENCE_QUERY_LEN: usize = 3;
const MAX_CANDIDATES: usize = 8;

pub(crate) fn math_completions(source: &str, cursor: usize) -> Option<MathCompletions> {
    let range = word_before(source, cursor)?;
    if !is_math_word(source, range.start) {
        return None;
    }
    let candidates = rank_candidates(&source[range.clone()]);
    let is_only_the_typed_word =
        candidates.len() == 1 && candidates[0].entry.spelling == &source[range.clone()];
    if candidates.is_empty() || is_only_the_typed_word {
        return None;
    }
    Some(MathCompletions { range, candidates })
}

/// The ASCII letters directly before `cursor`, if the cursor ends a word
/// rather than sitting inside one.
fn word_before(source: &str, cursor: usize) -> Option<Range<usize>> {
    if !source.is_char_boundary(cursor) {
        return None;
    }
    let continues_after = source[cursor..].starts_with(|c: char| c.is_ascii_alphanumeric());
    if continues_after {
        return None;
    }
    let letters = source[..cursor]
        .bytes()
        .rev()
        .take_while(u8::is_ascii_alphabetic)
        .count();
    (letters >= MIN_QUERY_LEN).then(|| cursor - letters..cursor)
}

/// Whether the word starting at byte `start` is a math word (bare or after
/// `\`), not prose, code or quoted text inside math.
fn is_math_word(source: &str, start: usize) -> bool {
    let after_backslash = source[..start].ends_with('\\');
    highlight(source)
        .iter()
        .find(|span| span.range.contains(&start))
        .is_some_and(|span| match span.kind {
            TokenKind::MathIdentifier | TokenKind::MathKeyword => true,
            // An unknown `\word` is highlighted as an error until it is complete.
            TokenKind::Error => after_backslash,
            _ => false,
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MatchQuality {
    Prefix,
    /// Same first letter, the rest in order with gaps: `lmd` for `lambda`.
    Subsequence,
}

/// Prefix matches before looser ones, then the shortest spelling (fewest
/// keystrokes), then syntax-file order. Aliases for the same symbol
/// (`eps`, `epsilon`) are listed once, under the best-ranked spelling.
fn rank_candidates(query: &str) -> Vec<MathCompletion> {
    let mut matches: Vec<(MatchQuality, MathCompletion)> = vocabulary()
        .iter()
        .flat_map(|section| {
            section.entries.iter().map(|entry| MathCompletion {
                entry: *entry,
                section: section.title,
            })
        })
        .filter_map(|candidate| Some((match_quality(query, candidate.entry.spelling)?, candidate)))
        .collect();
    matches.sort_by_key(|(quality, candidate)| (*quality, candidate.entry.spelling.len()));

    let mut seen_symbols = Vec::new();
    matches
        .into_iter()
        .map(|(_, candidate)| candidate)
        .filter(
            |candidate| match keyword_symbol_text(candidate.entry.spelling) {
                Some(text) if seen_symbols.contains(&text) => false,
                Some(text) => {
                    seen_symbols.push(text);
                    true
                }
                None => true,
            },
        )
        .take(MAX_CANDIDATES)
        .collect()
}

/// Operators (`<=`) never match: only words are completed.
fn match_quality(query: &str, spelling: &str) -> Option<MatchQuality> {
    if spelling.starts_with(query) {
        return Some(MatchQuality::Prefix);
    }
    if query.len() < MIN_SUBSEQUENCE_QUERY_LEN {
        return None;
    }
    let mut query_chars = query.chars();
    let mut spelling_chars = spelling.chars();
    if query_chars.next()? != spelling_chars.next()? {
        return None;
    }
    let mut remaining = query_chars.peekable();
    for c in spelling_chars {
        if remaining.peek() == Some(&c) {
            remaining.next();
        }
    }
    remaining
        .peek()
        .is_none()
        .then_some(MatchQuality::Subsequence)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_before_needs_the_cursor_at_a_word_end() {
        assert_eq!(word_before("x alp", 5), Some(2..5));
        assert_eq!(word_before("x alpha", 5), None);
        assert_eq!(word_before("x a", 3), None);
        assert_eq!(word_before("α", 1), None);
    }

    #[test]
    fn subsequence_keeps_first_letter_and_order() {
        assert_eq!(
            match_quality("lmd", "lambda"),
            Some(MatchQuality::Subsequence)
        );
        assert_eq!(match_quality("lam", "lambda"), Some(MatchQuality::Prefix));
        assert_eq!(match_quality("amb", "lambda"), None);
        assert_eq!(match_quality("ldm", "lambda"), None);
        assert_eq!(match_quality("le", "<="), None);
        assert_eq!(match_quality("sq", "simeq"), None);
    }
}
