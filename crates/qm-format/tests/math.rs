//! Math notation behaviour, checked mostly through the LaTeX output.

use qm_format::{
    math_to_latex, parse_math, vocabulary, AccentKind, AtomClass, AtomStyle, MathNode,
};

fn latex(source: &str) -> String {
    math_to_latex(&parse_math(source))
}

fn single(source: &str) -> MathNode {
    let mut list = parse_math(source);
    assert_eq!(
        list.len(),
        1,
        "expected one node for {source:?}, got {list:?}"
    );
    list.remove(0)
}

fn atom(source: &str) -> qm_format::Atom {
    match single(source) {
        MathNode::Atom(atom) => atom,
        other => panic!("expected an atom for {source:?}, got {other:?}"),
    }
}

// ----- atoms ---------------------------------------------------------------

#[test]
fn single_letters_are_italic_and_numbers_upright() {
    assert_eq!(atom("x").style, AtomStyle::Italic);
    let number = atom("3.14");
    assert_eq!(
        (number.text.as_str(), number.style),
        ("3.14", AtomStyle::Upright)
    );
}

#[test]
fn unknown_words_become_italic_letters() {
    let list = parse_math("abc");
    assert_eq!(list.len(), 3);
    assert_eq!(latex("abc"), "abc");
}

#[test]
fn spaces_do_not_render() {
    assert_eq!(latex("a  b"), "ab");
}

#[test]
fn greek_letters_have_unicode_and_latex() {
    let alpha = atom("alpha");
    assert_eq!(
        (alpha.text.as_str(), alpha.latex.as_str(), alpha.style),
        ("α", "\\alpha", AtomStyle::Italic)
    );
    assert_eq!(atom("Gamma").style, AtomStyle::Upright);
    assert_eq!(atom("eps").text, "ε");
    assert_eq!(atom("phi").latex, "\\varphi");
}

#[test]
fn letter_like_symbols() {
    assert_eq!(atom("oo").text, "∞");
    assert_eq!(atom("inf").text, "∞");
    assert_eq!(atom("del").text, "∂");
    assert_eq!(atom("grad").text, "∇");
    let reals = atom("RR");
    assert_eq!(
        (reals.text.as_str(), reals.latex.as_str()),
        ("ℝ", "\\mathbb{R}")
    );
    assert_eq!(
        latex("NN ZZ QQ CC"),
        "\\mathbb{N}\\mathbb{Z}\\mathbb{Q}\\mathbb{C}"
    );
    assert_eq!(atom("...").text, "…");
    assert_eq!(atom("cdots").latex, "\\cdots");
}

#[test]
fn relations_and_arrows() {
    let cases = [
        ("=", "="),
        ("/=", "≠"),
        ("!=", "≠"),
        ("<=", "≤"),
        (">=", "≥"),
        ("~=", "≈"),
        ("==", "≡"),
        ("~", "∼"),
        ("->", "→"),
        ("<-", "←"),
        ("=>", "⇒"),
        ("<=>", "⇔"),
        ("|->", "↦"),
        ("in", "∈"),
        ("!in", "∉"),
        ("sub", "⊂"),
        ("sube", "⊆"),
        ("supset", "⊃"),
        ("supe", "⊇"),
        ("le", "≤"),
        ("to", "→"),
    ];
    for (source, text) in cases {
        let relation = atom(source);
        assert_eq!(relation.text, text, "{source}");
        assert_eq!(relation.class, AtomClass::Relation, "{source}");
    }
}

#[test]
fn binary_operators() {
    for (source, text) in [
        ("+", "+"),
        ("-", "−"),
        ("+-", "±"),
        ("-+", "∓"),
        ("*", "⋅"),
        ("xx", "×"),
        ("cup", "∪"),
        ("cap", "∩"),
        ("and", "∧"),
        ("or", "∨"),
    ] {
        let operator = atom(source);
        assert_eq!(operator.text, text, "{source}");
        assert_eq!(operator.class, AtomClass::Binary, "{source}");
    }
    assert_eq!(atom("not").text, "¬");
    assert_eq!(atom("forall").latex, "\\forall");
    assert_eq!(atom("exists").latex, "\\exists");
}

