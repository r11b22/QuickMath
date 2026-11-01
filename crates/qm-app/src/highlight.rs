//! Syntax highlighting for the editor: turns `qm_format::highlight` spans
//! into an egui `LayoutJob`, cached because the layouter runs every frame.

use std::hash::{Hash, Hasher};
use std::ops::Range;

use egui::text::{LayoutJob, TextFormat};
use egui::{FontId, Stroke};
use qm_format::{HighlightSpan, TokenKind};
use qm_render::Theme;

/// Builds the coloured layout for `source`. Gaps between spans, or spans
/// that do not fall on character boundaries, are drawn as plain text so a
/// highlighter bug can never hide or panic on the user's text.
pub fn build_layout_job(
    source: &str,
    spans: &[HighlightSpan],
    theme: &Theme,
    font_id: &FontId,
) -> LayoutJob {
    let mut job = LayoutJob {
        keep_trailing_whitespace: true,
        ..LayoutJob::default()
    };
    let mut covered_until = 0;
    for span in spans {
        let range = span.range.start.max(covered_until)..span.range.end.min(source.len());
        if range.start >= range.end || source.get(range.clone()).is_none() {
            continue;
        }
        append_plain(&mut job, source, covered_until..range.start, theme, font_id);
        job.append(
            &source[range.clone()],
            0.0,
            format_for(span.kind, theme, font_id),
        );
        covered_until = range.end;
    }
    append_plain(
        &mut job,
        source,
        covered_until..source.len(),
        theme,
        font_id,
    );
    job
}

fn append_plain(
    job: &mut LayoutJob,
    source: &str,
    range: Range<usize>,
    theme: &Theme,
    font_id: &FontId,
) {
    if let Some(text) = source.get(range).filter(|text| !text.is_empty()) {
        job.append(text, 0.0, format_for(TokenKind::Text, theme, font_id));
    }
}

fn format_for(kind: TokenKind, theme: &Theme, font_id: &FontId) -> TextFormat {
    let syntax = &theme.syntax;
    let mut format = TextFormat::simple(font_id.clone(), theme.text);
    match kind {
        TokenKind::Text => {}
        TokenKind::Heading | TokenKind::Strong => format.color = syntax.heading,
        TokenKind::Emphasis => format.italics = true,
        TokenKind::Code => {
            format.color = syntax.code;
            format.background = theme.surface_raised;
        }
        TokenKind::Markup => format.color = syntax.markup,
        TokenKind::MathIdentifier => format.color = syntax.math,
        TokenKind::MathNumber => format.color = syntax.math_number,
        TokenKind::MathOperator => format.color = syntax.math_operator,
        TokenKind::MathKeyword => format.color = syntax.math_keyword,
        TokenKind::MathText => format.color = theme.text_muted,
        TokenKind::CalloutMarker => format.color = syntax.callout,
        TokenKind::WikiLink => {
            format.color = syntax.link;
            format.underline = Stroke::new(1.0, syntax.link);
        }
        TokenKind::FrontMatter => format.color = theme.text_faint,
        TokenKind::FrontMatterEmptyField => {
            format.color = theme.accent_text;
            format.background = theme.accent_soft;
        }
        TokenKind::Error => {
            format.color = theme.error;
            format.underline = Stroke::new(1.0, theme.error);
        }
    }
    format
}

/// Remembers the last highlighted layout so unchanged text is not re-highlighted.
#[derive(Default)]
pub struct HighlightCache {
    key: Option<u64>,
    job: LayoutJob,
}

impl HighlightCache {
    /// Returns the layout for `source`, recomputing only when the text,
    /// theme or font changed.
    pub fn layout_job(&mut self, source: &str, theme: &Theme, font_id: &FontId) -> LayoutJob {
        let key = cache_key(source, theme, font_id);
        if self.key != Some(key) {
            let spans = qm_format::highlight(source);
            self.job = build_layout_job(source, &spans, theme, font_id);
            self.key = Some(key);
        }
        self.job.clone()
    }
}

fn cache_key(source: &str, theme: &Theme, font_id: &FontId) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hasher);
    theme.is_dark.hash(&mut hasher);
    font_id.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(range: Range<usize>, kind: TokenKind) -> HighlightSpan {
        HighlightSpan { range, kind }
    }

    fn section_texts(job: &LayoutJob) -> Vec<&str> {
        job.sections
            .iter()
            .map(|section| &job.text[section.byte_range.start.0..section.byte_range.end.0])
            .collect()
    }

    #[test]
    fn spans_become_coloured_sections() {
        let theme = Theme::light();
        let source = "# Hi ;;x;;";
        let spans = [
            span(0..5, TokenKind::Heading),
            span(5..7, TokenKind::Markup),
            span(7..8, TokenKind::MathIdentifier),
            span(8..10, TokenKind::Markup),
        ];
        let job = build_layout_job(source, &spans, &theme, &FontId::monospace(14.0));
        assert_eq!(job.text, source);
        assert_eq!(section_texts(&job), ["# Hi ", ";;", "x", ";;"]);
        assert_eq!(job.sections[2].format.color, theme.syntax.math);
    }

    #[test]
    fn gaps_and_missing_tail_are_plain_text() {
        let theme = Theme::dark();
        let job = build_layout_job(
            "ab cd",
            &[span(1..2, TokenKind::Code)],
            &theme,
            &FontId::monospace(14.0),
        );
        assert_eq!(job.text, "ab cd");
        assert_eq!(section_texts(&job), ["a", "b", " cd"]);
        assert_eq!(job.sections[2].format.color, theme.text);
    }

    #[test]
    fn invalid_spans_never_drop_text() {
        let theme = Theme::light();
        let source = "αβ";
        // 1..3 splits the 2-byte `α`; 0..99 runs past the end.
        let spans = [
            span(1..3, TokenKind::Heading),
            span(0..99, TokenKind::Markup),
        ];
        let job = build_layout_job(source, &spans, &theme, &FontId::monospace(14.0));
        assert_eq!(job.text, source);
    }

    #[test]
    fn cache_reuses_layout_for_unchanged_text() {
        let theme = Theme::light();
        let font = FontId::monospace(14.0);
        let mut cache = HighlightCache::default();
        let first = cache.layout_job("text", &theme, &font);
        let second = cache.layout_job("text", &theme, &font);
        assert_eq!(first, second);
        assert_eq!(cache.layout_job("other", &theme, &font).text, "other");
    }
}
