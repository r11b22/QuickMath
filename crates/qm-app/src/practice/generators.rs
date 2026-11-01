//! Random exercises for the sprint, one generator per level. Each generator
//! fills a few formula shapes with random letters and numbers, using only the
//! notation its level teaches, so a sprint never asks for notation that is
//! still locked.
//!
//! Separate variables are always written with a space between them, so two
//! letters never merge into a keyword (`p i` is not `pi`).

use super::random::Random;

/// Makes one reference answer for the sprint.
pub type ExerciseGenerator = fn(&mut Random) -> String;

const VARIABLES: &[&str] = &["a", "b", "c", "p", "q", "r", "t", "u", "x", "y", "z"];
const INDICES: &[&str] = &["i", "j", "k", "m", "n"];
const CAPITALS: &[&str] = &["A", "B", "C", "S", "X"];
const FUNCTION_NAMES: &[&str] = &["f", "g", "h"];
const GREEK: &[&str] = &[
    "alpha", "beta", "gamma", "theta", "lambda", "mu", "rho", "sigma", "tau", "phi", "omega",
];
const CAPITAL_GREEK: &[&str] = &["Delta", "Gamma", "Lambda", "Omega", "Phi", "Sigma"];
const BINARY: &[&str] = &["+", "-", "*", "xx", "+-"];
const RELATIONS: &[&str] = &["=", "<", ">", "<=", ">=", "/=", "~="];
/// Ordered so that each set is a subset of the next.
const NUMBER_SETS: &[&str] = &["NN", "ZZ", "QQ", "RR", "CC"];
const TRIGONOMETRY: &[&str] = &["sin", "cos", "tan"];
const FUNCTIONS: &[&str] = &["sin", "cos", "tan", "ln", "exp", "sinh", "cosh"];
const ACCENTS: &[&str] = &["hat", "bar", "vec", "dot", "ddot", "tilde"];
const FONTS: &[&str] = &["cal", "bb"];
const LARGE_OPERATORS: &[&str] = &["sum", "prod"];
const EXTREMA: &[&str] = &["max", "min", "sup"];
const MATRICES: &[&str] = &["mat", "pmat", "bmat", "vmat"];

pub fn basics(random: &mut Random) -> String {
    let left = term(random);
    let operator = random.pick(BINARY);
    let right = term(random);
    if random.below(2) == 0 {
        return format!("{left} {operator} {right}");
    }
    let relation = random.pick(RELATIONS);
    format!("{left} {operator} {right} {relation} {}", term(random))
}

/// A variable, a number, or a number times a variable (`3x`).
fn term(random: &mut Random) -> String {
    match random.below(3) {
        0 => random.pick(VARIABLES).to_string(),
        1 => format!("{}{}", random.number(2..=9), random.pick(VARIABLES)),
        _ => random.number(1..=20).to_string(),
    }
}

pub fn scripts(random: &mut Random) -> String {
    let left = power(random);
    let relation = random.pick(&["+", "-", "="]);
    format!("{left} {relation} {}", power(random))
}

fn power(random: &mut Random) -> String {
    let base = random.pick(VARIABLES);
    let index = random.pick(INDICES);
    match random.below(7) {
        0 => format!("{base}^{}", random.number(2..=4)),
        1 => format!("{base}^{index}"),
        2 => format!("{base}^{index}+{}", random.number(1..=3)),
        3 => format!("{base}_{index}"),
        4 => format!("{base}_{index}^{}", random.number(2..=3)),
        5 => format!("{base}{}", random.number(0..=3)),
        _ => format!("{base} sq"),
    }
}

pub fn fractions(random: &mut Random) -> String {
    let [a, b, c] = random.pick_distinct(VARIABLES);
    let number = random.number(1..=9);
    match random.below(6) {
        0 => format!("{a}/{number}"),
        1 => format!("d{a}/d{b}"),
        2 => format!("{a}+{b}/{c}-{number}"),
        3 => format!("{a} + {b}/{c}"),
        4 => format!("1/{a} + 1/{b}"),
        _ => format!("{a} = {number} + {b} over {number} - {b}"),
    }
}

