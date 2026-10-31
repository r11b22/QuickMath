//! Paragraph flow, block layout and cache behaviour with real fonts.

use qm_format::{Atom, AtomClass, AtomStyle, Block, BlockKind, CalloutKind, Inline, MathNode};

use super::blocks::{BlockContext, BlockLayouter, Counters};
use super::cache::{LayoutCache, LayoutParams};
use super::flow::{layout_flow, Alignment, FlowBuilder, FlowOptions, InlineStyle};
use super::*;
use crate::measure::test_support::with_measure;
use crate::paint::{Primitive, Role};

const SIZE: f32 = 17.0;

fn paragraph(line: usize, content: Vec<Inline>) -> Block {
    Block {
        kind: BlockKind::Paragraph(content),
        line,
    }
}

fn text(s: &str) -> Inline {
    Inline::Text(s.into())
}

fn var(s: &str) -> MathNode {
    MathNode::Atom(Atom {
        text: s.into(),
        latex: s.into(),
        class: AtomClass::Ordinary,
        style: AtomStyle::Italic,
    })
}

fn flow(inlines: &[Inline], width: f32) -> flow::FlowLayout {
    with_measure(|m| {
        let mut builder = FlowBuilder::new(m, SIZE);
        builder.push_inlines(inlines, &InlineStyle::plain(Role::Text));
        layout_flow(
            &builder.finish(),
            FlowOptions {
                width,
                size: SIZE,
                alignment: Alignment::Justify,
            },
        )
    })
}

fn params(width: f32) -> LayoutParams {
    LayoutParams {
        width,
        font_size: SIZE,
        pixels_per_point: 1.0,
    }
}

#[test]
fn short_text_fits_on_one_line_and_long_text_wraps() {
    let words = "lorem ipsum dolor sit amet ".repeat(20);
    let short = flow(&[text("one line")], 600.0);
    let long = flow(&[text(&words)], 300.0);
    assert_eq!(short.line_count, 1);
    assert!(long.line_count > 3);
    assert!(long.height > short.height * 3.0);
}

#[test]
fn wrapped_words_stay_inside_the_width() {
    let long = flow(&[text(&"word ".repeat(60))], 250.0);
    for primitive in &long.drawing.primitives {
        if let Primitive::Text { pos, .. } = primitive {
            assert!(pos.x >= 0.0 && pos.x < 250.0, "word placed at {}", pos.x);
        }
    }
}

