//! The parser runs on every keystroke: it must never panic, hang or blow the
//! stack, whatever half-typed input it sees.

use std::time::{Duration, Instant};

use qm_format::{
    display_math_to_latex, highlight, math_to_latex, parse_document, parse_math, BlockKind,
};

/// Deterministic pseudo-random generator (no external crates, reproducible).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

const FRAGMENTS: &[&str] = &[
    ";;", ";", "$", "\\", "^", "_", "/", "(", ")", "[", "]", "{", "}", "[[", "]]", "|", "&", "\"",
    "`", "```", "*", "**", "'", " ", "  ", "\n", "\t", "- ", "1. ", "> ", "# ", "---", ".def ",
    ".proof", "sum", "int", "lim", "to", "over", "sqrt", "root", "mat", "cases", "if", "sq", "abs",
    "hat", "inf", "x", "y1", "2", "3.5", "=", "<=", "->", ",", "α", "ü", "∑", "\r\n", "alpha",
    "RR", "cal", "bb",
];

fn random_source(rng: &mut Lcg, length: usize) -> String {
    (0..length)
        .map(|_| FRAGMENTS[(rng.next() as usize) % FRAGMENTS.len()])
        .collect()
}

fn exercise(source: &str) {
    let document = parse_document(source);
    for block in &document.blocks {
        if let BlockKind::DisplayMath(math) = &block.kind {
            display_math_to_latex(math);
        }
    }
    math_to_latex(&parse_math(source));
    let spans = highlight(source);
    let covered: usize = spans.iter().map(|span| span.range.len()).sum();
    assert_eq!(covered, source.len(), "highlight coverage for {source:?}");
}

#[test]
fn random_fragment_soup_never_panics() {
    let mut rng = Lcg(42);
    for length in 0..400 {
        exercise(&random_source(&mut rng, length % 60));
    }
}

#[test]
fn every_prefix_of_the_example_note_parses() {
    let note = include_str!("../../../docs/examples/lecture.qm");
    for (index, _) in note.char_indices().step_by(7) {
        exercise(&note[..index]);
    }
}

#[test]
fn pathological_nesting_does_not_overflow() {
    let deep_math = format!("{}x{}", "(".repeat(4_000), ")".repeat(4_000));
    exercise(&deep_math);
    exercise(&"(".repeat(4_000));
    exercise(&"x^".repeat(4_000));
    exercise(&"x_".repeat(4_000));
    exercise(&"sqrt ".repeat(1_000));
    exercise(&"a/".repeat(4_000));
    exercise(&"> ".repeat(4_000));
    exercise(&"- ".repeat(4_000));
    let deep_list: String = (0..300)
        .map(|depth| format!("{}- x\n", "  ".repeat(depth)))
        .collect();
    exercise(&deep_list);
    let deep_callouts: String = (0..300)
        .map(|depth| format!("{}.note x\n", "  ".repeat(depth)))
        .collect();
    exercise(&deep_callouts);
}

#[test]
fn deep_math_reports_an_error_node() {
    let source = format!(";;{}x{};;", "[".repeat(200), "]".repeat(200));
    let text = format!("{:?}", parse_document(&source));
    assert!(text.contains("Error"));
}

#[test]
fn large_note_parses_quickly() {
    let note = include_str!("../../../docs/examples/lecture.qm");
    let repetitions = 2000 / note.lines().count().max(1) + 1;
    let big = note.repeat(repetitions);
    let started = Instant::now();
    parse_document(&big);
    highlight(&big);
    // Generous bound so the debug-mode test is not flaky; release is far faster.
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "took {:?}",
        started.elapsed()
    );
}
