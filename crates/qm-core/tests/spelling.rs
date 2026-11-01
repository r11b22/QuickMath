//! Spell checking: finding languages, checking in several at once, and the
//! personal word list.

use std::fs;
use std::path::{Path, PathBuf};

use qm_core::{CoreError, Language, SpellChecker, WordList};

/// A tiny Hunspell dictionary: `group` takes an `s` suffix.
const TINY_AFF: &str = "SET UTF-8\nSFX S Y 1\nSFX S 0 s .\n";
const TINY_DIC: &str = "2\ngroup/S\nlemma\n";

fn write_dictionary(folder: &Path, code: &str, aff: &str, dic: &str) {
    fs::create_dir_all(folder).unwrap();
    fs::write(folder.join(format!("{code}.aff")), aff).unwrap();
    fs::write(folder.join(format!("{code}.dic")), dic).unwrap();
}

fn codes(languages: &[Language]) -> Vec<&str> {
    languages.iter().map(Language::code).collect()
}

fn find(languages: &[Language], code: &str) -> Language {
    languages
        .iter()
        .find(|language| language.code() == code)
        .unwrap_or_else(|| panic!("{code} should be available"))
        .clone()
}

fn checker_for(languages: &[Language], word_list: &WordList) -> SpellChecker {
    let dictionaries = languages
        .iter()
        .map(|language| language.load().unwrap())
        .collect();
    let (checker, errors) = SpellChecker::new(dictionaries, word_list);
    assert!(errors.is_empty(), "{errors:?}");
    checker
}

fn tiny_checker(folder: &Path, word_list: &WordList) -> SpellChecker {
    write_dictionary(folder, "xx_TINY", TINY_AFF, TINY_DIC);
    let tiny = find(&Language::available_in(&[folder.to_path_buf()]), "xx_TINY");
    checker_for(&[tiny], word_list)
}

#[test]
fn english_and_dutch_are_built_in() {
    let languages = Language::available_in(&[]);
    assert_eq!(codes(&languages), ["en_US", "nl_NL"]);
}

#[test]
fn dictionary_pairs_in_folders_become_languages() {
    let dir = tempfile::tempdir().unwrap();
    write_dictionary(dir.path(), "de_DE", TINY_AFF, TINY_DIC);
    // A `.dic` without `.aff` (a hyphenation file) is not a dictionary.
    fs::write(dir.path().join("hyph_de_DE.dic"), "").unwrap();

    let languages = Language::available_in(&[dir.path().to_path_buf()]);

    assert_eq!(codes(&languages), ["de_DE", "en_US", "nl_NL"]);
}

#[test]
fn files_override_built_in_dictionaries_and_earlier_folders_win() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first");
    let second = dir.path().join("second");
    write_dictionary(&first, "en_US", TINY_AFF, "1\nfirstword\n");
    write_dictionary(&second, "en_US", TINY_AFF, "1\nsecondword\n");

    let languages = Language::available_in(&[first, second]);
    let checker = checker_for(&[find(&languages, "en_US")], &WordList::default());

    assert!(checker.is_correct("firstword"));
    assert!(!checker.is_correct("secondword"));
    assert!(!checker.is_correct("hello"));
}

#[test]
fn missing_folders_add_no_languages() {
    let languages = Language::available_in(&[PathBuf::from("/definitely/not/here")]);
    assert_eq!(codes(&languages), ["en_US", "nl_NL"]);
}

#[test]
fn checks_with_affix_rules_and_hunspell_casing() {
    let dir = tempfile::tempdir().unwrap();
    let checker = tiny_checker(dir.path(), &WordList::default());

    assert!(checker.is_correct("group"));
    assert!(checker.is_correct("groups"));
    assert!(checker.is_correct("Groups"));
    assert!(checker.is_correct("LEMMA"));
    assert!(!checker.is_correct("lemmas"));
    assert!(!checker.is_correct("gruop"));
}

#[test]
fn suggests_corrections() {
    let dir = tempfile::tempdir().unwrap();
    let checker = tiny_checker(dir.path(), &WordList::default());
    assert_eq!(
        checker.suggestions("gruop").first().map(String::as_str),
        Some("group")
    );
}

#[test]
fn accepts_words_from_the_word_list_in_every_casing() {
    let dir = tempfile::tempdir().unwrap();
    let mut word_list = WordList::load_from(&dir.path().join("words.txt"));
    word_list.add("eigenbasis").unwrap();

    let checker = tiny_checker(&dir.path().join("dicts"), &word_list);

    assert!(checker.is_correct("eigenbasis"));
    assert!(checker.is_correct("Eigenbasis"));
}

#[test]
fn accept_adds_a_word_to_a_running_checker() {
    let dir = tempfile::tempdir().unwrap();
    let mut checker = tiny_checker(dir.path(), &WordList::default());
    assert!(!checker.is_correct("Banach"));

    checker.accept("Banach").unwrap();

    assert!(checker.is_correct("Banach"));
}

#[test]
fn a_checker_without_dictionaries_accepts_everything() {
    let (checker, errors) = SpellChecker::new(Vec::new(), &WordList::default());
    assert!(errors.is_empty());
    assert!(checker.is_correct("asdfgh"));
    assert!(checker.suggestions("asdfgh").is_empty());
}

#[test]
fn mixed_language_notes_need_no_switching() {
    let languages = Language::available_in(&[]);
    let checker = checker_for(
        &[find(&languages, "en_US"), find(&languages, "nl_NL")],
        &WordList::default(),
    );

    assert_eq!(checker.languages().collect::<Vec<_>>(), ["en_US", "nl_NL"]);
    for word in [
        "theorem",
        "don't",
        "well-known",
        "stelling",
        "wiskunde",
        "hallo",
    ] {
        assert!(checker.is_correct(word), "{word} should be correct");
    }
    assert!(!checker.is_correct("integrall"));
    let suggestions = checker.suggestions("integrall");
    assert!(
        suggestions.contains(&"integral".to_string()),
        "{suggestions:?}"
    );
}

#[test]
fn dictionaries_that_are_not_utf8_are_reported() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("fr_FR.aff"), b"SET ISO8859-1\n").unwrap();
    fs::write(dir.path().join("fr_FR.dic"), b"1\nd\xe9j\xe0\n").unwrap();
    let languages = Language::available_in(&[dir.path().to_path_buf()]);

    let error = find(&languages, "fr_FR")
        .load()
        .err()
        .expect("load should fail");

    assert!(
        matches!(&error, CoreError::Dictionary { language, .. } if language == "fr_FR"),
        "{error:?}"
    );
    assert!(error.to_string().contains("UTF-8"), "{error}");
}
