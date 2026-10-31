//! Layout metrics checked against real fonts in a headless egui context.

use qm_format::{
    AccentKind, Atom, AtomClass, AtomStyle, DisplayMath, MathList, MathNode, MathRow, SpaceWidth,
};

use super::*;
use crate::measure::test_support::with_measure;
use crate::paint::{HotspotKind, Primitive};

const SIZE: f32 = 20.0;

fn atom(text: &str, class: AtomClass, style: AtomStyle) -> MathNode {
    MathNode::Atom(Atom {
        text: text.into(),
        latex: text.into(),
        class,
        style,
    })
}
fn var(text: &str) -> MathNode {
    atom(text, AtomClass::Ordinary, AtomStyle::Italic)
}
fn num(text: &str) -> MathNode {
    atom(text, AtomClass::Ordinary, AtomStyle::Upright)
}
fn bin(text: &str) -> MathNode {
    atom(text, AtomClass::Binary, AtomStyle::Upright)
}
fn rel(text: &str) -> MathNode {
    atom(text, AtomClass::Relation, AtomStyle::Upright)
}
fn frac(numerator: MathList, denominator: MathList) -> MathNode {
    MathNode::Fraction {
        numerator,
        denominator,
    }
}
fn scripts(base: MathNode, sub: Option<MathList>, sup: Option<MathList>) -> MathNode {
    MathNode::Scripts {
        base: Box::new(base),
        sub,
        sup,
    }
}
fn fenced(open: &str, close: &str, body: MathList) -> MathNode {
    MathNode::Fenced {
        open: open.into(),
        close: close.into(),
        body,
    }
}

fn layout(math: &[MathNode], style: MathStyle) -> MathBox {
    with_measure(|m| MathLayout::new(m, SIZE).list(math, style))
}

fn texts(math_box: &MathBox) -> Vec<(String, f32)> {
    math_box
        .drawing
        .primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::Text { text, font, .. } => Some((text.clone(), font.size)),
            _ => None,
        })
        .collect()
}

#[test]
fn fraction_is_taller_than_its_parts_and_centred_on_axis() {
    let x = layout(&[var("x")], MathStyle::TEXT);
    let fraction = layout(&[frac(vec![var("x")], vec![var("y")])], MathStyle::DISPLAY);
    assert!(fraction.height() > 2.0 * x.height(), "{fraction:?}");
    assert!(fraction.ascent > x.ascent && fraction.descent > x.descent);
    let rule = fraction.drawing.primitives.iter().find_map(|p| match p {
        Primitive::Rect { rect, .. } => Some(*rect),
        _ => None,
    });
    let rule = rule.expect("fraction draws a rule");
    assert!(
        (rule.center().y + 0.25 * SIZE).abs() < 0.5,
        "rule centre {} not on the axis",
        rule.center().y
    );
}

#[test]
fn text_style_fractions_use_smaller_type_than_display_ones() {
    let parts = [frac(vec![var("a")], vec![var("b")])];
    let display = layout(&parts, MathStyle::DISPLAY);
    let inline = layout(&parts, MathStyle::TEXT);
    assert!(texts(&display).iter().all(|(_, size)| *size == SIZE));
    assert!(texts(&inline).iter().all(|(_, size)| *size < SIZE));
    assert!(inline.height() < display.height());
}

#[test]
fn scripts_are_smaller_and_nested_scripts_smaller_still() {
    let power = layout(
        &[scripts(
            var("x"),
            None,
            Some(vec![scripts(var("y"), None, Some(vec![var("z")]))]),
        )],
        MathStyle::TEXT,
    );
    let sizes: Vec<f32> = texts(&power).into_iter().map(|(_, s)| s).collect();
    assert_eq!(sizes, vec![SIZE, SIZE * 0.7, SIZE * 0.5]);
}

#[test]
fn superscript_raises_and_subscript_lowers() {
    let x = layout(&[var("x")], MathStyle::TEXT);
    let sup = layout(
        &[scripts(var("x"), None, Some(vec![num("2")]))],
        MathStyle::TEXT,
    );
    let sub = layout(
        &[scripts(var("x"), Some(vec![var("i")]), None)],
        MathStyle::TEXT,
    );
    assert!(sup.ascent > x.ascent + 0.2 * SIZE);
    assert!(sub.descent > 0.15 * SIZE);
    assert!(sup.width > x.width && sub.width > x.width);
}