#[test]
fn function_names_are_upright() {
    for name in [
        "sin", "cos", "tan", "arcsin", "sinh", "log", "ln", "exp", "det", "dim", "ker", "gcd",
    ] {
        let function = atom(name);
        assert_eq!(function.text, name);
        assert_eq!(function.style, AtomStyle::Upright);
        assert_eq!(function.latex, format!("\\{name}"));
    }
    assert_eq!(atom("lcm").latex, "\\operatorname{lcm}");
    assert_eq!(atom("mod").latex, "\\bmod");
}

#[test]
fn large_operators_carry_limits_flag() {
    assert_eq!(atom("sum").class, AtomClass::LargeOperator { limits: true });
    assert_eq!(
        atom("prod").class,
        AtomClass::LargeOperator { limits: true }
    );
    assert_eq!(atom("lim").class, AtomClass::LargeOperator { limits: true });
    assert_eq!(
        atom("int").class,
        AtomClass::LargeOperator { limits: false }
    );
    assert_eq!(atom("oint").text, "∮");
}

#[test]
fn inf_is_infinity_unless_followed_by_subscript() {
    assert_eq!(latex("inf"), "\\infty");
    assert_eq!(latex("inf_n a_n"), "\\inf_{n}a_{n}");
    assert_eq!(latex("infimum S"), "\\inf S");
    assert_eq!(latex("sup S"), "\\sup S");
}

#[test]
fn backslash_commands_are_accepted_for_latex_habits() {
    assert_eq!(latex("\\alpha"), "\\alpha");
    assert_eq!(latex("\\inf"), "\\inf");
    assert!(matches!(single("\\foo"), MathNode::Error { .. }));
}

#[test]
fn typed_unicode_maps_back_to_latex() {
    assert_eq!(latex("α ≤ β"), "\\alpha\\leq\\beta");
}

// ----- fractions -----------------------------------------------------------

#[test]
fn fraction_takes_the_whole_space_free_run() {
    assert_eq!(latex("a+b/c+d"), "\\frac{a+b}{c+d}");
    assert_eq!(latex("dy/dx"), "\\frac{dy}{dx}");
}

#[test]
fn spaces_end_fraction_operands() {
    assert_eq!(latex("a + b/c + d"), "a+\\frac{b}{c}+d");
}

#[test]
fn spaced_slash_divides_neighbouring_runs() {
    assert_eq!(latex("x^2 / 2"), "\\frac{x^{2}}{2}");
    assert_eq!(latex("a + b / c + d"), "a+\\frac{b}{c}+d");
}

#[test]
fn relations_end_fraction_operands_even_without_spaces() {
    assert_eq!(latex("x=1/2"), "x=\\frac{1}{2}");
}

#[test]
fn bracketed_operands_lose_their_brackets() {
    assert_eq!(latex("(a+b)/(c+d)"), "\\frac{a+b}{c+d}");
    assert_eq!(latex("[a + b]/2"), "\\frac{a+b}{2}");
}

#[test]
fn slash_binds_weaker_than_scripts_in_a_run() {
    assert_eq!(latex("x^2/2"), "\\frac{x^{2}}{2}");
    assert_eq!(latex("e^[-x^2/2]"), "e^{\\frac{-x^{2}}{2}}");
}

#[test]
fn repeated_slashes_nest_to_the_left() {
    assert_eq!(latex("a/b/c"), "\\frac{\\frac{a}{b}}{c}");
}

#[test]
fn over_makes_a_fraction_up_to_the_nearest_relation() {
    assert_eq!(latex("y = a + b over c + d"), "y=\\frac{a+b}{c+d}");
    assert_eq!(latex("(1 over 2), x"), "\\left(\\frac{1}{2}\\right),x");
}

#[test]
fn incomplete_fraction_has_empty_parts() {
    assert_eq!(latex("a/"), "\\frac{a}{}");
    assert_eq!(latex("/b"), "\\frac{}{b}");
}

// ----- scripts -------------------------------------------------------------

#[test]
fn superscript_takes_the_rest_of_the_run() {
    assert_eq!(latex("x^n+1"), "x^{n+1}");
    assert_eq!(latex("x^2 + 1"), "x^{2}+1");
    assert_eq!(latex("e^(i pi)"), "e^{i\\pi}");
}

#[test]
fn subscript_stops_at_superscript() {
    assert_eq!(latex("x_i^2"), "x_{i}^{2}");
    assert_eq!(latex("x_i+1"), "x_{i+1}");
    assert_eq!(latex("x_(i+1)"), "x_{i+1}");
}

