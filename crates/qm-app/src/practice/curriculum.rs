//! The practice levels, from single operators up to lecture-speed formulas.
//! Each exercise is a reference answer; the formula the player sees is that
//! answer rendered, and any input that renders the same counts as correct.

use super::generators::{self, ExerciseGenerator};

/// One level of the practice game.
pub struct Level {
    /// Stable key for saved progress. Never change it once released.
    pub id: &'static str,
    pub title: &'static str,
    /// The idea this level teaches, in one or two sentences.
    pub lesson: &'static str,
    /// Example inputs shown (with their rendering) above the exercises.
    pub tips: &'static [&'static str],
    /// Reference answers, easiest first.
    pub answers: &'static [&'static str],
    /// Makes random formulas with this level's notation for the sprint.
    pub generate: ExerciseGenerator,
}

pub const LEVELS: &[Level] = &[
    Level {
        id: "basics",
        title: "First steps",
        lesson: "Type a formula the way you would say it. Spaces never show up, \
                 and most symbols have a short spelling without Shift.",
        tips: &["2x + 1", "a * b", "3 xx 4", "x +- 1", "a <= b", "x /= 0"],
        answers: &[
            "a + b",
            "2x + 1",
            "x - y = 3",
            "a * b",
            "3 xx 4",
            "x +- 1",
            "a <= b",
            "x /= 0",
            "a >= b > c",
            "x ~= 3.14",
        ],
        generate: generators::basics,
    },
    Level {
        id: "scripts",
        title: "Powers and indices",
        lesson: "^ and _ take everything up to the next space, so you never \
                 go back to add brackets. Digits right after a letter are an index.",
        tips: &["x^n+1", "x^2 + 1", "a_n+1", "x1", "x sq", "A inv"],
        answers: &[
            "x^2",
            "x^n",
            "x^2 + 1",
            "x^n+1",
            "e^-x",
            "x_i",
            "a_n+1",
            "x1 + x2",
            "x_i^2",
            "x sq + y sq = r sq",
            "A inv",
            "a^m+n = a^m a^n",
        ],
        generate: generators::scripts,
    },
    Level {
        id: "fractions",
        title: "Fractions",
        lesson: "/ splits a run of math without spaces, so a+b/c+d is a whole \
                 fraction. Spaces keep the slash local. over divides everything \
                 back to the last = or comma.",
        tips: &["a+b/c+d", "a + b/c", "dy/dx", "1 + x over 1 - x"],
        answers: &[
            "1/2",
            "a/b",
            "dy/dx",
            "x^2/2",
            "a+b/c+d",
            "a + b/c",
            "1/x + 1/y",
            "y = 1 + x over 1 - x",
            "a/b + c/d = a d + b c over b d",
        ],
        generate: generators::fractions,
    },
    Level {
        id: "roots",
        title: "Roots and brackets",
        lesson: "sqrt takes the rest of the run, like ^. Brackets [ ] group \
                 without Shift and disappear when they wrap a whole operand.",
        tips: &["sqrt x+1", "sqrt x + 1", "root 3 x", "abs x", "e^[-x^2/2]"],
        answers: &[
            "sqrt 2",
            "sqrt x+1",
            "sqrt x + 1",
            "1/sqrt 2",
            "root 3 x",
            "(a+b)^2",
            "abs x",
            "norm v",
            "[0, 1)",
            "e^[-x^2/2]",
            "sqrt[a^2 + b^2]",
        ],
        generate: generators::roots,
    },
    Level {
        id: "greek",
        title: "Greek and special letters",
        lesson: "Greek letters are their names. Put a space between a keyword \
                 and a variable: 2 pi r, not 2pir.",
        tips: &[
            "alpha beta",
            "2 pi r",
            "Delta x",
            "eps",
            "oo",
            "del",
            "grad",
        ],
        answers: &[
            "alpha + beta",
            "2 pi r",
            "Delta x",
            "lambda1",
            "eps > 0",
            "omega t + phi",
            "x -> oo",
            "[del f]/[del x]",
            "grad f = 0",
            "sigma sq = E[X sq] - mu sq",
        ],
        generate: generators::greek,
    },
    Level {
        id: "functions",
        title: "Functions",
        lesson: "Function names like sin, log and det are upright words. \
                 Scripts on them work as usual: log_2 n, sin^2 x.",
        tips: &["sin x", "log_2 n", "f'(x)", "e^[i pi]"],
        answers: &[
            "sin x",
            "ln x",
            "log_2 n",
            "f(x) = x^2",
            "f'(x)",
            "det A",
            "e^[i pi] = -1",
            "sin^2 x + cos^2 x = 1",
            "exp(ln x) = x",
            "tan x = sin x over cos x",
        ],
        generate: generators::functions,
    },
    Level {
        id: "sets",
        title: "Sets and logic",
        lesson: "Sets and logic are words too: in, sub, cup, cap, forall, \
                 exists. Double capitals give number sets: RR, NN, ZZ.",
        tips: &[
            "x in RR",
            "A cup B",
            "forall",
            "exists",
            "f: A to B",
            "x |-> x^2",
        ],
        answers: &[
            "x in RR",
            "NN sub ZZ sub QQ",
            "A cup B",
            "A cap B = empty",
            "x !in A",
            "f: A to B",
            "x |-> x^2",
            "forall eps > 0",
            "exists x in RR",
            "{x in RR : x > 0}",
            "A sube B <=> A cap B = A",
        ],
        generate: generators::sets,
    },
    Level {
        id: "accents",
        title: "Accents and fonts",
        lesson: "Accents and fonts are prefix words: hat, bar, vec, dot, bb, \
                 cal. Attach brackets for a wide accent: hat(x+y).",
        tips: &["hat x", "bar z", "vec v", "bb v", "cal A", "hat(x+y)"],
        answers: &[
            "hat x",
            "bar z",
            "vec v",
            "bb v",
            "cal A",
            "dot x = -k x",
            "hat(x+y)",
            "z bar z = abs z sq",
            "vec F = m vec a",
        ],
        generate: generators::accents,
    },
    Level {
        id: "operators",
        title: "Sums and integrals",
        lesson: "Say the limits: sum i=1 to n, int 0 to 1. A chunk with in \
                 becomes the lower limit: sum x in S.",
        tips: &["sum i=1 to n", "int 0 to 1 f(x) dx", "sum x in S"],
        answers: &[
            "sum i=1 to n i",
            "sum i=1 to n i^2",
            "prod k=1 to n k",
            "int 0 to 1 x dx",
            "sum x in S f(x)",
            "sum n=0 to oo q^n",
            "bigcup i=1 to n A_i",
            "int 0 to oo e^-x dx = 1",
            "sum k=1 to n k = n(n+1)/2",
        ],
        generate: generators::operators,
    },
    Level {
        id: "limits",
        title: "Limits and extrema",
        lesson: "lim, max, sup and friends take a spoken limit: lim x to 0. \
                 Use inf_n for the infimum, since inf alone is infinity.",
        tips: &["lim x to 0", "max x in S", "inf_n a_n"],
        answers: &[
            "lim x to 0 x = 0",
            "lim n to oo 1/n = 0",
            "max x in S f(x)",
            "inf_n a_n",
            "lim x to 0 (sin x)/x = 1",
            "sup x in A x <= M",
            "lim h to 0 [f(x+h) - f(x)]/h",
        ],
        generate: generators::limits,
    },
    Level {
        id: "matrices",
        title: "Matrices and cases",
        lesson: "Spaces separate cells and commas separate rows. Use \
                 mat[…] when more math follows on the same line.",
        tips: &[
            "mat 1 2, 3 4",
            "bmat a b, c d",
            "vmat a b, c d",
            "cases x if x >= 0, -x otherwise",
        ],
        answers: &[
            "mat 1 2, 3 4",
            "bmat a b, c d",
            "vmat a b, c d",
            "A = mat[1 0, 0 1]",
            "mat 1 0 0, 0 1 0, 0 0 1",
            "cases x if x >= 0, -x otherwise",
            "f(x) = cases 1 if x in QQ, 0 otherwise",
            "A inv = 1/[a d - b c] bmat d -b, -c a",
        ],
        generate: generators::matrices,
    },
    Level {
        id: "lecture",
        title: "Lecture speed",
        lesson: "Whole formulas as they appear on the blackboard. Read the \
                 formula once, then type it in one go.",
        tips: &[],
        answers: &[
            "e^[i pi] + 1 = 0",
            "(a+b)^2 = a^2 + 2a b + b^2",
            "sum n=0 to oo q^n = 1 over 1 - q",
            "x = -b +- sqrt[b^2 - 4a c] over 2a",
            "int a to b f'(x) dx = f(b) - f(a)",
            "abs x = cases x if x >= 0, -x otherwise",
            "det(A - lambda I) = 0",
            "norm x+y <= norm x + norm y",
            "P(A mid B) = P(A cap B) over P(B)",
            "forall eps > 0 exists delta > 0 : abs x-a < delta => abs f(x)-f(a) < eps",
        ],
        generate: generators::lecture,
    },
];

/// The level after `index`, if any.
pub fn next_level(index: usize) -> Option<usize> {
    (index + 1 < LEVELS.len()).then_some(index + 1)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn contains_error(math: &[qm_format::MathNode]) -> bool {
        format!("{math:?}").contains("Error {")
    }

    #[test]
    fn every_answer_and_tip_parses_without_errors() {
        for level in LEVELS {
            for input in level.answers.iter().chain(level.tips) {
                let math = qm_format::parse_math(input);
                assert!(
                    !contains_error(&math),
                    "{:?} in level {:?} does not parse",
                    input,
                    level.id
                );
            }
        }
    }

    #[test]
    fn level_ids_are_unique_and_levels_are_not_empty() {
        let mut ids = HashSet::new();
        for level in LEVELS {
            assert!(ids.insert(level.id), "duplicate level id {:?}", level.id);
            assert!(!level.answers.is_empty(), "level {:?} is empty", level.id);
        }
    }

    #[test]
    fn next_level_stops_at_the_last_level() {
        assert_eq!(next_level(0), Some(1));
        assert_eq!(next_level(LEVELS.len() - 1), None);
    }
}