#[test]
fn both_scripts_keep_a_minimum_gap() {
    let both = layout(
        &[scripts(
            var("x"),
            Some(vec![var("k")]),
            Some(vec![var("k")]),
        )],
        MathStyle::TEXT,
    );
    let sup_only = layout(
        &[scripts(var("x"), None, Some(vec![var("k")]))],
        MathStyle::TEXT,
    );
    assert!(both.ascent >= sup_only.ascent - 0.01);
    assert!(
        both.height() > 1.2 * SIZE * 0.7,
        "scripts overlap: {}",
        both.height()
    );
}

#[test]
fn display_limits_go_above_and_below_but_inline_beside() {
    let sum = atom(
        "∑",
        AtomClass::LargeOperator { limits: true },
        AtomStyle::Upright,
    );
    let math = [scripts(sum, Some(vec![var("k")]), Some(vec![var("n")]))];
    let display = layout(&math, MathStyle::DISPLAY);
    let inline = layout(&math, MathStyle::TEXT);
    assert!(display.height() > inline.height() + 0.5 * SIZE);
    assert!(display.width < inline.width + 0.5 * SIZE);
    let display_op = texts(&display)
        .into_iter()
        .find(|(t, _)| t == "∑")
        .expect("operator drawn");
    assert!(display_op.1 > SIZE, "display operator is enlarged");
}

#[test]
fn integrals_are_bigger_in_display_style() {
    let integral = [atom(
        "∫",
        AtomClass::LargeOperator { limits: false },
        AtomStyle::Upright,
    )];
    let display = layout(&integral, MathStyle::DISPLAY);
    let inline = layout(&integral, MathStyle::TEXT);
    assert!(display.height() > 1.5 * inline.height());
    // Centred on the math axis either way.
    let centre = (display.ascent - display.descent) / 2.0;
    assert!((centre - 0.25 * SIZE).abs() < 1.5, "centre {centre}");
}

#[test]
fn stretchy_delimiters_cover_their_body() {
    let tall = vec![frac(
        vec![frac(vec![var("a")], vec![var("b")])],
        vec![frac(vec![var("c")], vec![var("d")])],
    )];
    let body = layout(&tall, MathStyle::DISPLAY);
    for (open, close) in [
        ("(", ")"),
        ("[", "]"),
        ("{", "}"),
        ("|", "|"),
        ("‖", "‖"),
        ("⟨", "⟩"),
        ("⌊", "⌋"),
        ("⌈", "⌉"),
        ("/", "\\"),
    ] {
        let fenced_box = layout(&[fenced(open, close, tall.clone())], MathStyle::DISPLAY);
        assert!(
            fenced_box.height() >= body.height() * 0.9,
            "{open}{close}: {} < {}",
            fenced_box.height(),
            body.height()
        );
        assert!(fenced_box.width > body.width);
    }
}

#[test]
fn small_delimiters_use_font_glyphs() {
    let parens = layout(&[fenced("(", ")", vec![var("x")])], MathStyle::TEXT);
    let glyphs: Vec<String> = texts(&parens).into_iter().map(|(t, _)| t).collect();
    assert!(glyphs.contains(&"(".to_string()) && glyphs.contains(&")".to_string()));
}

#[test]
fn relations_get_more_space_than_binary_operators() {
    let plain = layout(&[var("a"), var("b")], MathStyle::TEXT).width;
    let with_bin = layout(&[var("a"), bin("+"), var("b")], MathStyle::TEXT).width;
    let with_rel = layout(&[var("a"), rel("="), var("b")], MathStyle::TEXT).width;
    let plus = layout(&[bin("+")], MathStyle::TEXT).width;
    let equals = layout(&[rel("=")], MathStyle::TEXT).width;
    let bin_space = with_bin - plain - plus;
    let rel_space = with_rel - plain - equals;
    assert!(
        (bin_space - 2.0 * 4.0 / 18.0 * SIZE).abs() < 0.5,
        "binary space {bin_space}"
    );
    assert!(
        (rel_space - 2.0 * 5.0 / 18.0 * SIZE).abs() < 0.5,
        "relation space {rel_space}"
    );
}

#[test]
fn leading_minus_is_unary() {
    let unary = layout(&[bin("−"), var("x")], MathStyle::TEXT).width;
    let parts =
        layout(&[bin("−")], MathStyle::TEXT).width + layout(&[var("x")], MathStyle::TEXT).width;
    assert!((unary - parts).abs() < 0.5);
}