pub fn roots(random: &mut Random) -> String {
    let [a, b] = random.pick_distinct(VARIABLES);
    let number = random.number(2..=5);
    match random.below(9) {
        0 => format!("sqrt {number}"),
        1 => format!("sqrt {a}+{number}"),
        2 => format!("sqrt {a} + {number}"),
        3 => format!("root {number} {a}"),
        4 => format!("abs {a}-{b}"),
        5 => format!("norm {a}"),
        6 => format!("({a}+{b})^{number}"),
        7 => format!("sqrt[{a}^2 + {b}^2]"),
        _ => format!("e^[-{a}^2/{number}]"),
    }
}

pub fn greek(random: &mut Random) -> String {
    let [first, second] = random.pick_distinct(GREEK);
    let variable = random.pick(VARIABLES);
    match random.below(8) {
        0 => format!("{first} + {second}"),
        1 => format!("{} {first} {variable}", random.number(2..=4)),
        2 => format!("{} {variable}", random.pick(CAPITAL_GREEK)),
        3 => format!("{first}{}", random.number(1..=3)),
        4 => format!("{first} > 0"),
        5 => format!("{first} {variable} + {second}"),
        6 => format!("{variable} -> oo"),
        _ => format!("[del f]/[del {variable}]"),
    }
}

pub fn functions(random: &mut Random) -> String {
    let [a, b] = random.pick_distinct(VARIABLES);
    let function = random.pick(FUNCTION_NAMES);
    match random.below(8) {
        0 => format!("{} {a}", random.pick(FUNCTIONS)),
        1 => format!("log_{} {a}", random.number(2..=10)),
        2 => format!("{function}({a}) = {a}^{}", random.number(2..=3)),
        3 => format!("{function}'({a})"),
        4 => format!("det {}", random.pick(CAPITALS)),
        5 => format!("{}({a}+{b})", random.pick(FUNCTIONS)),
        6 => format!("e^[i {} {a}]", random.pick(GREEK)),
        _ => {
            let [first, second] = random.pick_distinct(TRIGONOMETRY);
            format!("{first}^2 {a} + {second}^2 {a}")
        }
    }
}

pub fn sets(random: &mut Random) -> String {
    let [a, b] = random.pick_distinct(CAPITALS);
    let variable = random.pick(VARIABLES);
    let number_set = random.pick(NUMBER_SETS);
    match random.below(10) {
        0 => format!("{variable} in {number_set}"),
        1 => {
            let start = random.below(NUMBER_SETS.len() - 1);
            format!("{} sub {}", NUMBER_SETS[start], NUMBER_SETS[start + 1])
        }
        2 => format!("{a} cup {b}"),
        3 => format!("{a} cap {b} = empty"),
        4 => format!("{variable} !in {a}"),
        5 => format!("{}: {a} to {b}", random.pick(FUNCTION_NAMES)),
        6 => format!("{variable} |-> {variable}^{}", random.number(2..=3)),
        7 => format!("forall {variable} in {number_set}"),
        8 => format!("exists {variable} in {number_set}"),
        _ => format!("{{{variable} in {number_set} : {variable} > 0}}"),
    }
}

pub fn accents(random: &mut Random) -> String {
    let [a, b] = random.pick_distinct(VARIABLES);
    let accent = random.pick(ACCENTS);
    match random.below(5) {
        0 => format!("{accent} {a}"),
        1 => format!("{} {}", random.pick(FONTS), random.pick(CAPITALS)),
        2 => format!("{accent}({a}+{b})"),
        3 => format!("vec {a} + vec {b}"),
        _ => format!("dot {a} = -{} {a}", random.pick(INDICES)),
    }
}

pub fn operators(random: &mut Random) -> String {
    let [index, bound] = random.pick_distinct(INDICES);
    let variable = random.pick(&["t", "u", "x", "y"]);
    let lower = random.number(0..=1);
    match random.below(6) {
        0 => {
            let operator = random.pick(LARGE_OPERATORS);
            let exponent = random.number(2..=3);
            format!("{operator} {index}={lower} to {bound} {index}^{exponent}")
        }
        1 => {
            let upper = lower + random.number(1..=3);
            format!("int {lower} to {upper} {variable} d{variable}")
        }
        2 => format!("sum {variable} in {} f({variable})", random.pick(CAPITALS)),
        3 => format!("sum {index}=0 to oo {}^{index}", random.pick(VARIABLES)),
        4 => format!(
            "bigcup {index}=1 to {bound} {}_{index}",
            random.pick(CAPITALS)
        ),
        _ => format!("int 0 to oo e^-{variable} d{variable}"),
    }
}

