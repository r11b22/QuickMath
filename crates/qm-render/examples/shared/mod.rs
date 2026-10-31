//! A hand-built lecture note exercising every block and math construct, used
//! by the `preview` and `screenshot` examples until the parser lands.

#![allow(dead_code)]

use qm_format::*;

pub fn atom(text: &str, class: AtomClass, style: AtomStyle) -> MathNode {
    MathNode::Atom(Atom {
        text: text.into(),
        latex: text.into(),
        class,
        style,
    })
}
pub fn var(text: &str) -> MathNode {
    atom(text, AtomClass::Ordinary, AtomStyle::Italic)
}
pub fn bold(text: &str) -> MathNode {
    atom(text, AtomClass::Ordinary, AtomStyle::Bold)
}
pub fn num(text: &str) -> MathNode {
    atom(text, AtomClass::Ordinary, AtomStyle::Upright)
}
pub fn bin(text: &str) -> MathNode {
    atom(text, AtomClass::Binary, AtomStyle::Upright)
}
pub fn rel(text: &str) -> MathNode {
    atom(text, AtomClass::Relation, AtomStyle::Upright)
}
pub fn open(text: &str) -> MathNode {
    atom(text, AtomClass::Open, AtomStyle::Upright)
}
pub fn close(text: &str) -> MathNode {
    atom(text, AtomClass::Close, AtomStyle::Upright)
}
pub fn punct(text: &str) -> MathNode {
    atom(text, AtomClass::Punctuation, AtomStyle::Upright)
}
pub fn op(text: &str, limits: bool) -> MathNode {
    atom(
        text,
        AtomClass::LargeOperator { limits },
        AtomStyle::Upright,
    )
}
pub fn func(name: &str) -> MathNode {
    op(name, false)
}
pub fn frac(numerator: MathList, denominator: MathList) -> MathNode {
    MathNode::Fraction {
        numerator,
        denominator,
    }
}
pub fn sup(base: MathNode, sup: MathList) -> MathNode {
    MathNode::Scripts {
        base: Box::new(base),
        sub: None,
        sup: Some(sup),
    }
}
pub fn sub(base: MathNode, sub: MathList) -> MathNode {
    MathNode::Scripts {
        base: Box::new(base),
        sub: Some(sub),
        sup: None,
    }
}
pub fn subsup(base: MathNode, sub: MathList, sup: MathList) -> MathNode {
    MathNode::Scripts {
        base: Box::new(base),
        sub: Some(sub),
        sup: Some(sup),
    }
}
pub fn sqrt(radicand: MathList) -> MathNode {
    MathNode::Root {
        index: None,
        radicand,
    }
}
pub fn root(index: MathList, radicand: MathList) -> MathNode {
    MathNode::Root {
        index: Some(index),
        radicand,
    }
}
pub fn fenced(open: &str, close: &str, body: MathList) -> MathNode {
    MathNode::Fenced {
        open: open.into(),
        close: close.into(),
        body,
    }
}
pub fn matrix(open: &str, close: &str, rows: Vec<Vec<MathList>>) -> MathNode {
    MathNode::Matrix {
        open: open.into(),
        close: close.into(),
        rows,
    }
}
pub fn accent(accent: AccentKind, body: MathList) -> MathNode {
    MathNode::Accent { accent, body }
}
pub fn text(text: &str) -> MathNode {
    MathNode::Text(text.into())
}

pub fn t(text: &str) -> Inline {
    Inline::Text(text.into())
}
pub fn m(math: MathList) -> Inline {
    Inline::Math(math)
}

pub fn paragraph(line: usize, content: Vec<Inline>) -> Block {
    Block {
        kind: BlockKind::Paragraph(content),
        line,
    }
}
pub fn heading(line: usize, level: u8, title: &str) -> Block {
    Block {
        kind: BlockKind::Heading {
            level,
            content: vec![t(title)],
        },
        line,
    }
}
pub fn display(line: usize, rows: Vec<Vec<MathList>>) -> Block {
    let rows = rows.into_iter().map(|cells| MathRow { cells }).collect();
    Block {
        kind: BlockKind::DisplayMath(DisplayMath { rows }),
        line,
    }
}
pub fn callout(line: usize, kind: CalloutKind, title: &str, body: Vec<Block>) -> Block {
    let title = if title.is_empty() {
        vec![]
    } else {
        vec![t(title)]
    };
    Block {
        kind: BlockKind::Callout { kind, title, body },
        line,
    }
}

