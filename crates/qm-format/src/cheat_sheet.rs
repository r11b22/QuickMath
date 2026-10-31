//! Short examples for the editor's help panel. Keep in sync with the cheat
//! sheet table in `docs/FORMAT.md`.

use crate::{CheatSheetEntry, CheatSheetKind};

const fn entry(
    input: &'static str,
    description: &'static str,
    kind: CheatSheetKind,
) -> CheatSheetEntry {
    CheatSheetEntry {
        input,
        description,
        kind,
    }
}

/// Help-panel entries; `Math` inputs are exactly what goes between `;;`.
pub(crate) const CHEAT_SHEET: &[CheatSheetEntry] = &[
    entry(
        "# Title",
        "Heading (## to ###### for smaller)",
        CheatSheetKind::Markup,
    ),
    entry(
        "*word*  **word**",
        "Emphasis and strong",
        CheatSheetKind::Markup,
    ),
    entry(
        "- item  /  1. item",
        "Bullet / numbered list (indent 2 to nest)",
        CheatSheetKind::Markup,
    ),
    entry(
        "[[Other note|label]]",
        "Link to another note",
        CheatSheetKind::Markup,
    ),
    entry(
        ".def Group",
        "Definition box; also .thm .lem .cor .proof .ex .note .warn",
        CheatSheetKind::Markup,
    ),
    entry(";;x^2;;", "Inline math inside text", CheatSheetKind::Markup),
    entry(
        ";; f(x) = x^2",
        "Display math line; consecutive lines align at the first =",
        CheatSheetKind::Markup,
    ),
    entry(
        ";; = 2x + 1",
        "Continue a derivation (row starting with a relation)",
        CheatSheetKind::Markup,
    ),
    entry(
        "a+b/c+d",
        "Fraction of the space-free run: (a+b)/(c+d)",
        CheatSheetKind::Math,
    ),
    entry(
        "a + b/c",
        "Spaces end the operand: a + b⁄c",
        CheatSheetKind::Math,
    ),
    entry(
        "a + b over c + d",
        "Big fraction up to the nearest = , or bracket",
        CheatSheetKind::Math,
    ),
    entry(
        "x^n+1  x_i+1",
        "Script takes the rest of the run: xⁿ⁺¹, xᵢ₊₁",
        CheatSheetKind::Math,
    ),
    entry("x_i^2", "Subscript stops at ^", CheatSheetKind::Math),
    entry(
        "x1  a12",
        "Digits right after a variable become a subscript",
        CheatSheetKind::Math,
    ),
    entry("x sq  x cb  A inv", "x², x³, A⁻¹", CheatSheetKind::Math),
    entry(
        "[n+1]  (n+1)",
        "Brackets around a whole operand are invisible: x^[n+1]",
        CheatSheetKind::Math,
    ),
    entry(
        "[0, 1)",
        "Brackets elsewhere are shown; any closer may close",
        CheatSheetKind::Math,
    ),
    entry(
        "sqrt x+1",
        "Square root of the rest of the run",
        CheatSheetKind::Math,
    ),
    entry("root 3 x", "n-th root", CheatSheetKind::Math),
    entry(
        "abs x  norm v",
        "|x|, ‖v‖ (also floor, ceil, ang)",
        CheatSheetKind::Math,
    ),
    entry(
        "hat x  bar x  vec v",
        "Accents (also dot, ddot, tilde)",
        CheatSheetKind::Math,
    ),
    entry(
        "bb v  cal A  bbb R",
        "Bold, calligraphic, blackboard letters",
        CheatSheetKind::Math,
    ),
    entry(
        "RR NN ZZ QQ CC",
        "Number sets ℝ ℕ ℤ ℚ ℂ",
        CheatSheetKind::Math,
    ),
    entry(
        "alpha beta Gamma",
        "Greek letters by name (eps, phi, …)",
        CheatSheetKind::Math,
    ),
    entry("oo  del  grad", "∞, ∂, ∇", CheatSheetKind::Math),
    entry(
        "sum i=1 to n",
        "Sum with limits (also prod, bigcup)",
        CheatSheetKind::Math,
    ),
    entry(
        "int 0 to 1 f(x) dx",
        "Integral with limits (also iint, oint)",
        CheatSheetKind::Math,
    ),
    entry(
        "lim x to 0",
        "Limit (also max x in S, sup, infimum)",
        CheatSheetKind::Math,
    ),
    entry(
        "sum_[i=1]^n",
        "Explicit limits with _ and ^",
        CheatSheetKind::Math,
    ),
    entry(
        "sin x  log_2 n  ln x",
        "Upright function names",
        CheatSheetKind::Math,
    ),
    entry(
        "<= >= /= ~= ==",
        "≤ ≥ ≠ ≈ ≡ (also le ge ne)",
        CheatSheetKind::Math,
    ),
    entry(
        "-> => <=> |->",
        "→ ⇒ ⇔ ↦ (also to, implies, iff)",
        CheatSheetKind::Math,
    ),
    entry("in !in sub sube", "∈ ∉ ⊂ ⊆", CheatSheetKind::Math),
    entry("xx  *  pm  cdot", "× ⋅ ± ⋅", CheatSheetKind::Math),
    entry("cup cap and or not", "∪ ∩ ∧ ∨ ¬", CheatSheetKind::Math),
    entry("forall exists", "∀ ∃", CheatSheetKind::Math),
    entry("... cdots", "… ⋯", CheatSheetKind::Math),
    entry(
        "mat 1 2, 3 4",
        "Matrix: spaces between cells, commas between rows",
        CheatSheetKind::Math,
    ),
    entry(
        "bmat a b, c d",
        "Matrix in square brackets (vmat for a determinant)",
        CheatSheetKind::Math,
    ),
    entry(
        "cases x if x >= 0, -x otherwise",
        "Piecewise definition",
        CheatSheetKind::Math,
    ),
    entry(
        "`any text`",
        "Upright text inside math (also \"text\")",
        CheatSheetKind::Math,
    ),
    entry("f'(x)", "Prime", CheatSheetKind::Math),
    entry("\\;;", "A literal ;; in text", CheatSheetKind::Markup),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_math, MathNode};

    fn contains_error(list: &[MathNode]) -> bool {
        format!("{list:?}").contains("Error {")
    }

    #[test]
    fn every_math_entry_parses_without_errors() {
        for entry in CHEAT_SHEET
            .iter()
            .filter(|entry| entry.kind == CheatSheetKind::Math)
        {
            assert!(!contains_error(&parse_math(entry.input)), "{}", entry.input);
        }
    }

    #[test]
    fn markup_entries_are_not_bare_math() {
        assert!(CHEAT_SHEET
            .iter()
            .any(|entry| entry.kind == CheatSheetKind::Markup));
        assert!(CHEAT_SHEET.len() >= 30);
    }
}
