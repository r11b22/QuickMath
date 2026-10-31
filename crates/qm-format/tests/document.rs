//! Block structure, inline markup, front matter and line numbers.

use qm_format::{
    display_math_to_latex, math_to_latex, parse_document, Block, BlockKind, CalloutKind,
    DisplayMath, Inline,
};

fn blocks(source: &str) -> Vec<Block> {
    parse_document(source).blocks
}

fn kinds(source: &str) -> Vec<BlockKind> {
    blocks(source).into_iter().map(|block| block.kind).collect()
}

fn text(s: &str) -> Inline {
    Inline::Text(s.to_string())
}

fn paragraph_inlines(source: &str) -> Vec<Inline> {
    match kinds(source).remove(0) {
        BlockKind::Paragraph(content) => content,
        other => panic!("expected a paragraph, got {other:?}"),
    }
}

fn display(source: &str) -> DisplayMath {
    match kinds(source).remove(0) {
        BlockKind::DisplayMath(math) => math,
        other => panic!("expected display math, got {other:?}"),
    }
}

// ----- basics --------------------------------------------------------------

#[test]
fn empty_input_has_no_blocks() {
    assert_eq!(parse_document(""), qm_format::Document::default());
    assert!(blocks("\n\n  \n").is_empty());
}

#[test]
fn headings_by_level() {
    assert_eq!(
        kinds("# One\n###### Six"),
        vec![
            BlockKind::Heading {
                level: 1,
                content: vec![text("One")]
            },
            BlockKind::Heading {
                level: 6,
                content: vec![text("Six")]
            },
        ]
    );
}

#[test]
fn paragraph_keeps_line_breaks_and_ends_at_blank_line() {
    assert_eq!(
        kinds("a\nb\n\nc"),
        vec![
            BlockKind::Paragraph(vec![text("a"), Inline::LineBreak, text("b")]),
            BlockKind::Paragraph(vec![text("c")]),
        ]
    );
}

#[test]
fn rule_and_code_fence() {
    assert_eq!(
        kinds("---\n```rust\nfn main() {}\n  x\n```\nafter"),
        vec![
            BlockKind::Rule,
            BlockKind::Code {
                language: Some("rust".to_string()),
                code: "fn main() {}\n  x".to_string()
            },
            BlockKind::Paragraph(vec![text("after")]),
        ]
    );
}

#[test]
fn unclosed_code_fence_runs_to_end() {
    assert_eq!(
        kinds("```\ncode"),
        vec![BlockKind::Code {
            language: None,
            code: "code".to_string()
        }]
    );
}

#[test]
fn quote_contains_blocks() {
    assert_eq!(
        kinds("> # Q\n> text"),
        vec![BlockKind::Quote(vec![
            Block {
                kind: BlockKind::Heading {
                    level: 1,
                    content: vec![text("Q")]
                },
                line: 0
            },
            Block {
                kind: BlockKind::Paragraph(vec![text("text")]),
                line: 1
            },
        ])]
    );
}

// ----- inline --------------------------------------------------------------

#[test]
fn inline_markup() {
    assert_eq!(
        paragraph_inlines("*a* **b** `c` [[Note]] [[Note#H|label]]"),
        vec![
            Inline::Emphasis(vec![text("a")]),
            text(" "),
            Inline::Strong(vec![text("b")]),
            text(" "),
            Inline::Code("c".to_string()),
            text(" "),
            Inline::WikiLink {
                target: "Note".to_string(),
                label: None
            },
            text(" "),
            Inline::WikiLink {
                target: "Note#H".to_string(),
                label: Some("label".to_string())
            },
        ]
    );
}

#[test]
fn inline_math_between_double_semicolons() {
    let content = paragraph_inlines("so ;;x^2;; holds");
    assert_eq!(content.len(), 3);
    let Inline::Math(math) = &content[1] else {
        panic!()
    };
    assert_eq!(math_to_latex(math), "x^{2}");
}

#[test]
fn unclosed_inline_math_extends_to_end_of_line() {
    let content = paragraph_inlines("let ;;a/b");
    let Inline::Math(math) = &content[1] else {
        panic!()
    };
    assert_eq!(math_to_latex(math), "\\frac{a}{b}");
}

