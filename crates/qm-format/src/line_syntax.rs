//! Recognises what kind of block a line starts. Shared by the block parser
//! and the highlighter so both always agree.

use crate::ast::CalloutKind;

/// Delimits inline math (`;;x^2;;`) and, at the start of a line, marks a
/// display-math row.
pub(crate) const MATH_DELIMITER: &str = ";;";
/// Starts a callout header such as `.def Group`.
const CALLOUT_PREFIX: char = '.';
const CODE_FENCE: &str = "```";
const MAX_HEADING_LEVEL: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineStart<'a> {
    CodeFence {
        language: &'a str,
    },
    /// `content` excludes the leading `;;` and an optional trailing `;;`.
    DisplayMath {
        content: &'a str,
    },
    Heading {
        level: u8,
        content: &'a str,
    },
    Rule,
    Callout(CalloutHeader<'a>),
    Quote,
    ListItem(ListMarker),
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CalloutHeader<'a> {
    pub kind: CalloutKind,
    /// Bytes of `.keyword` plus an optional `:`.
    pub marker_len: usize,
    pub title: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ListMarker {
    pub is_ordered: bool,
    /// Bytes from the marker to the item content (marker plus spaces).
    pub content_offset: usize,
}

/// Classifies a line whose leading whitespace has already been removed.
pub(crate) fn classify(text: &str) -> LineStart<'_> {
    if let Some(language) = code_fence_language(text) {
        return LineStart::CodeFence { language };
    }
    if let Some(content) = display_math_content(text) {
        return LineStart::DisplayMath { content };
    }
    if let Some((level, content)) = heading(text) {
        return LineStart::Heading { level, content };
    }
    if is_rule(text) {
        return LineStart::Rule;
    }
    if let Some(header) = callout_header(text) {
        return LineStart::Callout(header);
    }
    if text.starts_with('>') {
        return LineStart::Quote;
    }
    match list_marker(text) {
        Some(marker) => LineStart::ListItem(marker),
        None => LineStart::Text,
    }
}

/// The info string of an opening fence. A backtick in it means the line is
/// inline code instead (```` ```x``` ````).
fn code_fence_language(text: &str) -> Option<&str> {
    let rest = text.strip_prefix(CODE_FENCE)?.trim_start_matches('`');
    (!rest.contains('`')).then(|| rest.trim())
}

pub(crate) fn is_code_fence_close(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.starts_with(CODE_FENCE) && trimmed.trim_start_matches('`').is_empty()
}

/// A line starting with `;;` is display math unless it is a paragraph that
/// merely begins with inline math (`;;x;; is positive`).
fn display_math_content(text: &str) -> Option<&str> {
    let rest = text.strip_prefix(MATH_DELIMITER)?;
    let content = rest.trim_end().strip_suffix(MATH_DELIMITER).unwrap_or(rest);
    (!content.contains(MATH_DELIMITER)).then_some(content)
}

fn heading(text: &str) -> Option<(u8, &str)> {
    let level = text.bytes().take_while(|&b| b == b'#').count();
    let rest = &text[level..];
    let is_heading = (1..=MAX_HEADING_LEVEL).contains(&level)
        && (rest.is_empty() || rest.starts_with(char::is_whitespace));
    is_heading.then(|| (level as u8, rest.trim()))
}

fn is_rule(text: &str) -> bool {
    let trimmed = text.trim_end();
    let Some(first) = trimmed.chars().next() else {
        return false;
    };
    trimmed.len() >= 3 && matches!(first, '-' | '*' | '_') && trimmed.chars().all(|c| c == first)
}

pub(crate) fn callout_header(text: &str) -> Option<CalloutHeader<'_>> {
    let rest = text.strip_prefix(CALLOUT_PREFIX)?;
    let keyword_len = rest.bytes().take_while(u8::is_ascii_alphabetic).count();
    let kind = callout_kind(&rest[..keyword_len])?;
    let after = &rest[keyword_len..];
    let (colon_len, title) = match after.strip_prefix(':') {
        Some(title) => (1, title),
        None if after.is_empty() || after.starts_with(char::is_whitespace) => (0, after),
        None => return None,
    };
    let marker_len = CALLOUT_PREFIX.len_utf8() + keyword_len + colon_len;
    Some(CalloutHeader {
        kind,
        marker_len,
        title: title.trim(),
    })
}

