//! Editor syntax highlighting. Works line by line using the same line
//! classification and inline tokenizer as the parser.

use std::ops::Range;

use crate::front_matter::{front_matter_len, is_empty_field};
use crate::inline::highlight_inlines;
use crate::line_syntax::{classify, is_code_fence_close, list_marker, LineStart, MATH_DELIMITER};
use crate::lines::split_lines;
use crate::math::highlight_math;
use crate::{HighlightSpan, TokenKind};

/// Collects spans in order, filling gaps with [`TokenKind::Text`], clipping
/// overlaps and merging neighbours of the same kind, so the result always
/// covers the source contiguously.
#[derive(Debug, Default)]
pub(crate) struct SpanSink {
    spans: Vec<HighlightSpan>,
    cursor: usize,
}

impl SpanSink {
    pub(crate) fn push(&mut self, range: Range<usize>, kind: TokenKind) {
        let start = range.start.max(self.cursor);
        if start >= range.end {
            return;
        }
        if start > self.cursor {
            self.append(self.cursor..start, TokenKind::Text);
        }
        self.append(start..range.end, kind);
    }

    fn append(&mut self, range: Range<usize>, kind: TokenKind) {
        self.cursor = range.end;
        if let Some(last) = self.spans.last_mut() {
            if last.kind == kind && last.range.end == range.start {
                last.range.end = range.end;
                return;
            }
        }
        self.spans.push(HighlightSpan { range, kind });
    }

    pub(crate) fn finish(mut self, source_len: usize) -> Vec<HighlightSpan> {
        if self.cursor < source_len {
            self.append(self.cursor..source_len, TokenKind::Text);
        }
        self.spans
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Normal,
    InCode,
}

pub(crate) fn highlight(source: &str) -> Vec<HighlightSpan> {
    let lines = split_lines(source);
    let front_matter_lines = front_matter_len(&lines).unwrap_or(0);
    let mut sink = SpanSink::default();
    let mut state = State::Normal;
    let mut offset = 0;
    for (number, raw_line) in source.split('\n').enumerate() {
        let text = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if number < front_matter_lines {
            let kind = if is_empty_field(&lines, number) {
                TokenKind::FrontMatterEmptyField
            } else {
                TokenKind::FrontMatter
            };
            sink.push(offset..offset + text.len(), kind);
        } else {
            state = highlight_line(text, offset, state, &mut sink);
        }
        offset += raw_line.len() + 1;
    }
    sink.finish(source.len())
}

fn highlight_line(text: &str, offset: usize, state: State, sink: &mut SpanSink) -> State {
    if state == State::InCode {
        let kind = if is_code_fence_close(text) {
            TokenKind::Markup
        } else {
            TokenKind::Code
        };
        sink.push(offset..offset + text.len(), kind);
        return if kind == TokenKind::Markup {
            State::Normal
        } else {
            State::InCode
        };
    }
    let cursor = highlight_container_markers(text, offset, sink);
    highlight_leaf(&text[cursor..], offset + cursor, sink)
}

/// Marks leading `>` and list markers; returns where the line content starts.
fn highlight_container_markers(text: &str, offset: usize, sink: &mut SpanSink) -> usize {
    let mut cursor = leading_whitespace(text);
    loop {
        let rest = &text[cursor..];
        let marker_len = if rest.starts_with('>') {
            1
        } else if let Some(marker) = list_marker(rest) {
            marker.content_offset
        } else {
            return cursor;
        };
        sink.push(
            offset + cursor..offset + cursor + marker_len,
            TokenKind::Markup,
        );
        cursor += marker_len;
        cursor += leading_whitespace(&text[cursor..]);
    }
}

fn highlight_leaf(text: &str, offset: usize, sink: &mut SpanSink) -> State {
    let end = offset + text.len();
    match classify(text) {
        LineStart::CodeFence { .. } => {
            sink.push(offset..end, TokenKind::Markup);
            return State::InCode;
        }
        LineStart::DisplayMath { content } => {
            let content_start = offset + MATH_DELIMITER.len();
            sink.push(offset..content_start, TokenKind::Markup);
            highlight_math(content, content_start, sink);
            sink.push(content_start + content.len()..end, TokenKind::Markup);
        }
        LineStart::Heading { level, content } => {
            sink.push(offset..offset + level as usize, TokenKind::Markup);
            highlight_inlines(
                content,
                offset + subslice_offset(text, content),
                TokenKind::Heading,
                sink,
            );
        }
        LineStart::Rule => sink.push(offset..end, TokenKind::Markup),
        LineStart::Callout(header) => {
            sink.push(offset..offset + header.marker_len, TokenKind::CalloutMarker);
            highlight_inlines(
                header.title,
                offset + subslice_offset(text, header.title),
                TokenKind::Text,
                sink,
            );
        }
        LineStart::Quote | LineStart::ListItem(_) | LineStart::Text => {
            highlight_inlines(text, offset, TokenKind::Text, sink);
        }
    }
    State::Normal
}

fn leading_whitespace(text: &str) -> usize {
    text.len() - text.trim_start().len()
}

/// Byte position of `part` within `whole`; `part` must be a subslice of it.
fn subslice_offset(whole: &str, part: &str) -> usize {
    (part.as_ptr() as usize)
        .saturating_sub(whole.as_ptr() as usize)
        .min(whole.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sink_fills_gaps_and_merges() {
        let mut sink = SpanSink::default();
        sink.push(2..3, TokenKind::Markup);
        sink.push(3..4, TokenKind::Markup);
        sink.push(1..5, TokenKind::Code);
        let spans = sink.finish(6);
        let ranges: Vec<_> = spans.iter().map(|s| (s.range.clone(), s.kind)).collect();
        assert_eq!(
            ranges,
            vec![
                (0..2, TokenKind::Text),
                (2..4, TokenKind::Markup),
                (4..5, TokenKind::Code),
                (5..6, TokenKind::Text)
            ]
        );
    }
}
