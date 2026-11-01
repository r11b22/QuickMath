//! The generator that compiles `src/practice/lessons.toml` (run by
//! `build.rs`): which levels it accepts and how it reports mistakes.

#[path = "../build/lesson_tables.rs"]
mod lesson_tables;

use lesson_tables::generate;

const MINIMAL_LEVEL: &str = "[[level]]\nid = \"basics\"\ntitle = \"First steps\"\n\
                             lesson = \"Type it.\"\nanswers = [\"a + b\"]\n";

fn error(source: &str) -> String {
    match generate(source) {
        Ok(code) => panic!("expected an error for {source:?}, got:\n{code}"),
        Err(message) => message,
    }
}

#[test]
fn shipped_lessons_file_generates() {
    let source = include_str!("../src/practice/lessons.toml");
    assert!(generate(source).is_ok(), "{}", error(source));
}

#[test]
fn level_becomes_a_rust_entry_with_its_generator() {
    let code = generate(&format!("{MINIMAL_LEVEL}tips = [\"a * b\"]")).unwrap();

    assert!(code.contains("id: \"basics\","));
    assert!(code.contains("tips: &[\"a * b\"],"));
    assert!(code.contains("answers: &[\"a + b\"],"));
    assert!(code.contains("generate: generators::basics,"));
}

#[test]
fn tips_are_optional() {
    let code = generate(MINIMAL_LEVEL).unwrap();

    assert!(code.contains("tips: &[],"));
}

#[test]
fn levels_keep_file_order() {
    let second = MINIMAL_LEVEL.replace("basics", "scripts");
    let code = generate(&format!("{second}{MINIMAL_LEVEL}")).unwrap();

    assert!(code.find("\"scripts\"").unwrap() < code.find("\"basics\"").unwrap());
}

#[test]
fn quotes_and_backslashes_are_escaped() {
    let code = generate(&MINIMAL_LEVEL.replace("\"a + b\"", r#"'\alpha "x"'"#)).unwrap();

    assert!(code.contains(r#"answers: &["\\alpha \"x\""],"#));
}

#[test]
fn duplicate_id_is_an_error() {
    let message = error(&format!("{MINIMAL_LEVEL}{MINIMAL_LEVEL}"));

    assert!(
        message.contains("level 2: duplicate id `basics`"),
        "{message}"
    );
}

#[test]
fn id_that_is_not_a_rust_name_is_an_error() {
    let message = error(&MINIMAL_LEVEL.replace("\"basics\"", "\"First-steps\""));

    assert!(
        message.contains("id `First-steps` must be lowercase"),
        "{message}"
    );
}

#[test]
fn missing_key_names_the_level() {
    let message = error(&MINIMAL_LEVEL.replace("title = \"First steps\"\n", ""));

    assert!(
        message.contains("level 1: `basics`: missing `title`"),
        "{message}"
    );
}

#[test]
fn empty_answers_are_an_error() {
    let message = error(&MINIMAL_LEVEL.replace("[\"a + b\"]", "[]"));

    assert!(message.contains("`answers` must not be empty"), "{message}");
}

#[test]
fn unknown_key_is_an_error() {
    let message = error(&format!("{MINIMAL_LEVEL}hint = \"x\""));

    assert!(message.contains("unknown key `hint`"), "{message}");
}

#[test]
fn non_string_tip_is_an_error() {
    let message = error(&format!("{MINIMAL_LEVEL}tips = [1]"));

    assert!(
        message.contains("`tips` must be a list of non-empty strings"),
        "{message}"
    );
}

#[test]
fn file_without_levels_is_an_error() {
    assert!(error("").contains("at least one [[level]]"));
    assert!(error("[level]\nid = \"x\"").contains("at least one [[level]]"));
}
