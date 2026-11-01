//! Syntax highlighting: coverage invariant and token categories.

use qm_format::{highlight, HighlightSpan, TokenKind};

fn assert_covers(source: &str, spans: &[HighlightSpan]) {
    let mut expected_start = 0;
    for span in spans {
        assert_eq!(
            span.range.start, expected_start,
            "gap or overlap in {source:?}: {spans:?}"
        );
        assert!(
            span.range.start < span.range.end,
            "empty span in {source:?}"
        );
        assert!(
            source.is_char_boundary(span.range.start) && source.is_char_boundary(span.range.end)
        );
        expected_start = span.range.end;
    }
    assert_eq!(
        expected_start,
        source.len(),
        "spans do not reach the end of {source:?}"
    );
}

fn kind_of(source: &str, needle: &str) -> TokenKind {
    let position = source.find(needle).expect("needle in source");
    let spans = highlight(source);
    spans
        .iter()
        .find(|span| span.range.contains(&position))
        .map(|span| span.kind)
        .expect("covered")
}

#[test]
fn empty_source_has_no_spans() {
    assert!(highlight("").is_empty());
}

#[test]
fn covers_every_byte_of_a_full_note() {
    let source = include_str!("../../../docs/examples/lecture.qm");
    assert_covers(source, &highlight(source));
}

#[test]
fn covers_assorted_snippets() {
    let snippets = [
        "plain",
        "# Heading with ;;x;;\n",
        "---\ntitle: T\n---\nbody",
        "- **bold** and *em* `code`\n> quote [[link]]",
        ";; a &= b\r\n;; = c ;;",
        ".def Group ;;G;;\n  body",
        "```\ncode\n```",
        "αβγ ;;α≤β;; ünïcödé\n\n\t\n",
        ";;\"unclosed",
        "\\;; ;;\\",
    ];
    for snippet in snippets {
        assert_covers(snippet, &highlight(snippet));
    }
}

#[test]
fn categories() {
    let source = "# Title\n.thm Name\n- item ;;sum x1 + 2;; [[Link]] `c` **s**\n;; y = \"t\"";
    assert_eq!(kind_of(source, "Title"), TokenKind::Heading);
    assert_eq!(kind_of(source, ".thm"), TokenKind::CalloutMarker);
    assert_eq!(kind_of(source, "- "), TokenKind::Markup);
    assert_eq!(kind_of(source, ";;sum"), TokenKind::Markup);
    assert_eq!(kind_of(source, "sum"), TokenKind::MathKeyword);
    assert_eq!(kind_of(source, "x1"), TokenKind::MathIdentifier);
    assert_eq!(kind_of(source, "2;;"), TokenKind::MathNumber);
    assert_eq!(kind_of(source, "+ 2"), TokenKind::MathOperator);
    assert_eq!(kind_of(source, "[[Link]]"), TokenKind::WikiLink);
    assert_eq!(kind_of(source, "`c`"), TokenKind::Code);
    assert_eq!(kind_of(source, "s**"), TokenKind::Strong);
    assert_eq!(kind_of(source, "\"t\""), TokenKind::MathText);
}

#[test]
fn front_matter_and_code_regions() {
    let source = "---\ntitle: T\n---\n```\n# not a heading\n```";
    assert_eq!(kind_of(source, "title"), TokenKind::FrontMatter);
    assert_eq!(kind_of(source, "# not"), TokenKind::Code);
}

#[test]
fn empty_front_matter_fields_stand_out_until_filled_in() {
    let source = "---\ntitle: T\nauthor:\n---\nbody";
    assert_eq!(kind_of(source, "title"), TokenKind::FrontMatter);
    assert_eq!(kind_of(source, "author:"), TokenKind::FrontMatterEmptyField);

    let filled = "---\ntitle: T\nauthor: Ada\n---\nbody";
    assert_eq!(kind_of(filled, "author"), TokenKind::FrontMatter);
}

#[test]
fn new_note_template_highlights_only_its_empty_fields() {
    let source = qm_format::new_note_template("Groups", "2026-11-01");
    assert_eq!(kind_of(&source, "title"), TokenKind::FrontMatter);
    assert_eq!(kind_of(&source, "date"), TokenKind::FrontMatter);
    for field in ["author:", "course:", "tags:"] {
        assert_eq!(kind_of(&source, field), TokenKind::FrontMatterEmptyField);
    }
}

#[test]
fn unknown_backslash_command_is_an_error() {
    assert_eq!(kind_of(";;\\foo;;", "\\foo"), TokenKind::Error);
}