pub fn limits(random: &mut Random) -> String {
    let [variable, point] = random.pick_distinct(VARIABLES);
    let index = random.pick(INDICES);
    let function = random.pick(FUNCTION_NAMES);
    match random.below(5) {
        0 => format!("lim {variable} to {point} {function}({variable}) = {function}({point})"),
        1 => format!("lim {index} to oo 1/{index} = 0"),
        2 => {
            let extremum = random.pick(EXTREMA);
            let set = random.pick(CAPITALS);
            format!("{extremum} {variable} in {set} {function}({variable})")
        }
        3 => format!("inf_{index} {variable}_{index}"),
        _ => {
            let trigonometric = random.pick(TRIGONOMETRY);
            format!("lim {variable} to 0 ({trigonometric} {variable})/{variable}")
        }
    }
}

pub fn matrices(random: &mut Random) -> String {
    let variable = random.pick(VARIABLES);
    match random.below(4) {
        0 => {
            let cells: [String; 4] = std::array::from_fn(|_| matrix_cell(random));
            let [a, b, c, d] = cells;
            format!("{} {a} {b}, {c} {d}", random.pick(MATRICES))
        }
        1 => {
            let rows: [String; 3] = std::array::from_fn(|_| digit_row(random));
            format!("mat {}", rows.join(", "))
        }
        2 => format!("cases {variable} if {variable} >= 0, -{variable} otherwise"),
        _ => {
            let set = random.pick(NUMBER_SETS);
            format!("f({variable}) = cases 1 if {variable} in {set}, 0 otherwise")
        }
    }
}

fn digit_row(random: &mut Random) -> String {
    let digits: [String; 3] = std::array::from_fn(|_| random.number(0..=9).to_string());
    digits.join(" ")
}

/// A number, a variable or a negated variable.
fn matrix_cell(random: &mut Random) -> String {
    match random.below(3) {
        0 => random.number(0..=9).to_string(),
        1 => random.pick(VARIABLES).to_string(),
        _ => format!("-{}", random.pick(VARIABLES)),
    }
}

pub fn lecture(random: &mut Random) -> String {
    let [a, b, c] = random.pick_distinct(VARIABLES);
    let index = random.pick(INDICES);
    let function = random.pick(FUNCTION_NAMES);
    match random.below(7) {
        0 => format!("({a}+{b})^2 = {a}^2 + 2{a} {b} + {b}^2"),
        1 => format!("sum {index}=0 to oo {a}^{index} = 1 over 1 - {a}"),
        2 => format!("{c} = -{b} +- sqrt[{b}^2 - 4{a} {c}] over 2{a}"),
        3 => format!("int {a} to {b} {function}'({c}) d{c} = {function}({b}) - {function}({a})"),
        4 => format!("abs {a} = cases {a} if {a} >= 0, -{a} otherwise"),
        5 => format!(
            "det({} - {} I) = 0",
            random.pick(CAPITALS),
            random.pick(GREEK)
        ),
        _ => format!("norm {a}+{b} <= norm {a} + norm {b}"),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::super::curriculum::LEVELS;
    use super::*;

    const SAMPLES: u64 = 300;

    fn contains_error(math: &[qm_format::MathNode]) -> bool {
        format!("{math:?}").contains("Error {")
    }

    #[test]
    fn every_generated_formula_parses_without_errors() {
        for level in LEVELS {
            let mut random = Random::new(1);
            for _ in 0..SAMPLES {
                let answer = (level.generate)(&mut random);
                let math = qm_format::parse_math(&answer);
                assert!(
                    !contains_error(&math),
                    "{answer:?} from level {:?} does not parse",
                    level.id
                );
            }
        }
    }

    #[test]
    fn every_generator_varies_its_formulas() {
        for level in LEVELS {
            let mut random = Random::new(5);
            let answers: HashSet<String> = (0..SAMPLES)
                .map(|_| (level.generate)(&mut random))
                .collect();
            assert!(
                answers.len() >= 20,
                "level {:?} only made {} different formulas",
                level.id,
                answers.len()
            );
        }
    }

    #[test]
    fn same_seed_gives_the_same_formulas() {
        let mut first = Random::new(77);
        let mut second = Random::new(77);

        for _ in 0..20 {
            assert_eq!(lecture(&mut first), lecture(&mut second));
        }
    }
}