#[test]
fn dollar_signs_are_plain_text() {
    assert_eq!(paragraph_inlines("costs $5"), vec![text("costs $5")]);
}

#[test]
fn escaped_double_semicolon_is_literal() {
    assert_eq!(paragraph_inlines("a\\;; b"), vec![text("a;; b")]);
}

#[test]
fn empty_wikilink_is_text() {
    assert_eq!(paragraph_inlines("[[ ]]"), vec![text("[[ ]]")]);
}

// ----- display math --------------------------------------------------------

#[test]
fn single_display_line() {
    let math = display(";; E = mc^2");
    assert_eq!(math.rows.len(), 1);
    assert_eq!(display_math_to_latex(&math), "E=mc^{2}");
}

#[test]
fn consecutive_display_lines_align_at_first_relation() {
    let math = display(";; (a+b)^2 = (a+b)(a+b)\n;; = a^2 + 2a b + b^2");
    assert_eq!(math.rows.len(), 2);
    assert!(math.rows[1].cells[0].is_empty());
    assert_eq!(
        display_math_to_latex(&math),
        "\\begin{aligned}\\left(a+b\\right)^{2} & =\\left(a+b\\right)\\left(a+b\\right) \\\\  & =a^{2}+2ab+b^{2}\\end{aligned}"
    );
}

#[test]
fn display_line_with_trailing_delimiter() {
    assert_eq!(display_math_to_latex(&display(";;x+1;;")), "x+1");
}

#[test]
fn line_starting_with_inline_math_is_a_paragraph() {
    assert!(matches!(
        kinds(";;x;; is positive").remove(0),
        BlockKind::Paragraph(_)
    ));
}

#[test]
fn empty_display_marker_has_no_rows() {
    assert_eq!(display(";;").rows.len(), 0);
}

// ----- lists ---------------------------------------------------------------

#[test]
fn flat_and_nested_lists() {
    let parsed = kinds("- a\n  - b\n- c\n\n1. one\n2. two");
    let BlockKind::List {
        ordered: false,
        items,
    } = &parsed[0]
    else {
        panic!("{parsed:?}")
    };
    assert_eq!(items.len(), 2);
    assert!(matches!(
        items[0][1].kind,
        BlockKind::List { ordered: false, .. }
    ));
    assert_eq!(items[0][1].line, 1);
    let BlockKind::List {
        ordered: true,
        items,
    } = &parsed[1]
    else {
        panic!("{parsed:?}")
    };
    assert_eq!(items.len(), 2);
}

#[test]
fn ordered_item_nests_under_content_column() {
    let parsed = kinds("1. a\n   - b\n   - c");
    let BlockKind::List { items, .. } = &parsed[0] else {
        panic!()
    };
    let BlockKind::List { items: nested, .. } = &items[0][1].kind else {
        panic!("{:?}", items[0])
    };
    assert_eq!(nested.len(), 2);
}

#[test]
fn list_items_can_hold_display_math_and_continue_after_blank_line() {
    let parsed = kinds("- a\n  ;; x = 1\n\n  more\n- b");
    let BlockKind::List { items, .. } = &parsed[0] else {
        panic!()
    };
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].len(), 3);
    assert!(matches!(items[0][1].kind, BlockKind::DisplayMath(_)));
    assert_eq!(items[0][2].line, 3);
}

#[test]
fn unindented_line_ends_list() {
    let parsed = kinds("- a\nb");
    assert_eq!(parsed.len(), 2);
}

#[test]
fn switching_list_kind_starts_new_list() {
    assert_eq!(kinds("- a\n1. b").len(), 2);
}

#[test]
fn empty_list_item_while_typing() {
    assert_eq!(
        kinds("-"),
        vec![BlockKind::List {
            ordered: false,
            items: vec![vec![]]
        }]
    );
}

// ----- callouts ------------------------------------------------------------