#[test]
fn digits_after_a_variable_are_a_subscript() {
    assert_eq!(latex("x1 + a12"), "x_{1}+a_{12}");
    assert_eq!(latex("alpha2"), "\\alpha_{2}");
    assert_eq!(latex("x1^2"), "x_{1}^{2}");
    assert_eq!(latex("2x"), "2x");
    assert_eq!(latex("RR^2"), "\\mathbb{R}^{2}");
}

#[test]
fn postfix_power_words() {
    assert_eq!(latex("x sq + y cb"), "x^{2}+y^{3}");
    assert_eq!(latex("A inv"), "A^{-1}");
}

#[test]
fn postfix_power_words_bind_tighter_than_the_slash() {
    assert_eq!(latex("x sq/2"), "\\frac{x^{2}}{2}");
    assert_eq!(latex("x sq / 2"), "\\frac{x^{2}}{2}");
    assert_eq!(latex("1/x sq"), "\\frac{1}{x^{2}}");
    assert_eq!(latex("1/x sq + 1"), "\\frac{1}{x^{2}}+1");
    assert_eq!(latex("a/b cb/c"), "\\frac{\\frac{a}{b^{3}}}{c}");
    assert_eq!(latex("x sq inv/2"), "\\frac{{x^{2}}^{-1}}{2}");
}

#[test]
fn star_as_script_is_the_conjugate_star() {
    assert_eq!(latex("z^*"), "z^{*}");
    assert_eq!(latex("z***"), "z^{*}");
}

#[test]
fn double_star_is_a_superscript_like_caret() {
    assert_eq!(latex("x**2"), latex("x^2"));
    assert_eq!(latex("e**-x"), "e^{-x}");
    assert_eq!(latex("x_i**2"), "x_{i}^{2}");
    assert_eq!(latex("a**2 + b"), "a^{2}+b");
}

#[test]
fn ast_is_the_asterisk_operator() {
    assert_eq!(latex("f ast g"), "f\\ast g");
}

#[test]
fn primes_attach_as_superscripts() {
    assert_eq!(latex("f'(x)"), "f^{\\prime}\\left(x\\right)");
    assert_eq!(latex("f''"), "f^{\\prime\\prime}");
}

#[test]
fn dangling_script_has_empty_list() {
    let MathNode::Scripts { sup, sub, .. } = single("x^") else {
        panic!()
    };
    assert_eq!(sup, Some(vec![]));
    assert_eq!(sub, None);
    assert_eq!(latex("^2"), "{}^{2}");
}

// ----- large operators -----------------------------------------------------

#[test]
fn word_limits_for_sums_and_integrals() {
    assert_eq!(latex("sum i=1 to n i^2"), "\\sum_{i=1}^{n}i^{2}");
    assert_eq!(
        latex("int 0 to 1 f(x) dx"),
        "\\int_{0}^{1}f\\left(x\\right)dx"
    );
    assert_eq!(latex("prod k=1 to oo a_k"), "\\prod_{k=1}^{\\infty}a_{k}");
    assert_eq!(latex("sum x in S f(x)"), "\\sum_{x\\in S}f\\left(x\\right)");
}

#[test]
fn word_limits_for_limit_like_operators_read_to_as_arrow() {
    assert_eq!(latex("lim x to 0 f(x)"), "\\lim_{x\\to0}f\\left(x\\right)");
    assert_eq!(latex("lim x->0"), "\\lim_{x\\to0}");
    assert_eq!(latex("max x in S f"), "\\max_{x\\in S}f");
}

#[test]
fn operators_without_limit_words_keep_their_body() {
    assert_eq!(latex("sum a_n = 1"), "\\sum a_{n}=1");
    assert_eq!(latex("int f dx"), "\\int fdx");
    assert_eq!(latex("lim a_n = L"), "\\lim a_{n}=L");
}

#[test]
fn explicit_script_limits() {
    assert_eq!(latex("sum_[i=1]^n"), "\\sum_{i=1}^{n}");
    assert_eq!(latex("sum_(i=1)^n"), "\\sum_{i=1}^{n}");
    assert_eq!(latex("int_0^1"), "\\int_{0}^{1}");
    assert_eq!(latex("lim_[x->0]"), "\\lim_{x\\to0}");
}

#[test]
fn unfinished_upper_limit_is_empty() {
    assert_eq!(latex("sum i=1 to"), "\\sum_{i=1}^{}");
}