fn callout_kind(keyword: &str) -> Option<CalloutKind> {
    const KEYWORDS: &[(&str, CalloutKind)] = &[
        ("def", CalloutKind::Definition),
        ("defn", CalloutKind::Definition),
        ("definition", CalloutKind::Definition),
        ("thm", CalloutKind::Theorem),
        ("theorem", CalloutKind::Theorem),
        ("lem", CalloutKind::Lemma),
        ("lemma", CalloutKind::Lemma),
        ("cor", CalloutKind::Corollary),
        ("corollary", CalloutKind::Corollary),
        ("proof", CalloutKind::Proof),
        ("pf", CalloutKind::Proof),
        ("ex", CalloutKind::Example),
        ("eg", CalloutKind::Example),
        ("example", CalloutKind::Example),
        ("note", CalloutKind::Note),
        ("rem", CalloutKind::Note),
        ("remark", CalloutKind::Note),
        ("warn", CalloutKind::Warning),
        ("warning", CalloutKind::Warning),
        ("caution", CalloutKind::Warning),
    ];
    KEYWORDS
        .iter()
        .find(|(candidate, _)| candidate.eq_ignore_ascii_case(keyword))
        .map(|(_, kind)| *kind)
}

pub(crate) fn list_marker(text: &str) -> Option<ListMarker> {
    let digits = text.bytes().take_while(u8::is_ascii_digit).count();
    let (marker_len, is_ordered) = match text.as_bytes().first()? {
        b'-' | b'*' => (1, false),
        b'0'..=b'9' if digits <= 9 && text.as_bytes().get(digits) == Some(&b'.') => {
            (digits + 1, true)
        }
        _ => return None,
    };
    let rest = &text[marker_len..];
    if !rest.is_empty() && !rest.starts_with(' ') && !rest.starts_with('\t') {
        return None;
    }
    let spaces = rest.len() - rest.trim_start_matches([' ', '\t']).len();
    Some(ListMarker {
        is_ordered,
        content_offset: marker_len + spaces,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_math_line_with_inline_math_later_is_text() {
        assert_eq!(classify(";;x;; is positive"), LineStart::Text);
        assert_eq!(
            classify(";; E = mc^2 ;;"),
            LineStart::DisplayMath {
                content: " E = mc^2 "
            }
        );
    }

    #[test]
    fn heading_needs_space_after_hashes() {
        assert_eq!(classify("#tag"), LineStart::Text);
        assert_eq!(classify("####### seven"), LineStart::Text);
        assert_eq!(
            classify("## Two"),
            LineStart::Heading {
                level: 2,
                content: "Two"
            }
        );
    }

    #[test]
    fn callout_needs_known_keyword_followed_by_space_colon_or_end() {
        assert!(matches!(classify(".def Group"), LineStart::Callout(h) if h.title == "Group"));
        assert!(
            matches!(classify(".THM: Big"), LineStart::Callout(h) if h.title == "Big" && h.marker_len == 5)
        );
        assert!(matches!(classify(".proof"), LineStart::Callout(h) if h.title.is_empty()));
        assert_eq!(classify(".defx"), LineStart::Text);
        assert_eq!(classify(".5 is half"), LineStart::Text);
    }

    #[test]
    fn list_markers() {
        assert_eq!(
            list_marker("- a"),
            Some(ListMarker {
                is_ordered: false,
                content_offset: 2
            })
        );
        assert_eq!(
            list_marker("12.  a"),
            Some(ListMarker {
                is_ordered: true,
                content_offset: 5
            })
        );
        assert_eq!(
            list_marker("-"),
            Some(ListMarker {
                is_ordered: false,
                content_offset: 1
            })
        );
        assert_eq!(list_marker("-x"), None);
        assert_eq!(list_marker("3.14"), None);
    }

    #[test]
    fn rules_and_fences() {
        assert_eq!(classify("---"), LineStart::Rule);
        assert_eq!(
            classify("```rust"),
            LineStart::CodeFence { language: "rust" }
        );
        assert_eq!(classify("```x```"), LineStart::Text);
        assert!(is_code_fence_close("  ```  "));
    }
}
