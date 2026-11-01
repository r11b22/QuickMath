//! Math word completion: where suggestions appear and how they are ranked.

use qm_format::math_completions;

/// Completes at the `|` marker, which is removed from the source.
fn spellings_at_marker(marked: &str) -> Option<Vec<&'static str>> {
    let cursor = marked.find('|').expect("cursor marker");
    let source = marked.replacen('|', "", 1);
    let completions = math_completions(&source, cursor)?;
    Some(
        completions
            .candidates
            .iter()
            .map(|candidate| candidate.entry.spelling)
            .collect(),
    )
}

#[test]
fn suggests_keywords_in_unclosed_inline_math() {
    let spellings = spellings_at_marker("Let ;;alp|").expect("suggestions");
    assert_eq!(spellings[0], "alpha");
}

#[test]
fn range_covers_the_typed_word() {
    let completions = math_completions("Let ;;x + alp;; be", 13).expect("suggestions");
    assert_eq!(completions.range, 10..13);
}

#[test]
fn suggests_in_display_math() {
    let spellings = spellings_at_marker(";; x = sqr|").expect("suggestions");
    assert_eq!(spellings, ["sqrt"]);
}

#[test]
fn suggests_backslash_words() {
    let spellings = spellings_at_marker(";;\\alp|").expect("suggestions");
    assert_eq!(spellings[0], "alpha");
}

#[test]
fn no_suggestions_outside_math() {
    assert_eq!(spellings_at_marker("Some alp|"), None);
    assert_eq!(spellings_at_marker("`;;alp|`"), None);
    assert_eq!(spellings_at_marker("```\n;; alp|\n```"), None);
}

#[test]
fn no_suggestions_inside_quoted_text_in_math() {
    assert_eq!(spellings_at_marker(";;x \"alp|"), None);
}

#[test]
fn no_suggestions_for_single_letters() {
    assert_eq!(spellings_at_marker(";;a|"), None);
}

#[test]
fn no_suggestions_inside_a_word() {
    assert_eq!(spellings_at_marker(";;al|pha"), None);
}

#[test]
fn no_suggestions_when_only_the_typed_word_matches() {
    assert_eq!(spellings_at_marker(";;alpha|"), None);
}

#[test]
fn exact_word_is_listed_first_when_longer_words_also_match() {
    let spellings = spellings_at_marker(";;sq|").expect("suggestions");
    assert_eq!(spellings, ["sq", "sqrt"]);
}

#[test]
fn shorter_spellings_come_first() {
    let spellings = spellings_at_marker(";;in|").expect("suggestions");
    assert_eq!(spellings[0], "in");
    let lengths: Vec<usize> = spellings.iter().map(|s| s.len()).collect();
    assert!(
        lengths.windows(2).all(|pair| pair[0] <= pair[1]),
        "{spellings:?}"
    );
}

#[test]
fn aliases_of_one_symbol_are_listed_once_under_the_shortest() {
    let spellings = spellings_at_marker(";;ep|").expect("suggestions");
    assert!(spellings.contains(&"eps"), "{spellings:?}");
    assert!(!spellings.contains(&"epsilon"), "{spellings:?}");
}

#[test]
fn letters_in_order_with_gaps_match_after_prefixes() {
    let spellings = spellings_at_marker(";;sqt|").expect("suggestions");
    assert_eq!(spellings, ["sqrt"]);
}

#[test]
fn candidates_come_with_their_section_and_example() {
    let completions = math_completions(";;sqr", 5).expect("suggestions");
    let candidate = completions.candidates[0];
    assert_eq!(candidate.entry.example, "sqrt x");
    assert!(!candidate.section.is_empty());
}

#[test]
fn bad_cursors_never_panic() {
    assert_eq!(math_completions(";;αβ", 3), None);
    assert_eq!(math_completions(";;alp", 99), None);
    assert_eq!(math_completions("", 0), None);
}