#[test]
fn inline_math_shares_the_text_baseline() {
    let laid_out = flow(
        &[text("let "), Inline::Math(vec![var("x")]), text(" be")],
        600.0,
    );
    assert_eq!(laid_out.line_count, 1);
    let tops: Vec<f32> = laid_out
        .drawing
        .primitives
        .iter()
        .filter_map(|p| {
            if let Primitive::Text { pos, .. } = p {
                Some(pos.y)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(tops.len(), 3);
    // Same font size for text and math, so galley tops line up within rounding.
    assert!((tops[0] - tops[2]).abs() < 0.01);
}

#[test]
fn forced_line_breaks_start_new_lines() {
    let laid_out = flow(&[text("a"), Inline::LineBreak, text("b")], 600.0);
    assert_eq!(laid_out.line_count, 2);
}

#[test]
fn empty_paragraph_still_has_height() {
    let laid_out = flow(&[], 600.0);
    assert_eq!(laid_out.line_count, 1);
    assert!(laid_out.height > SIZE);
}

#[test]
fn wikilinks_produce_clickable_hotspots() {
    let laid_out = flow(
        &[Inline::WikiLink {
            target: "Lecture 6".into(),
            label: Some("previous lecture".into()),
        }],
        600.0,
    );
    let targets: Vec<_> = laid_out
        .drawing
        .hotspots
        .iter()
        .map(|h| h.kind.clone())
        .collect();
    assert_eq!(targets.len(), 2, "one hotspot per word");
    assert!(targets
        .iter()
        .all(|k| *k == HotspotKind::WikiLink("Lecture 6".into())));
}

#[test]
fn callouts_share_one_counter_and_proofs_end_with_a_single_qed() {
    let callout = |kind: CalloutKind, line: usize, body: &str| Block {
        kind: BlockKind::Callout {
            kind,
            title: vec![],
            body: vec![paragraph(line + 1, vec![text(body)])],
        },
        line,
    };
    let definition = callout(CalloutKind::Definition, 0, "Term.");
    let theorem = callout(CalloutKind::Theorem, 2, "Statement.");
    let proof = callout(CalloutKind::Proof, 4, "Done.");
    let (theorem_layout, proof_layout) = with_measure(|m| {
        let mut layouter = BlockLayouter::new(m, SIZE);
        let mut context = BlockContext {
            width: 500.0,
            text_role: Role::Text,
            list_depth: 0,
            counters: Counters::default(),
        };
        layouter.block(&definition, 0, &mut context);
        (
            layouter.block(&theorem, 2, &mut context),
            layouter.block(&proof, 4, &mut context),
        )
    });
    assert_eq!(theorem_layout.counters_after.numbered, 2);
    let drawn = |layout: &blocks::BlockLayout| -> Vec<String> {
        layout
            .drawing
            .primitives
            .iter()
            .filter_map(|p| {
                if let Primitive::Text { text, .. } = p {
                    Some(text.clone())
                } else {
                    None
                }
            })
            .collect()
    };
    let words = drawn(&theorem_layout);
    assert!(
        words.windows(2).any(|w| w[0] == "Theorem" && w[1] == "2"),
        "{words:?}"
    );
    assert_eq!(drawn(&proof_layout).iter().filter(|t| *t == "∎").count(), 1);
}

#[test]
fn cache_reuses_unchanged_blocks_even_when_they_move() {
    let block = |line: usize, s: &str| paragraph(line, vec![text(s)]);
    let first = Document {
        blocks: vec![block(0, "alpha"), block(2, "beta")],
        ..Default::default()
    };
    let edited = Document {
        blocks: vec![block(0, "alpha"), block(1, "new"), block(3, "beta")],
        ..Default::default()
    };
    with_measure(|m| {
        let mut cache = LayoutCache::default();
        cache.layout_page(&first, params(500.0), m);
        assert_eq!(cache.misses, 2);
        cache.layout_page(&first, params(500.0), m);
        assert_eq!(cache.misses, 2, "an unchanged document is fully cached");
        let page = cache.layout_page(&edited, params(500.0), m);
        assert_eq!(cache.misses, 3, "only the inserted block is laid out");
        assert_eq!(page.blocks[2].line, 3);
        cache.layout_page(&edited, params(400.0), m);
        assert_eq!(cache.misses, 6, "a new width invalidates everything");
    });
}

#[test]
fn double_click_and_scroll_map_to_innermost_source_lines() {
    let item = |line: usize| vec![paragraph(line, vec![text("item")])];
    let list = Block {
        kind: BlockKind::List {
            ordered: false,
            items: vec![item(5), item(6)],
        },
        line: 5,
    };
    let document = Document {
        blocks: vec![paragraph(0, vec![text("intro")]), list],
        ..Default::default()
    };
    let page = with_measure(|m| LayoutCache::default().layout_page(&document, params(500.0), m));
    let list_block = &page.blocks[1];
    let second_item = list_block
        .layout
        .regions
        .iter()
        .find(|r| r.line_offset == 1)
        .expect("nested region")
        .rect;
    let inside_second = second_item.center() + vec2(0.0, list_block.top);
    assert_eq!(line_at(&page, inside_second), 6);
    assert_eq!(line_at(&page, pos2(1.0, 1.0)), 0);
    let target = rect_for_line(&page, 6).expect("rect for line 6");
    assert!(target.top() > list_block.top);
}

#[test]
fn page_geometry_caps_width_and_keeps_padding_on_narrow_screens() {
    let wide = PageGeometry::new(2000.0);
    assert_eq!(wide.page_width, MAX_PAGE_WIDTH);
    let narrow = PageGeometry::new(300.0);
    assert!(narrow.content_width() > 200.0 && narrow.content_width() < 300.0);
}

#[test]
fn show_document_runs_headless_and_reports_nothing_without_input() {
    let ctx = egui::Context::default();
    let mut fonts = egui::FontDefinitions::default();
    crate::install_fonts(&mut fonts);
    ctx.set_fonts(fonts);
    let document =
        qm_format::parse_document("---\ntitle: T\n---\n# Heading\n\nText with ;;x^2;;.\n");
    let mut response = None;
    for _ in 0..2 {
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            response = Some(crate::show_document(
                ui,
                &document,
                &Theme::light(),
                &PreviewOptions {
                    scroll_to_line: Some(3),
                },
            ));
            let _ = crate::show_math(ui, &[var("x")], true, &Theme::dark());
        });
    }
    assert_eq!(response, Some(PreviewResponse::default()));
}