/// Σ_{k=1}^{∞} 1/k² = π²/6
fn basel() -> MathList {
    vec![
        subsup(
            op("∑", true),
            vec![var("k"), rel("="), num("1")],
            vec![num("∞")],
        ),
        frac(vec![num("1")], vec![sup(var("k"), vec![num("2")])]),
        rel("="),
        frac(vec![sup(var("π"), vec![num("2")])], vec![num("6")]),
    ]
}

fn integral() -> MathList {
    vec![
        subsup(op("∫", false), vec![num("0")], vec![num("1")]),
        sup(var("x"), vec![num("2")]),
        MathNode::Space(SpaceWidth::Thin),
        var("d"),
        var("x"),
        rel("="),
        subsup(
            fenced(
                "[",
                "]",
                vec![frac(vec![sup(var("x"), vec![num("3")])], vec![num("3")])],
            ),
            vec![num("0")],
            vec![num("1")],
        ),
        rel("="),
        frac(vec![num("1")], vec![num("3")]),
    ]
}

fn cases() -> MathList {
    vec![
        var("f"),
        fenced("(", ")", vec![var("x")]),
        rel("="),
        matrix(
            "{",
            "",
            vec![
                vec![
                    vec![sup(var("x"), vec![num("2")])],
                    vec![var("x"), rel("≥"), num("0")],
                ],
                vec![vec![bin("−"), var("x")], vec![text("otherwise")]],
            ],
        ),
    ]
}

fn matrices() -> MathList {
    let cell = |s: &str| vec![num(s)];
    vec![
        var("A"),
        rel("="),
        matrix(
            "(",
            ")",
            vec![vec![cell("1"), cell("2")], vec![cell("3"), cell("4")]],
        ),
        punct(","),
        MathNode::Space(SpaceWidth::Quad),
        func("det"),
        var("A"),
        rel("="),
        bin("−"),
        num("2"),
        punct(","),
        MathNode::Space(SpaceWidth::Quad),
        matrix(
            "[",
            "]",
            vec![
                vec![vec![var("a")], vec![var("b")], vec![var("c")]],
                vec![vec![var("d")], vec![var("e")], vec![var("f")]],
                vec![vec![var("g")], vec![var("h")], vec![var("i")]],
            ],
        ),
    ]
}

fn gallery() -> MathList {
    vec![
        sqrt(vec![
            sup(var("x"), vec![num("2")]),
            bin("+"),
            sup(var("y"), vec![num("2")]),
        ]),
        punct(","),
        MathNode::Space(SpaceWidth::Quad),
        root(vec![num("3")], vec![frac(vec![var("a")], vec![var("b")])]),
        punct(","),
        MathNode::Space(SpaceWidth::Quad),
        accent(AccentKind::Hat, vec![var("x")]),
        accent(AccentKind::Vector, vec![var("v")]),
        accent(AccentKind::Bar, vec![var("z")]),
        accent(AccentKind::Dot, vec![var("x")]),
        accent(AccentKind::DoubleDot, vec![var("x")]),
        accent(AccentKind::Tilde, vec![var("n")]),
        accent(AccentKind::Vector, vec![var("A"), var("B")]),
        punct(","),
        MathNode::Space(SpaceWidth::Quad),
        fenced(
            "(",
            ")",
            vec![frac(
                vec![frac(vec![var("a")], vec![var("b")])],
                vec![var("c")],
            )],
        ),
        fenced(
            "{",
            "}",
            vec![
                sub(var("x"), vec![var("i")]),
                MathNode::Space(SpaceWidth::Medium),
                text(":"),
                MathNode::Space(SpaceWidth::Medium),
                frac(vec![num("1")], vec![sup(num("2"), vec![var("i")])]),
                rel("<"),
                var("ε"),
            ],
        ),
        fenced("|", "|", vec![frac(vec![var("x")], vec![num("2")])]),
        fenced(
            "⟨",
            "⟩",
            vec![var("u"), punct(","), frac(vec![var("v")], vec![num("2")])],
        ),
    ]
}

fn aligned() -> Vec<Vec<MathList>> {
    vec![
        vec![
            vec![
                subsup(
                    op("∑", true),
                    vec![var("k"), rel("="), num("0")],
                    vec![var("n")],
                ),
                sup(var("r"), vec![var("k")]),
            ],
            vec![
                rel("="),
                frac(
                    vec![
                        num("1"),
                        bin("−"),
                        sup(var("r"), vec![var("n"), bin("+"), num("1")]),
                    ],
                    vec![num("1"), bin("−"), var("r")],
                ),
            ],
        ],
        vec![
            vec![],
            vec![
                rel("→"),
                frac(vec![num("1")], vec![num("1"), bin("−"), var("r")]),
            ],
        ],
    ]
}

