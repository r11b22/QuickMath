//! Hand-built documents exercising every syntax-tree variant, shared by the
//! exporter tests (the parser is developed in parallel, so tests must not
//! depend on it).

use qm_format::*;

pub(crate) fn atom(text: &str, latex: &str, class: AtomClass, style: AtomStyle) -> MathNode {
    MathNode::Atom(Atom {
        text: text.to_string(),
        latex: latex.to_string(),
        class,
        style,
    })
}

pub(crate) fn var(name: &str) -> MathNode {
    atom(name, name, AtomClass::Ordinary, AtomStyle::Italic)
}

pub(crate) fn num(value: &str) -> MathNode {
    atom(value, value, AtomClass::Ordinary, AtomStyle::Upright)
}

pub(crate) fn rel(text: &str, latex: &str) -> MathNode {
    atom(text, latex, AtomClass::Relation, AtomStyle::Upright)
}

pub(crate) fn text(value: &str) -> Inline {
    Inline::Text(value.to_string())
}

pub(crate) fn block(kind: BlockKind) -> Block {
    Block { kind, line: 0 }
}

pub(crate) fn paragraph(content: Vec<Inline>) -> Block {
    block(BlockKind::Paragraph(content))
}

pub(crate) fn display(rows: Vec<Vec<MathList>>) -> Block {
    block(BlockKind::DisplayMath(DisplayMath {
        rows: rows.into_iter().map(|cells| MathRow { cells }).collect(),
    }))
}

pub(crate) fn callout(kind: CalloutKind, title: &str, body: Vec<Block>) -> Block {
    let title = if title.is_empty() {
        vec![]
    } else {
        vec![text(title)]
    };
    block(BlockKind::Callout { kind, title, body })
}

/// Every atom shape the converters special-case: letters, numbers, Unicode
/// symbols, operators with and without limits, multi-character atoms of
/// every class and style, and ASCII punctuation with meaning in Typst/LaTeX.
pub(crate) fn every_atom() -> MathList {
    use AtomClass::*;
    use AtomStyle::*;
    vec![
        var("x"),
        num("42"),
        num("3.14"),
        atom("+", "+", Binary, Upright),
        rel("≤", "\\leq"),
        rel("=", "="),
        atom("α", "\\alpha", Ordinary, Italic),
        atom("Γ", "\\Gamma", Ordinary, Upright),
        atom("ℝ", "\\mathbb{R}", Ordinary, Upright),
        atom("sin", "\\sin", Ordinary, Upright),
        atom("ab", "ab", Ordinary, Italic),
        atom("v", "\\mathbf{v}", Ordinary, Bold),
        atom("AB", "\\mathbf{AB}", Ordinary, Bold),
        atom("7", "\\mathbf{7}", Ordinary, Bold),
        atom("12", "\\mathbf{12}", Ordinary, Bold),
        atom("∑", "\\sum", LargeOperator { limits: true }, Upright),
        atom("lim", "\\lim", LargeOperator { limits: true }, Upright),
        atom("max", "\\max", LargeOperator { limits: false }, Upright),
        atom(":=", "\\coloneqq", Relation, Upright),
        atom("mod", "\\bmod", Binary, Upright),
        atom("((", "((", Open, Upright),
        atom("))", "))", Close, Upright),
        atom(",,", ",,", Punctuation, Upright),
        atom("", "", Ordinary, Italic),
        atom("1.", "1.", Ordinary, Upright),
        atom("x", "x", LargeOperator { limits: false }, Italic),
    ]
    .into_iter()
    .chain(
        ",;:/\\\"'#$&_^!?()[]{}|<>-*~@%`."
            .chars()
            .map(|c| atom(&c.to_string(), &c.to_string(), Ordinary, Upright)),
    )
    .collect()
}

