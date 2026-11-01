//! The generator that compiles `src/math/syntax.toml` (run by `build.rs`):
//! which entries it accepts and how it reports mistakes.

#[path = "../build/syntax_tables.rs"]
mod syntax_tables;

use syntax_tables::generate;

fn error(source: &str) -> String {
    match generate(source) {
        Ok(code) => panic!("expected an error for {source:?}, got:\n{code}"),
        Err(message) => message,
    }
}

#[test]
fn shipped_syntax_file_generates() {
    let source = include_str!("../src/math/syntax.toml");
    assert!(generate(source).is_ok(), "{}", error(source));
}

#[test]
fn word_becomes_a_keyword_with_its_class() {
    let code = generate("[integral]\nintegral2 = [\"∬\", '\\iint']").unwrap();
    assert!(code.contains(
        "(\"integral2\", Keyword::Symbol(Symbol { text: \"∬\", latex: \"\\\\iint\", \
         class: AtomClass::LargeOperator { limits: false }, style: AtomStyle::Upright })),"
    ));
}

#[test]
fn operator_spellings_are_sorted_longest_first_for_the_lexer() {
    let code = generate(
        "[relation]\n\"<\" = [\"<\", \"<\"]\n\"<=\" = [\"≤\", '\\leq']\n\"<=>\" = [\"⇔\", '\\iff']",
    )
    .unwrap();
    assert!(code.contains("MULTI_CHAR_OPERATORS: [&str; 2] = [\"<=>\", \"<=\"];"));
    assert!(code.contains("SINGLE_CHAR_OPERATORS: &str = \"<\";"));
}

#[test]
fn structure_words_map_to_their_keywords() {
    let code = generate(
        "[fence]\nset = [\"{\", \"}\"]\n[accent]\nvv = \"vector\"\n\
         [space]\nqqq = { width = \"quad\", count = 3 }\n[prose]\now = \"otherwise\"",
    )
    .unwrap();
    assert!(code.contains("(\"set\", Keyword::Fence { open: \"{\", close: \"}\" }),"));
    assert!(code.contains("(\"vv\", Keyword::Accent(AccentKind::Vector)),"));
    assert!(code.contains("(\"qqq\", Keyword::Space { width: SpaceWidth::Quad, count: 3 }),"));
    assert!(code.contains("(\"ow\", Keyword::Prose(\"otherwise\")),"));
}

#[test]
fn vocabulary_lists_sections_with_titles_and_examples_in_file_order() {
    let code = generate(
        "[fence]\nabs = [\"|\", \"|\"]\n[integral]\nint = [\"∫\", '\\int']\n\
         [postfix]\nsq = [\"2\", \"2\"]\n[matrix]",
    )
    .unwrap();
    let vocabulary = &code[code.find("static VOCABULARY").unwrap()..];
    let fences = vocabulary.find("title: \"Fences\"").unwrap();
    let integrals = vocabulary.find("title: \"Integrals\"").unwrap();

    assert!(fences < integrals);
    assert!(vocabulary.contains("VocabularyEntry { spelling: \"abs\", example: \"abs x\" },"));
    assert!(vocabulary.contains("VocabularyEntry { spelling: \"int\", example: \"int\" },"));
    assert!(vocabulary.contains("VocabularyEntry { spelling: \"sq\", example: \"x sq\" },"));
    assert!(
        !vocabulary.contains("Matrices"),
        "empty sections are left out"
    );
}

#[test]
fn spelling_defined_in_two_sections_is_rejected() {
    let message = error("[letter]\nnu = [\"ν\", '\\nu']\n[relation]\nnu = [\"∋\", '\\ni']");
    assert_eq!(message, "[relation] `nu`: is defined twice");
}

#[test]
fn unknown_section_lists_the_valid_ones() {
    let message = error("[integrals]\nintegral2 = [\"∬\", '\\iint']");
    assert!(message.starts_with("[integrals] is not a section; expected one of letter, symbol,"));
}

#[test]
fn symbol_needs_text_and_latex() {
    let message = error("[integral]\nintegral2 = \"∬\"");
    assert_eq!(
        message,
        "[integral] `integral2`: expected [\"Unicode text\", 'LaTeX']"
    );
    assert!(error("[symbol]\nnothing = [\"\", \"x\"]").ends_with("must not be empty"));
}

#[test]
fn word_with_digits_before_letters_is_rejected() {
    let message = error("[integral]\nint2x = [\"∬\", '\\iint']");
    assert!(message.starts_with("[integral] `int2x`: not a valid spelling"));
}

#[test]
fn operator_starting_with_a_bracket_or_lone_slash_is_rejected() {
    assert!(error("[relation]\n\"(=\" = [\"x\", \"x\"]").contains("not a valid spelling"));
    assert!(error("[binary]\n\"/\" = [\"/\", \"/\"]").contains("fraction slash"));
}

#[test]
fn operator_spelling_in_a_large_operator_section_is_rejected() {
    let message = error("[large-operator]\n\"++\" = [\"∑\", '\\sum']");
    assert!(message.contains("operator spellings only go in"));
}

#[test]
fn structure_sections_only_take_words() {
    let message = error("[fence]\n\"||\" = [\"‖\", \"‖\"]");
    assert!(message.contains("only takes words"));
}

#[test]
fn unknown_accent_name_lists_the_valid_ones() {
    let message = error("[accent]\narrow = \"arrow\"");
    assert_eq!(
        message,
        "[accent] `arrow`: expected one of \"hat\", \"bar\", \"vector\", \"dot\", \
         \"double-dot\", \"tilde\""
    );
}

#[test]
fn space_needs_a_positive_count() {
    let message = error("[space]\nnone = { width = \"quad\", count = 0 }");
    assert!(message.contains("`count` must be a whole number of at least 1"));
}

#[test]
fn toml_syntax_error_is_reported_with_its_location() {
    let message = error("[integral]\nintegral2 = [\"∬\"");
    assert!(message.contains("line 2"), "{message}");
}