pub fn sample_document() -> Document {
    let n_in_n = vec![sub(
        fenced("(", ")", vec![sub(var("a"), vec![var("n")])]),
        vec![var("n"), rel("∈"), num("ℕ")],
    )];
    let blocks = vec![
        heading(8, 1, "Sequences and series"),
        paragraph(
            10,
            vec![
                t("Let "),
                m(n_in_n),
                t(" be a sequence of real numbers. We say it "),
                Inline::Strong(vec![t("converges")]),
                t(" to "),
                m(vec![var("L"), rel("∈"), num("ℝ")]),
                t(" if for every "),
                m(vec![var("ε"), rel(">"), num("0")]),
                t(" there is an "),
                m(vec![var("N")]),
                t(" such that "),
                m(vec![open("|"), sub(var("a"), vec![var("n")]), bin("−"), var("L"), close("|"), rel("<"), var("ε")]),
                t(" for all "),
                m(vec![var("n"), rel("≥"), var("N")]),
                t(". See "),
                Inline::WikiLink { target: "Lecture 6".into(), label: Some("the previous lecture".into()) },
                t(" for "),
                Inline::Emphasis(vec![t("Cauchy sequences")]),
                t(", and recall that "),
                m(vec![frac(vec![num("1")], vec![var("n")]), rel("→"), num("0")]),
                t(" while "),
                m(vec![sup(var("x"), vec![sup(num("2"), vec![var("k")])])]),
                t(" grows quickly; long formulas such as "),
                m(vec![
                    var("f"),
                    fenced("(", ")", vec![var("x")]),
                    rel("="),
                    sub(var("a"), vec![num("0")]),
                    bin("+"),
                    sub(var("a"), vec![num("1")]),
                    var("x"),
                    bin("+"),
                    sub(var("a"), vec![num("2")]),
                    sup(var("x"), vec![num("2")]),
                    bin("+"),
                    sub(var("a"), vec![num("3")]),
                    sup(var("x"), vec![num("3")]),
                    bin("+"),
                    num("⋯"),
                ]),
                t(" break after operators. Code like "),
                Inline::Code("cargo test".into()),
                t(" stays inline."),
            ],
        ),
        callout(14, CalloutKind::Definition, "Series", vec![paragraph(15, vec![
            t("The series "),
            m(vec![subsup(op("∑", true), vec![var("k"), rel("="), num("1")], vec![num("∞")]), sub(var("a"), vec![var("k")])]),
            t(" converges if its partial sums "),
            m(vec![sub(var("s"), vec![var("n")]), rel("="), subsup(op("∑", true), vec![var("k"), rel("="), num("1")], vec![var("n")]), sub(var("a"), vec![var("k")])]),
            t(" converge."),
        ])]),
        display(17, vec![vec![basel()]]),
        callout(19, CalloutKind::Theorem, "Geometric series", vec![
            paragraph(20, vec![t("For "), m(vec![open("|"), var("r"), close("|"), rel("<"), num("1")]), t(" we have")]),
            display(21, aligned()),
        ]),
        callout(24, CalloutKind::Proof, "", vec![paragraph(25, vec![
            t("Multiply "),
            m(vec![sub(var("s"), vec![var("n")])]),
            t(" by "),
            m(vec![num("1"), bin("−"), var("r")]),
            t(" and telescope; then use "),
            m(vec![sup(var("r"), vec![var("n"), bin("+"), num("1")]), rel("→"), num("0")]),
            t("."),
        ])]),
        heading(27, 2, "Integrals and friends"),
        display(29, vec![vec![integral()]]),
        display(31, vec![vec![cases()]]),
        display(33, vec![vec![matrices()]]),
        display(35, vec![vec![gallery()]]),
        display(37, vec![vec![vec![
            fenced("(", ")", vec![matrix("", "", vec![vec![vec![frac(vec![num("1")], vec![num("2")])]], vec![vec![frac(vec![num("3")], vec![num("4")])]], vec![vec![frac(vec![num("5")], vec![num("6")])]]])]),
            fenced("[", "]", vec![matrix("", "", vec![vec![vec![var("x")]], vec![vec![var("y")]], vec![vec![var("z")]], vec![vec![var("w")]]])]),
            fenced("‖", "‖", vec![matrix("", "", vec![vec![vec![var("x")]], vec![vec![var("y")]], vec![vec![var("z")]]])]),
            fenced("⌊", "⌋", vec![frac(vec![var("n")], vec![num("2")])]),
            fenced("⌈", "⌉", vec![frac(vec![var("n")], vec![num("2")])]),
            fenced("⟨", "⟩", vec![matrix("", "", vec![vec![vec![var("x")]], vec![vec![var("y")]], vec![vec![var("z")]]])]),
            subsup(op("∫", false), vec![bin("−"), num("∞")], vec![num("∞")]),
            sup(var("e"), vec![bin("−"), sup(var("x"), vec![num("2")])]),
            var("d"),
            var("x"),
            rel("="),
            sqrt(vec![var("π")]),
            punct(","),
            MathNode::Space(SpaceWidth::Quad),
            sub(op("lim", true), vec![var("n"), rel("→"), num("∞")]),
            sup(fenced("(", ")", vec![num("1"), bin("+"), frac(vec![num("1")], vec![var("n")])]), vec![var("n")]),
            rel("="),
            var("e"),
        ]]]),
        heading(39, 3, "Lists, code and quotes"),
        Block {
            kind: BlockKind::List {
                ordered: false,
                items: vec![
                    vec![paragraph(41, vec![t("Bold vectors like "), m(vec![bold("v"), rel("∈"), sup(num("ℝ"), vec![var("n")])]), t(" and Greek "), m(vec![var("α"), punct(","), var("β"), punct(","), var("Γ")]), t(".")])],
                    vec![
                        paragraph(42, vec![t("A nested list:")]),
                        Block {
                            kind: BlockKind::List {
                                ordered: true,
                                items: vec![
                                    vec![paragraph(43, vec![t("first, with "), m(vec![sub(var("x"), vec![var("i")]), rel("="), frac(vec![var("i")], vec![var("n")])])])],
                                    vec![paragraph(44, vec![t("second item that is long enough to wrap onto a following line so we can check the hanging indent of list items.")])],
                                ],
                            },
                            line: 43,
                        },
                    ],
                    vec![paragraph(45, vec![t("Errors show in red: "), m(vec![var("x"), bin("+"), MathNode::Error { source: "\\frac{1}{".into(), message: "unclosed `{` in fraction".into() }])])],
                ],
            },
            line: 41,
        },
        Block { kind: BlockKind::Code { language: Some("python".into()), code: "def partial_sum(a, n):\n    return sum(a(k) for k in range(1, n + 1))\n".into() }, line: 47 },
        Block { kind: BlockKind::Quote(vec![paragraph(51, vec![t("God made the integers; all else is the work of man. — Kronecker")])]), line: 51 },
        Block { kind: BlockKind::Rule, line: 53 },
        callout(55, CalloutKind::Lemma, "", vec![paragraph(56, vec![t("If "), m(vec![sub(var("a"), vec![var("n")]), rel("→"), var("L")]), t(" then every subsequence converges to "), m(vec![var("L")]), t(".")])]),
        callout(58, CalloutKind::Corollary, "", vec![paragraph(59, vec![t("Limits are unique.")])]),
        callout(61, CalloutKind::Example, "Harmonic series", vec![paragraph(62, vec![m(vec![subsup(op("∑", true), vec![var("k"), rel("="), num("1")], vec![num("∞")]), frac(vec![num("1")], vec![var("k")]), rel("="), num("∞")]), t(" although the terms go to zero.")])]),
        callout(64, CalloutKind::Note, "", vec![paragraph(65, vec![t("Absolute convergence implies convergence.")])]),
        callout(67, CalloutKind::Warning, "", vec![paragraph(68, vec![t("Rearranging a conditionally convergent series can change its sum.")])]),
    ];
    Document {
        front_matter: FrontMatter {
            title: Some("Real Analysis — Lecture 7".into()),
            author: Some("Joost".into()),
            date: Some("2026-10-31".into()),
            course: Some("MATH 201".into()),
            tags: vec!["analysis".into(), "series".into(), "integrals".into()],
        },
        blocks,
    }
}

/// The sample repeated until it spans roughly `lines` source lines.
pub fn long_document(lines: usize) -> Document {
    let sample = sample_document();
    let span = 70;
    let mut blocks = Vec::new();
    for copy in 0..lines.div_ceil(span) {
        blocks.extend(sample.blocks.iter().cloned().map(|mut block| {
            shift_lines(&mut block, copy * span);
            block
        }));
    }
    Document {
        front_matter: sample.front_matter,
        blocks,
    }
}

fn shift_lines(block: &mut Block, by: usize) {
    block.line += by;
    match &mut block.kind {
        BlockKind::List { items, .. } => {
            items.iter_mut().flatten().for_each(|b| shift_lines(b, by))
        }
        BlockKind::Callout { body, .. } | BlockKind::Quote(body) => {
            body.iter_mut().for_each(|b| shift_lines(b, by))
        }
        _ => {}
    }
}