/// One node of every [`MathNode`] variant, plus the edge cases (empty lists,
/// one-sided fences, unusual delimiters) that must still typeset.
pub(crate) fn every_math_node() -> MathList {
    use AtomClass::*;
    use AtomStyle::*;
    let sum = atom("∑", "\\sum", LargeOperator { limits: true }, Upright);
    let integral = atom("∫", "\\int", LargeOperator { limits: false }, Upright);
    let fenced = |open: &str, close: &str, body: MathList| MathNode::Fenced {
        open: open.into(),
        close: close.into(),
        body,
    };
    let matrix = |open: &str, close: &str, rows: Vec<Vec<MathList>>| MathNode::Matrix {
        open: open.into(),
        close: close.into(),
        rows,
    };
    let accent = |accent| MathNode::Accent {
        accent,
        body: vec![var("x")],
    };
    vec![
        MathNode::Group(every_atom()),
        MathNode::Text("otherwise".into()),
        MathNode::Text("a \"quoted\" \\ text".into()),
        MathNode::Text(String::new()),
        MathNode::Fraction {
            numerator: vec![var("a"), atom("+", "+", Binary, Upright), var("b")],
            denominator: vec![num("2")],
        },
        MathNode::Fraction {
            numerator: vec![],
            denominator: vec![],
        },
        MathNode::Scripts {
            base: Box::new(var("x")),
            sub: Some(vec![var("i")]),
            sup: Some(vec![num("2")]),
        },
        MathNode::Scripts {
            base: Box::new(sum),
            sub: Some(vec![var("i"), rel("=", "="), num("1")]),
            sup: Some(vec![var("n")]),
        },
        MathNode::Scripts {
            base: Box::new(integral),
            sub: Some(vec![num("0")]),
            sup: Some(vec![num("1")]),
        },
        MathNode::Scripts {
            base: Box::new(atom(
                "lim",
                "\\lim",
                LargeOperator { limits: true },
                Upright,
            )),
            sub: Some(vec![
                var("n"),
                rel("→", "\\to"),
                atom("∞", "\\infty", Ordinary, Upright),
            ]),
            sup: None,
        },
        MathNode::Scripts {
            base: Box::new(MathNode::Group(vec![])),
            sub: Some(vec![]),
            sup: Some(vec![]),
        },
        MathNode::Scripts {
            base: Box::new(MathNode::Text(String::new())),
            sub: None,
            sup: None,
        },
        MathNode::Root {
            index: None,
            radicand: vec![var("x")],
        },
        MathNode::Root {
            index: Some(vec![num("3")]),
            radicand: vec![],
        },
        MathNode::Root {
            index: Some(vec![]),
            radicand: vec![var("y")],
        },
        fenced(
            "(",
            ")",
            vec![var("a"), atom(",", ",", Punctuation, Upright), var("b")],
        ),
        fenced(
            "{",
            "",
            vec![
                var("f"),
                atom("(", "(", Open, Upright),
                var("x"),
                atom(")", ")", Close, Upright),
            ],
        ),
        fenced("", "|", vec![var("x")]),
        fenced("", "", vec![var("z")]),
        fenced("⟨", "⟩", vec![]),
        fenced("‖", "‖", vec![var("v")]),
        fenced("\\{", "\\}", vec![var("w")]),
        matrix(
            "[",
            "]",
            vec![
                vec![vec![num("1")], vec![num("2")]],
                vec![vec![num("3")], vec![num("4")]],
            ],
        ),
        matrix(
            "(",
            ")",
            vec![vec![vec![var("a")]], vec![vec![var("b")], vec![var("c")]]],
        ),
        matrix("", "", vec![vec![vec![], vec![var("d")]], vec![]]),
        matrix(
            "{",
            "",
            vec![
                vec![
                    vec![num("1")],
                    vec![
                        MathNode::Text("if".into()),
                        var("x"),
                        rel(">", ">"),
                        num("0"),
                    ],
                ],
                vec![vec![num("0")], vec![MathNode::Text("otherwise".into())]],
                vec![],
            ],
        ),
        matrix("", "}", vec![vec![vec![var("p")]]]),
        matrix("x", "yy", vec![vec![vec![var("q")]]]),
        matrix("[", "]", vec![]),
        accent(AccentKind::Hat),
        accent(AccentKind::Bar),
        accent(AccentKind::Vector),
        accent(AccentKind::Dot),
        accent(AccentKind::DoubleDot),
        accent(AccentKind::Tilde),
        MathNode::Accent {
            accent: AccentKind::Hat,
            body: vec![],
        },
        MathNode::Group(vec![]),
        MathNode::Space(SpaceWidth::Thin),
        MathNode::Space(SpaceWidth::Medium),
        MathNode::Space(SpaceWidth::Thick),
        MathNode::Space(SpaceWidth::Quad),
        MathNode::Error {
            source: "\\frac{\"#$".into(),
            message: "unclosed".into(),
        },
    ]
}

