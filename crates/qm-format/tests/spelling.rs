//! Which words spell checking looks at.

use qm_format::spell_check_words;

fn words(source: &str) -> Vec<&str> {
    spell_check_words(source)
        .into_iter()
        .map(|range| &source[range])
        .collect()
}

#[test]
fn checks_prose_in_paragraphs_headings_and_lists() {
    assert_eq!(
        words("# Groeps theory\n\nSome *emphasis* and **strong** text.\n\n- itemm one"),
        ["Groeps", "theory", "Some", "emphasis", "and", "strong", "text", "itemm", "one"]
    );
}

#[test]
fn checks_callout_titles_but_not_the_marker() {
    assert_eq!(
        words(".def germs\nI don't know"),
        ["germs", "don't", "know"]
    );
}

#[test]
fn never_checks_math() {
    assert_eq!(
        words("Let ;;alpha + beta;; be small"),
        ["Let", "be", "small"]
    );
    assert_eq!(
        words(";; f(x) = cases x if x = 5, 0 otherwise;;"),
        Vec::<&str>::new()
    );
}

#[test]
fn never_checks_code_wikilinks_or_front_matter() {
    let source = "---\ntitle: Lecteur\n---\nSee [[Limitz]] and `foo bar`\n\n```\nlet qux\n```";
    assert_eq!(words(source), ["See", "and"]);
}

#[test]
fn skips_variables_abbreviations_and_web_addresses() {
    assert_eq!(
        words("Let f be a PDE in x2, see https://example.org/pagge now"),
        ["Let", "be", "in", "see", "now"]
    );
}

#[test]
fn ranges_are_byte_offsets_into_the_source() {
    let source = "αβ café";
    assert_eq!(spell_check_words(source), [0..4, 5..10]);
}

#[test]
fn empty_and_whitespace_sources_have_no_words() {
    assert!(spell_check_words("").is_empty());
    assert!(spell_check_words(" \n\t\n").is_empty());
}
