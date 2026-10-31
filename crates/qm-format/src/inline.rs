//! Inline markup within one line: `*emph*`, `**strong**`, `` `code` ``,
//! `;;math;;`, `[[wikilinks]]` and backslash escapes. Inline markup never
//! spans lines.

use std::ops::Range;

use crate::ast::Inline;
use crate::highlight::SpanSink;
use crate::line_syntax::MATH_DELIMITER;
use crate::math::{highlight_math, parse_math};
use crate::TokenKind;

#[derive(Debug, Clone, PartialEq, Eq)]
enum InlineToken {
    Text(Range<usize>),
    /// `\` plus one ASCII punctuation character, which is kept literally.
    Escape(Range<usize>),
    Code {
        range: Range<usize>,
        content: Range<usize>,
    },
    Math {
        range: Range<usize>,
        content: Range<usize>,
        is_closed: bool,
    },
    WikiLink {
        range: Range<usize>,
        inner: Range<usize>,
    },
    /// A run of one or two `*`.
    Stars {
        range: Range<usize>,
        can_open: bool,
        can_close: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StarRole {
    Literal,
    Open,
    Close,
}

/// Parses the inline content of one line.
pub(crate) fn parse_inlines(line: &str) -> Vec<Inline> {
    let tokens = tokenize(line);
    let roles = resolve_emphasis(&tokens);
    build_inlines(line, &tokens, &roles)
}

/// Highlights one line of inline content located at byte `offset`; plain
/// text gets `base` (e.g. [`TokenKind::Heading`] inside headings).
pub(crate) fn highlight_inlines(line: &str, offset: usize, base: TokenKind, sink: &mut SpanSink) {
    let tokens = tokenize(line);
    let roles = resolve_emphasis(&tokens);
    let mut strong_depth = 0usize;
    let mut emphasis_depth = 0usize;
    for (token, role) in tokens.iter().zip(&roles) {
        let text_kind = match (strong_depth, emphasis_depth) {
            (0, 0) => base,
            (0, _) => TokenKind::Emphasis,
            _ => TokenKind::Strong,
        };
        let shifted = |range: &Range<usize>| offset + range.start..offset + range.end;
        match token {
            InlineToken::Text(range) | InlineToken::Escape(range) => {
                sink.push(shifted(range), text_kind)
            }
            InlineToken::Code { range, .. } => sink.push(shifted(range), TokenKind::Code),
            InlineToken::WikiLink { range, .. } => sink.push(shifted(range), TokenKind::WikiLink),
            InlineToken::Math { range, content, .. } => {
                sink.push(
                    offset + range.start..offset + content.start,
                    TokenKind::Markup,
                );
                highlight_math(&line[content.clone()], offset + content.start, sink);
                sink.push(offset + content.end..offset + range.end, TokenKind::Markup);
            }
            InlineToken::Stars { range, .. } => {
                let is_strong = range.len() == 2;
                let depth = if is_strong {
                    &mut strong_depth
                } else {
                    &mut emphasis_depth
                };
                match role {
                    StarRole::Literal => sink.push(shifted(range), text_kind),
                    StarRole::Open => {
                        *depth += 1;
                        sink.push(shifted(range), TokenKind::Markup);
                    }
                    StarRole::Close => {
                        *depth = depth.saturating_sub(1);
                        sink.push(shifted(range), TokenKind::Markup);
                    }
                }
            }
        }
    }
}

fn tokenize(line: &str) -> Vec<InlineToken> {
    let bytes = line.as_bytes();
    let mut tokens = Vec::new();
    let mut text_start = 0;
    let mut index = 0;
    while index < bytes.len() {
        let scanned = match bytes[index] {
            b'\\' => scan_escape(bytes, index),
            b'`' => Some(scan_code(line, index)),
            b';' => scan_math(line, index),
            b'[' => scan_wikilink(line, index),
            b'*' => Some(scan_stars(line, index)),
            _ => None,
        };
        let Some((token, end)) = scanned else {
            index += 1;
            continue;
        };
        if text_start < index {
            tokens.push(InlineToken::Text(text_start..index));
        }
        tokens.push(token);
        index = end;
        text_start = end;
    }
    if text_start < bytes.len() {
        tokens.push(InlineToken::Text(text_start..bytes.len()));
    }
    tokens
}

fn scan_escape(bytes: &[u8], index: usize) -> Option<(InlineToken, usize)> {
    let next = *bytes.get(index + 1)?;
    next.is_ascii_punctuation()
        .then(|| (InlineToken::Escape(index..index + 2), index + 2))
}

/// A run of backticks closed by the next run of the same length; unclosed
/// runs are literal text.
fn scan_code(line: &str, index: usize) -> (InlineToken, usize) {
    let run = run_length(line.as_bytes(), index, b'`');
    let content_start = index + run;
    let mut search = content_start;
    while let Some(offset) = line[search..].find('`') {
        let close = search + offset;
        let close_run = run_length(line.as_bytes(), close, b'`');
        if close_run == run {
            let token = InlineToken::Code {
                range: index..close + run,
                content: content_start..close,
            };
            return (token, close + run);
        }
        search = close + close_run;
    }
    (InlineToken::Text(index..content_start), content_start)
}

/// `;;math;;`. While typing, an unclosed formula extends to the end of the
/// line so the preview already shows it.
fn scan_math(line: &str, index: usize) -> Option<(InlineToken, usize)> {
    if !line[index..].starts_with(MATH_DELIMITER) {
        return None;
    }
    let content_start = index + MATH_DELIMITER.len();
    let bytes = line.as_bytes();
    let mut position = content_start;
    while position < bytes.len() {
        if bytes[position] == b'\\' {
            position += 2;
            continue;
        }
        if bytes[position..].starts_with(MATH_DELIMITER.as_bytes()) {
            let end = position + MATH_DELIMITER.len();
            let token = InlineToken::Math {
                range: index..end,
                content: content_start..position,
                is_closed: true,
            };
            return Some((token, end));
        }
        position += 1;
    }
    let token = InlineToken::Math {
        range: index..line.len(),
        content: content_start..line.len(),
        is_closed: false,
    };
    Some((token, line.len()))
}

fn scan_wikilink(line: &str, index: usize) -> Option<(InlineToken, usize)> {
    let inner_start = index + 2;
    if !line[index..].starts_with("[[") {
        return None;
    }
    let close = inner_start + line[inner_start..].find("]]")?;
    let target = line[inner_start..close].split('|').next().unwrap_or("");
    if target.trim().is_empty() {
        return None;
    }
    let end = close + 2;
    Some((
        InlineToken::WikiLink {
            range: index..end,
            inner: inner_start..close,
        },
        end,
    ))
}

fn scan_stars(line: &str, index: usize) -> (InlineToken, usize) {
    let run = run_length(line.as_bytes(), index, b'*');
    let end = index + run;
    if run > 2 {
        return (InlineToken::Text(index..end), end);
    }
    let can_open = line[end..]
        .chars()
        .next()
        .is_some_and(|c| !c.is_whitespace());
    let can_close = line[..index]
        .chars()
        .next_back()
        .is_some_and(|c| !c.is_whitespace());
    (
        InlineToken::Stars {
            range: index..end,
            can_open,
            can_close,
        },
        end,
    )
}

fn run_length(bytes: &[u8], index: usize, byte: u8) -> usize {
    bytes[index..].iter().take_while(|&&b| b == byte).count()
}

/// Pairs `*`/`**` delimiters: a closer matches the nearest opener of the
/// same length; openers skipped over become literal text.
fn resolve_emphasis(tokens: &[InlineToken]) -> Vec<StarRole> {
    let mut roles = vec![StarRole::Literal; tokens.len()];
    let mut openers: Vec<(usize, usize)> = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        let InlineToken::Stars {
            range,
            can_open,
            can_close,
        } = token
        else {
            continue;
        };
        let matching = openers
            .iter()
            .rposition(|&(_, length)| length == range.len());
        if let (true, Some(position)) = (*can_close, matching) {
            roles[openers[position].0] = StarRole::Open;
            roles[index] = StarRole::Close;
            openers.truncate(position);
        } else if *can_open {
            openers.push((index, range.len()));
        }
    }
    roles
}

fn build_inlines(line: &str, tokens: &[InlineToken], roles: &[StarRole]) -> Vec<Inline> {
    let mut stack: Vec<Vec<Inline>> = vec![Vec::new()];
    for (token, role) in tokens.iter().zip(roles) {
        match (token, role) {
            (InlineToken::Stars { .. }, StarRole::Open) => stack.push(Vec::new()),
            (InlineToken::Stars { range, .. }, StarRole::Close) if stack.len() > 1 => {
                let children = stack.pop().unwrap_or_default();
                let node = if range.len() == 2 {
                    Inline::Strong(children)
                } else {
                    Inline::Emphasis(children)
                };
                push_inline(&mut stack, node);
            }
            _ => push_inline(&mut stack, leaf_inline(line, token)),
        }
    }
    stack.into_iter().next().unwrap_or_default()
}

fn leaf_inline(line: &str, token: &InlineToken) -> Inline {
    match token {
        InlineToken::Text(range) | InlineToken::Stars { range, .. } => {
            Inline::Text(line[range.clone()].to_string())
        }
        InlineToken::Escape(range) => Inline::Text(line[range.start + 1..range.end].to_string()),
        InlineToken::Code { content, .. } => Inline::Code(line[content.clone()].to_string()),
        InlineToken::Math { content, .. } => Inline::Math(parse_math(&line[content.clone()])),
        InlineToken::WikiLink { inner, .. } => wikilink(&line[inner.clone()]),
    }
}

fn wikilink(inner: &str) -> Inline {
    let (target, label) = match inner.split_once('|') {
        Some((target, label)) => (target, Some(label.trim()).filter(|label| !label.is_empty())),
        None => (inner, None),
    };
    Inline::WikiLink {
        target: target.trim().to_string(),
        label: label.map(str::to_string),
    }
}

fn push_inline(stack: &mut [Vec<Inline>], node: Inline) {
    let Some(top) = stack.last_mut() else { return };
    if let (Inline::Text(new), Some(Inline::Text(existing))) = (&node, top.last_mut()) {
        existing.push_str(new);
        return;
    }
    top.push(node);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Inline {
        Inline::Text(s.to_string())
    }

    #[test]
    fn unmatched_star_is_literal() {
        assert_eq!(parse_inlines("a * b"), vec![text("a * b")]);
        assert_eq!(parse_inlines("**open"), vec![text("**open")]);
    }

    #[test]
    fn nested_strong_and_emphasis() {
        assert_eq!(
            parse_inlines("*a* **b**"),
            vec![
                Inline::Emphasis(vec![text("a")]),
                text(" "),
                Inline::Strong(vec![text("b")])
            ]
        );
        assert_eq!(
            parse_inlines("**a *b* c**"),
            vec![Inline::Strong(vec![
                text("a "),
                Inline::Emphasis(vec![text("b")]),
                text(" c")
            ])]
        );
    }

    #[test]
    fn escapes_keep_punctuation_literally() {
        assert_eq!(parse_inlines("\\*a\\* \\;;"), vec![text("*a* ;;")]);
    }

    #[test]
    fn unclosed_code_is_text() {
        assert_eq!(parse_inlines("`a"), vec![text("`a")]);
    }

    #[test]
    fn double_backtick_code_can_contain_backticks() {
        assert_eq!(
            parse_inlines("``a`b``"),
            vec![Inline::Code("a`b".to_string())]
        );
    }
}