/// A note using every block kind, every inline kind, every callout kind and
/// every math node, with full front matter.
pub(crate) fn kitchen_sink() -> Document {
    let mut blocks: Vec<Block> = (0..=7u8)
        .map(|level| {
            block(BlockKind::Heading {
                level,
                content: vec![
                    text(&format!("Heading {level} with $ & #")),
                    Inline::Math(vec![var("x")]),
                ],
            })
        })
        .collect();
    blocks.extend([
        paragraph(every_inline()),
        paragraph(vec![]),
        paragraph(vec![Inline::LineBreak]),
        paragraph(vec![text("1. - > = + not markup")]),
        display(vec![vec![every_math_node()]]),
        display(vec![
            vec![vec![var("a")], vec![rel("=", "="), var("b")]],
            vec![vec![], vec![rel("=", "="), var("c")]],
        ]),
        display(vec![vec![vec![var("u")]], vec![vec![var("v")]]]),
        display(vec![]),
        display(vec![vec![vec![]]]),
        block(BlockKind::List {
            ordered: false,
            items: vec![
                vec![paragraph(vec![text("first")])],
                vec![
                    paragraph(vec![text("second")]),
                    block(BlockKind::List {
                        ordered: true,
                        items: vec![vec![paragraph(vec![text("nested")])], vec![]],
                    }),
                    display(vec![vec![vec![var("y")]]]),
                ],
            ],
        }),
        block(BlockKind::List {
            ordered: true,
            items: vec![],
        }),
        block(BlockKind::Code {
            language: Some("rust".into()),
            code: "fn main() { println!(\"\\\\ ```\"); }\n".into(),
        }),
        block(BlockKind::Code {
            language: None,
            code: String::new(),
        }),
        block(BlockKind::Quote(vec![
            paragraph(vec![text("quoted")]),
            block(BlockKind::Quote(vec![paragraph(vec![text(
                "nested quote",
            )])])),
        ])),
        block(BlockKind::Quote(vec![])),
        block(BlockKind::Rule),
    ]);
    blocks.extend(every_callout());
    Document {
        front_matter: FrontMatter {
            title: Some("Linear \"Algebra\" & #1".into()),
            author: Some("Ada Lovelace".into()),
            date: Some("2026-10-31".into()),
            course: Some("MATH_101 {intro}".into()),
            tags: vec!["math".into(), "#lecture".into()],
        },
        blocks,
    }
}

pub(crate) fn every_inline() -> Vec<Inline> {
    vec![
        text("Specials: \\ * _ ` # [ ] $ < > @ = - + / ~ : % & ^ { } | \" ' 1. //\nnext line "),
        Inline::Strong(vec![text("bold "), Inline::Emphasis(vec![text("both")])]),
        text("("),
        Inline::Emphasis(vec![text("emph")]),
        text(")"),
        Inline::Strong(vec![]),
        Inline::Code("let x = `y`;".into()),
        Inline::Code(String::new()),
        Inline::Math(vec![
            var("x"),
            atom("+", "+", AtomClass::Binary, AtomStyle::Upright),
            num("1"),
        ]),
        Inline::Math(vec![]),
        Inline::Math(every_math_node()),
        Inline::WikiLink {
            target: "Topology.qm#Open sets".into(),
            label: None,
        },
        Inline::WikiLink {
            target: "week1/intro".into(),
            label: Some("intro [1]".into()),
        },
        Inline::LineBreak,
        text("after break"),
    ]
}

pub(crate) fn every_callout() -> Vec<Block> {
    use CalloutKind::*;
    [
        Definition, Theorem, Lemma, Corollary, Proof, Example, Note, Warning,
    ]
    .into_iter()
    .flat_map(|kind| {
        [
            callout(
                kind,
                "Named [x]",
                vec![
                    paragraph(vec![text("Body.")]),
                    display(vec![vec![vec![var("x")]]]),
                ],
            ),
            callout(kind, "", vec![]),
        ]
    })
    .collect()
}