#[test]
fn root_covers_radicand_and_index_sits_left() {
    let radicand = layout(&[var("x"), bin("+"), num("1")], MathStyle::TEXT);
    let root = layout(
        &[MathNode::Root {
            index: None,
            radicand: vec![var("x"), bin("+"), num("1")],
        }],
        MathStyle::TEXT,
    );
    assert!(root.ascent > radicand.ascent && root.width > radicand.width + 0.4 * SIZE);
    let indexed = layout(
        &[MathNode::Root {
            index: Some(vec![num("3")]),
            radicand: vec![var("x")],
        }],
        MathStyle::TEXT,
    );
    let three = texts(&indexed)
        .into_iter()
        .find(|(t, _)| t == "3")
        .expect("index drawn");
    assert_eq!(three.1, SIZE * 0.5);
}

#[test]
fn matrix_rows_do_not_overlap() {
    let cell = |n: &str, d: &str| vec![frac(vec![num(n)], vec![num(d)])];
    let rows = vec![
        vec![cell("1", "2")],
        vec![cell("3", "4")],
        vec![cell("5", "6")],
    ];
    let one = layout(&[frac(vec![num("1")], vec![num("2")])], MathStyle::TEXT);
    let matrix = layout(
        &[MathNode::Matrix {
            open: "(".into(),
            close: ")".into(),
            rows,
        }],
        MathStyle::DISPLAY,
    );
    assert!(
        matrix.height() >= 3.0 * one.height(),
        "{} < 3 × {}",
        matrix.height(),
        one.height()
    );
}

#[test]
fn cases_align_cells_left() {
    let rows = vec![
        vec![vec![var("x")], vec![num("1")]],
        vec![
            vec![var("x"), bin("+"), var("y"), bin("+"), var("z")],
            vec![num("2")],
        ],
    ];
    let cases = layout(
        &[MathNode::Matrix {
            open: "{".into(),
            close: "".into(),
            rows,
        }],
        MathStyle::DISPLAY,
    );
    let lefts: Vec<f32> = cases
        .drawing
        .primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::Text { pos, text, .. } if text == "𝑥" => Some(pos.x),
            _ => None,
        })
        .collect();
    assert_eq!(lefts.len(), 2);
    assert!(
        (lefts[0] - lefts[1]).abs() < 0.01,
        "first column not left-aligned: {lefts:?}"
    );
}

#[test]
fn accents_sit_above_their_body() {
    let x = layout(&[var("x")], MathStyle::TEXT);
    for kind in [
        AccentKind::Hat,
        AccentKind::Bar,
        AccentKind::Vector,
        AccentKind::Dot,
        AccentKind::DoubleDot,
        AccentKind::Tilde,
    ] {
        let accented = layout(
            &[MathNode::Accent {
                accent: kind,
                body: vec![var("x")],
            }],
            MathStyle::TEXT,
        );
        assert!(accented.ascent > x.ascent + 0.1 * SIZE, "{kind:?}");
        assert!(
            accented.drawing.primitives.len() > 1,
            "{kind:?} draws a mark"
        );
    }
}

#[test]
fn italic_letters_map_to_math_italic_and_bold_to_math_bold() {
    let letters = layout(
        &[
            var("x"),
            atom("v", AtomClass::Ordinary, AtomStyle::Bold),
            num("2"),
        ],
        MathStyle::TEXT,
    );
    let drawn: Vec<String> = texts(&letters).into_iter().map(|(t, _)| t).collect();
    assert_eq!(drawn, vec!["𝑥", "𝐯", "2"]);
}

#[test]
fn errors_show_source_with_tooltip() {
    let error = layout(
        &[MathNode::Error {
            source: "\\frac{".into(),
            message: "unclosed".into(),
        }],
        MathStyle::TEXT,
    );
    assert!(texts(&error).iter().any(|(t, _)| t == "\\frac{"));
    assert!(error
        .drawing
        .hotspots
        .iter()
        .any(|h| h.kind == HotspotKind::Tooltip("unclosed".into())));
}

#[test]
fn spaces_add_width() {
    let thin = layout(&[MathNode::Space(SpaceWidth::Thin)], MathStyle::TEXT).width;
    let quad = layout(&[MathNode::Space(SpaceWidth::Quad)], MathStyle::TEXT).width;
    assert!((quad - SIZE).abs() < 0.01 && (thin - SIZE / 6.0).abs() < 0.01);
}