fn callout(source: &str) -> (CalloutKind, Vec<Inline>, Vec<Block>) {
    match kinds(source).remove(0) {
        BlockKind::Callout { kind, title, body } => (kind, title, body),
        other => panic!("expected a callout, got {other:?}"),
    }
}

#[test]
fn callout_keywords() {
    let expected = [
        (".def", CalloutKind::Definition),
        (".thm", CalloutKind::Theorem),
        (".lem", CalloutKind::Lemma),
        (".cor", CalloutKind::Corollary),
        (".proof", CalloutKind::Proof),
        (".ex", CalloutKind::Example),
        (".note", CalloutKind::Note),
        (".warn", CalloutKind::Warning),
        (".Theorem", CalloutKind::Theorem),
    ];
    for (source, kind) in expected {
        assert_eq!(callout(source).0, kind, "{source}");
    }
}

#[test]
fn callout_title_may_contain_math_or_be_empty() {
    let (_, title, _) = callout(".thm Pythagoras ;;a^2+b^2=c^2;;");
    assert_eq!(title.len(), 2);
    assert!(callout(".proof").1.is_empty());
}

#[test]
fn callout_body_is_the_lines_directly_below() {
    let (_, _, body) = callout(".def Group\nA set with\n;; a * b in G\n\nafter");
    assert_eq!(body.len(), 2);
    assert_eq!(body[0].line, 1);
    assert_eq!(body[1].line, 2);
    assert_eq!(
        kinds(".def Group\nA set with\n;; a * b in G\n\nafter").len(),
        2
    );
}

#[test]
fn callout_body_continues_after_blank_line_when_indented() {
    let source = ".proof\nFirst.\n\n  Second paragraph.\n\n  - a list\nOutside";
    let (_, _, body) = callout(source);
    assert_eq!(body.len(), 3);
    assert_eq!(body[1].line, 3);
    assert_eq!(body[2].line, 5);
    assert_eq!(
        kinds(source).len(),
        2,
        "after a blank line only indented lines continue the body"
    );
}

#[test]
fn heading_or_new_callout_ends_callout() {
    assert_eq!(kinds(".def A\nbody\n.thm B\nbody\n# H").len(), 3);
}

#[test]
fn unknown_dot_word_is_a_paragraph() {
    assert!(matches!(
        kinds(".net rocks").remove(0),
        BlockKind::Paragraph(_)
    ));
}

// ----- front matter and line numbers ---------------------------------------

#[test]
fn front_matter_is_parsed_and_skipped() {
    let document = parse_document("---\ntitle: Lecture 1\ntags: [a, b]\n---\n# Intro");
    assert_eq!(document.front_matter.title.as_deref(), Some("Lecture 1"));
    assert_eq!(document.front_matter.tags, vec!["a", "b"]);
    assert_eq!(document.blocks[0].line, 4);
}

#[test]
fn block_lines_are_zero_based_source_lines() {
    let source = "# A\n\npara\n\n;; x\n;; y\n\n- item\n\n.note N\nbody\n\n```\ncode\n```\n> q\n---";
    let lines: Vec<usize> = blocks(source).iter().map(|block| block.line).collect();
    assert_eq!(lines, vec![0, 2, 4, 7, 9, 12, 15, 16]);
}

#[test]
fn crlf_line_endings() {
    assert_eq!(kinds("# A\r\ntext\r\n").len(), 2);
}

fn callout_body(source: &str) -> Vec<Block> {
    match kinds(source).remove(0) {
        BlockKind::Callout { body, .. } => body,
        other => panic!("expected a callout, got {other:?}"),
    }
}

#[test]
fn proof_drops_hand_written_trailing_qed() {
    let body = callout_body(".proof\nHence it holds. ;;qed;;");

    assert_eq!(
        body[0].kind,
        BlockKind::Paragraph(vec![Inline::Text("Hence it holds.".into())])
    );
}

#[test]
fn qed_outside_a_proof_is_kept() {
    let body = callout_body(".note\nDone ;;qed;;");

    let BlockKind::Paragraph(content) = &body[0].kind else {
        panic!("expected a paragraph");
    };
    assert!(matches!(content.last(), Some(Inline::Math(_))));
}