// ----- prefix words --------------------------------------------------------

#[test]
fn sqrt_takes_the_rest_of_the_run() {
    assert_eq!(latex("sqrt x+1"), "\\sqrt{x+1}");
    assert_eq!(latex("sqrt x + 1"), "\\sqrt{x}+1");
    assert_eq!(latex("sqrt(x)+1"), "\\sqrt{x}+1");
    assert_eq!(latex("1/sqrt 2"), "\\frac{1}{\\sqrt{2}}");
    assert_eq!(latex("sqrt"), "\\sqrt{}");
}

#[test]
fn roots_with_index() {
    assert_eq!(latex("root 3 x"), "\\sqrt[3]{x}");
    assert_eq!(latex("root(3, x)"), "\\sqrt[3]{x}");
    assert_eq!(latex("root(n)(x+1)"), "\\sqrt[n]{x+1}");
}

#[test]
fn fence_words() {
    assert_eq!(latex("abs x-y"), "\\left|x-y\\right|");
    assert_eq!(latex("norm v"), "\\left\\|v\\right\\|");
    assert_eq!(latex("floor x"), "\\left\\lfloor x\\right\\rfloor");
    assert_eq!(latex("ceil x"), "\\left\\lceil x\\right\\rceil");
    assert_eq!(latex("abs(x)^2"), "\\left|x\\right|^{2}");
}

#[test]
fn accents() {
    let MathNode::Accent { accent, .. } = single("hat x") else {
        panic!()
    };
    assert_eq!(accent, AccentKind::Hat);
    assert_eq!(latex("hat x"), "\\hat{x}");
    assert_eq!(latex("bar x"), "\\bar{x}");
    assert_eq!(latex("bar(x+y)"), "\\overline{x+y}");
    assert_eq!(latex("vec v"), "\\vec{v}");
    assert_eq!(latex("dot x + ddot x"), "\\dot{x}+\\ddot{x}");
    assert_eq!(latex("tilde x"), "\\tilde{x}");
}

#[test]
fn font_words() {
    let bold = atom("bb v");
    assert_eq!(
        (bold.style, bold.latex.as_str()),
        (AtomStyle::Bold, "\\mathbf{v}")
    );
    let script = atom("cal A");
    assert_eq!(
        (script.text.as_str(), script.latex.as_str()),
        ("𝒜", "\\mathcal{A}")
    );
    assert_eq!(atom("bbb R").text, "ℝ");
}

// ----- brackets ------------------------------------------------------------

#[test]
fn brackets_are_stretchy_fences() {
    assert_eq!(latex("(a+b)"), "\\left(a+b\\right)");
    assert_eq!(latex("[0, 1)"), "\\left[0,1\\right)");
    assert_eq!(
        latex("{x in RR : x > 0}"),
        "\\left\\{x\\in\\mathbb{R}:x>0\\right\\}"
    );
}

#[test]
fn unbalanced_brackets_still_parse() {
    assert_eq!(latex("(a+b"), "\\left(a+b\\right.");
    assert_eq!(latex("a)"), "a)");
    assert_eq!(latex("a}"), "a\\}");
}

// ----- matrices ------------------------------------------------------------

#[test]
fn mat_uses_spaces_between_cells_and_commas_between_rows() {
    let MathNode::Matrix { open, close, rows } = single("mat 1 2, 3 4") else {
        panic!()
    };
    assert_eq!((open.as_str(), close.as_str()), ("(", ")"));
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].len(), 2);
    assert_eq!(
        latex("mat 1 2, 3 4"),
        "\\begin{pmatrix}1 & 2 \\\\ 3 & 4\\end{pmatrix}"
    );
}

#[test]
fn matrix_variants() {
    assert_eq!(
        latex("bmat a b, c d"),
        "\\begin{bmatrix}a & b \\\\ c & d\\end{bmatrix}"
    );
    assert_eq!(
        latex("vmat a b, c d"),
        "\\begin{vmatrix}a & b \\\\ c & d\\end{vmatrix}"
    );
    assert_eq!(
        latex("A = mat[1 0, 0 1] B"),
        "A=\\begin{pmatrix}1 & 0 \\\\ 0 & 1\\end{pmatrix}B"
    );
    assert_eq!(
        latex("((1,2),(3,4))"),
        "\\begin{pmatrix}1 & 2 \\\\ 3 & 4\\end{pmatrix}"
    );
}