#[test]
fn inline_math_breaks_after_relations_and_binary_operators() {
    let math = [
        var("a"),
        rel("="),
        var("b"),
        bin("+"),
        var("c"),
        rel("≤"),
        rel("="),
        var("d"),
    ];
    let inline = with_measure(|m| MathLayout::new(m, SIZE).inline(&math));
    // After `=`, after `+`, and after the second relation (not between ≤ and =).
    assert_eq!(inline.breaks.len(), 3, "{:?}", inline.breaks);
    let pieces = inline.split_at_breaks();
    assert_eq!(pieces.len(), 4);
    let total: f32 = pieces.iter().map(|(piece, glue)| piece.width + glue).sum();
    assert!((total - inline.width).abs() < 0.01);
    assert_eq!(
        pieces
            .iter()
            .map(|(p, _)| p.drawing.primitives.len())
            .sum::<usize>(),
        inline.drawing.primitives.len()
    );
}

#[test]
fn display_rows_align_on_cell_boundaries() {
    let row = |left: MathList, right: MathList| MathRow {
        cells: vec![left, right],
    };
    let math = DisplayMath {
        rows: vec![
            row(vec![var("x")], vec![rel("="), num("1")]),
            row(
                vec![var("x"), bin("+"), var("y"), bin("+"), var("z")],
                vec![rel("="), num("2")],
            ),
        ],
    };
    let display = with_measure(|m| MathLayout::new(m, SIZE).display(&math));
    let equals: Vec<f32> = display
        .drawing
        .primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::Text { pos, text, .. } if text == "=" => Some(pos.x),
            _ => None,
        })
        .collect();
    assert_eq!(equals.len(), 2);
    assert!(
        (equals[0] - equals[1]).abs() < 0.01,
        "relations not aligned: {equals:?}"
    );
    assert!(display.height() > 2.0 * SIZE);
}

#[test]
fn every_node_kind_lays_out_without_panicking_even_when_empty() {
    let empty: MathList = vec![];
    let nodes = vec![
        atom("", AtomClass::Ordinary, AtomStyle::Italic),
        MathNode::Text(String::new()),
        frac(empty.clone(), empty.clone()),
        scripts(
            MathNode::Group(empty.clone()),
            Some(empty.clone()),
            Some(empty.clone()),
        ),
        scripts(
            atom(
                "∑",
                AtomClass::LargeOperator { limits: true },
                AtomStyle::Upright,
            ),
            Some(empty.clone()),
            None,
        ),
        MathNode::Root {
            index: Some(empty.clone()),
            radicand: empty.clone(),
        },
        fenced("", "", empty.clone()),
        fenced("(", "?", empty.clone()),
        MathNode::Matrix {
            open: "(".into(),
            close: ")".into(),
            rows: vec![],
        },
        MathNode::Matrix {
            open: "{".into(),
            close: "".into(),
            rows: vec![vec![], vec![empty.clone()]],
        },
        MathNode::Accent {
            accent: AccentKind::Vector,
            body: empty.clone(),
        },
        MathNode::Group(empty.clone()),
        MathNode::Error {
            source: String::new(),
            message: String::new(),
        },
    ];
    with_measure(|m| {
        let mut layout = MathLayout::new(m, SIZE);
        for style in [
            MathStyle::DISPLAY,
            MathStyle::TEXT,
            MathStyle::SCRIPT_SCRIPT,
        ] {
            let laid_out = layout.list(&nodes, style);
            assert!(
                laid_out.width.is_finite()
                    && laid_out.ascent.is_finite()
                    && laid_out.descent.is_finite()
            );
        }
        let empty_display = layout.display(&DisplayMath {
            rows: vec![MathRow { cells: vec![] }, MathRow::default()],
        });
        assert!(empty_display.height().is_finite());
        assert_eq!(layout.inline(&[]).width, 0.0);
    });
}

#[test]
fn pathological_nesting_is_cut_off_instead_of_overflowing() {
    let mut node = var("x");
    for _ in 0..500 {
        node = frac(vec![node], vec![num("1")]);
    }
    let deep = layout(&[node], MathStyle::DISPLAY);
    assert!(deep
        .drawing
        .hotspots
        .iter()
        .any(|h| matches!(h.kind, HotspotKind::Tooltip(_))));
}