#[test]
fn matrix_cells_can_be_grouped() {
    assert_eq!(
        latex("mat [a + b] c"),
        "\\begin{pmatrix}a+b & c\\end{pmatrix}"
    );
}

#[test]
fn cases_split_before_prose_word() {
    let source = "cases x if x >= 0, -x otherwise";
    let MathNode::Matrix { open, close, rows } = single(source) else {
        panic!()
    };
    assert_eq!((open.as_str(), close.as_str()), ("{", ""));
    assert_eq!(rows.len(), 2);
    assert_eq!(
        latex(source),
        "\\begin{cases}x & \\text{ if }x\\geq0 \\\\ -x & \\text{ otherwise }\\end{cases}"
    );
}

#[test]
fn keyword_ending_in_digits_is_one_word() {
    assert_eq!(latex("integral2 f dA"), "\\iint fdA");
    assert_eq!(latex("alpha2"), "\\alpha_{2}");
}

#[test]
fn every_vocabulary_example_parses_without_errors() {
    let examples = vocabulary()
        .iter()
        .flat_map(|section| section.entries)
        .map(|entry| entry.example);
    for example in examples {
        let parsed = format!("{:?}", parse_math(example));
        assert!(!parsed.contains("Error {"), "{example}: {parsed}");
    }
}

#[test]
fn vocabulary_covers_every_keyword_once() {
    let spellings: Vec<&str> = vocabulary()
        .iter()
        .flat_map(|section| section.entries)
        .map(|entry| entry.spelling)
        .collect();
    let unique: std::collections::HashSet<&&str> = spellings.iter().collect();

    assert_eq!(unique.len(), spellings.len());
    for spelling in [
        "alpha",
        "<=",
        "integral2",
        "abs",
        "sqrt",
        "quad",
        "otherwise",
    ] {
        assert!(spellings.contains(&spelling), "{spelling}");
    }
}

#[test]
fn backslash_prose_word_shows_the_word_without_backslash() {
    assert_eq!(latex("x \\if y"), "x\\text{ if }y");
}

#[test]
fn cases_without_prose_split_after_first_chunk() {
    assert_eq!(
        latex("cases 1 x>0, 0 x<=0"),
        "\\begin{cases}1 & x>0 \\\\ 0 & x\\leq0\\end{cases}"
    );
}

#[test]
fn empty_matrix_keyword_has_one_empty_cell() {
    let MathNode::Matrix { rows, .. } = single("mat") else {
        panic!()
    };
    assert_eq!(rows, vec![vec![vec![]]]);
}

// ----- text and spacing ----------------------------------------------------

#[test]
fn quoted_and_backtick_text() {
    assert_eq!(
        single("\"otherwise\""),
        MathNode::Text("otherwise".to_string())
    );
    assert_eq!(single("`for all`"), MathNode::Text("for all".to_string()));
    assert_eq!(latex("`50% & more`"), "\\text{50\\% \\& more}");
    assert_eq!(
        single("\"unfinished"),
        MathNode::Text("unfinished".to_string())
    );
}

#[test]
fn explicit_spaces() {
    assert_eq!(latex("a quad b"), "a\\quad b");
    assert_eq!(latex("a \\, b"), "a\\,b");
}

#[test]
fn latex_separates_control_words_from_letters() {
    assert_eq!(latex("alpha x"), "\\alpha x");
    assert_eq!(latex("sin x"), "\\sin x");
}

#[test]
fn function_names_are_operator_atoms_without_limits() {
    for name in ["sin", "log", "det", "gcd", "lcm"] {
        assert_eq!(
            atom(name).class,
            AtomClass::LargeOperator { limits: false },
            "{name}"
        );
    }
    assert_eq!(atom("max").class, AtomClass::LargeOperator { limits: true });
}

#[test]
fn function_names_never_take_spoken_limits() {
    assert_eq!(latex("sin x=1"), "\\sin x=1");
    assert_eq!(latex("log x in RR"), "\\log x\\in\\mathbb{R}");
}

#[test]
fn quoted_text_keeps_surrounding_source_spaces() {
    let list = parse_math("a_n \"converges\" fast");
    assert_eq!(list[1], MathNode::Text(" converges ".to_string()));
    assert_eq!(single("\"alone\""), MathNode::Text("alone".to_string()));
    assert_eq!(latex("x`=`y"), "x\\text{=}y");
}
