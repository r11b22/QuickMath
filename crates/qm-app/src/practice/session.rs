//! One practice run: which formula comes next, the clock, and grading.
//!
//! Times are seconds on the caller's clock (egui's `input.time`), so the
//! session itself is deterministic and testable.

use qm_format::{MathList, MathNode};

use super::curriculum::LEVELS;
use super::generators::ExerciseGenerator;
use super::random::Random;

/// Length of a sprint.
pub const SPRINT_SECONDS: f64 = 60.0;
/// Typing-speed convention: one "word" is five characters.
const CHARS_PER_WORD: f64 = 5.0;
pub const TWO_STAR_WPM: f64 = 20.0;
pub const THREE_STAR_WPM: f64 = 35.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKind {
    /// Index into [`LEVELS`].
    Level(usize),
    Sprint,
}

/// How a finished session went.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SessionResult {
    pub wpm: f64,
    pub seconds: f64,
    pub solved: usize,
    pub reveals: usize,
    /// 1 to 3 for a finished level; 0 for a sprint.
    pub stars: u8,
}

/// A formula to type, with what it must render as.
struct Exercise {
    answer: String,
    target: MathList,
    target_latex: String,
    /// `target` without LaTeX spellings, so `=>` and `implies` (both ⇒) match.
    target_drawing: MathList,
}

impl Exercise {
    fn new(answer: String) -> Exercise {
        let target = qm_format::parse_math(&answer);
        Exercise {
            answer,
            target_latex: qm_format::math_to_latex(&target),
            target_drawing: without_latex(target.clone()),
            target,
        }
    }

    /// Correct when the input produces the same LaTeX (which ignores
    /// invisible grouping, e.g. `(a+b)/c` vs `a+b/c`) or draws the same symbols.
    fn is_solved_by(&self, input: &str) -> bool {
        let math = qm_format::parse_math(input);
        qm_format::math_to_latex(&math) == self.target_latex
            || without_latex(math) == self.target_drawing
    }
}

/// Clears every atom's LaTeX spelling, leaving only what is drawn.
fn without_latex(mut math: MathList) -> MathList {
    math.iter_mut().for_each(clear_latex);
    math
}

fn clear_latex(node: &mut MathNode) {
    let clear_all = |list: &mut MathList| list.iter_mut().for_each(clear_latex);
    match node {
        MathNode::Atom(atom) => atom.latex.clear(),
        MathNode::Fraction {
            numerator,
            denominator,
        } => {
            clear_all(numerator);
            clear_all(denominator);
        }
        MathNode::Scripts { base, sub, sup } => {
            clear_latex(base);
            sub.iter_mut().chain(sup.iter_mut()).for_each(clear_all);
        }
        MathNode::Root { index, radicand } => {
            index.iter_mut().for_each(clear_all);
            clear_all(radicand);
        }
        MathNode::Matrix { rows, .. } => rows.iter_mut().flatten().for_each(clear_all),
        MathNode::Fenced { body, .. } | MathNode::Accent { body, .. } | MathNode::Group(body) => {
            clear_all(body)
        }
        MathNode::Text(_) | MathNode::Space(_) | MathNode::Error { .. } => {}
    }
}

/// Where the next formula comes from.
enum ExerciseQueue {
    /// A level's answers in order.
    Ordered {
        answers: &'static [&'static str],
        next: usize,
    },
    /// Fresh formulas for a sprint from randomly chosen generators, never
    /// the same formula twice in a row.
    Generated {
        generators: Vec<ExerciseGenerator>,
        random: Random,
    },
}

impl ExerciseQueue {
    fn next(&mut self, previous: Option<&str>) -> Option<String> {
        match self {
            ExerciseQueue::Ordered { answers, next } => {
                let answer = answers.get(*next)?;
                *next += 1;
                Some(answer.to_string())
            }
            ExerciseQueue::Generated { generators, random } => loop {
                let answer = random.pick(generators)(random);
                if Some(answer.as_str()) != previous {
                    return Some(answer);
                }
            },
        }
    }
}

pub struct Session {
    kind: SessionKind,
    queue: ExerciseQueue,
    current: Exercise,
    /// What the player has typed for the current formula.
    pub input: String,
    total: Option<usize>,
    started_at: Option<f64>,
    finished_at: Option<f64>,
    last_solved_at: Option<f64>,
    solved: usize,
    solved_chars: usize,
    reveals: usize,
    is_revealed: bool,
}

impl Session {
    /// Plays the answers of `LEVELS[index]` in order.
    pub fn level(index: usize) -> Session {
        let answers = LEVELS[index].answers;
        let queue = ExerciseQueue::Ordered { answers, next: 0 };
        Session::new(SessionKind::Level(index), queue, Some(answers.len()))
    }

    /// Formulas made by `generators` for [`SPRINT_SECONDS`]. `generators`
    /// must not be empty, and each must be able to make more than one formula.
    pub fn sprint(generators: Vec<ExerciseGenerator>, seed: u64) -> Session {
        assert!(
            !generators.is_empty(),
            "a sprint needs at least one generator"
        );
        let queue = ExerciseQueue::Generated {
            generators,
            random: Random::new(seed),
        };
        Session::new(SessionKind::Sprint, queue, None)
    }

    fn new(kind: SessionKind, mut queue: ExerciseQueue, total: Option<usize>) -> Session {
        let first = queue.next(None).expect("sessions start with a formula");
        Session {
            kind,
            queue,
            current: Exercise::new(first),
            input: String::new(),
            total,
            started_at: None,
            finished_at: None,
            last_solved_at: None,
            solved: 0,
            solved_chars: 0,
            reveals: 0,
            is_revealed: false,
        }
    }

    pub fn kind(&self) -> SessionKind {
        self.kind
    }

    /// The formula to reproduce.
    pub fn target(&self) -> &MathList {
        &self.current.target
    }

    /// The reference answer for the current formula.
    pub fn answer(&self) -> &str {
        &self.current.answer
    }

    pub fn is_revealed(&self) -> bool {
        self.is_revealed
    }

    pub fn solved(&self) -> usize {
        self.solved
    }

    /// Number of formulas in a level; `None` for a sprint.
    pub fn total(&self) -> Option<usize> {
        self.total
    }

    pub fn last_solved_at(&self) -> Option<f64> {
        self.last_solved_at
    }

    pub fn is_finished(&self) -> bool {
        self.finished_at.is_some()
    }

    pub fn has_started(&self) -> bool {
        self.started_at.is_some()
    }

    /// Call after `input` changed. Starts the clock on the first keystroke and
    /// moves on when the input renders like the target. Returns whether it did.
    pub fn check_input(&mut self, now: f64) -> bool {
        if self.is_finished() || self.input.is_empty() {
            return false;
        }
        self.started_at.get_or_insert(now);
        if !self.current.is_solved_by(&self.input) {
            return false;
        }
        self.solved += 1;
        self.solved_chars += self.current.answer.chars().count();
        self.last_solved_at = Some(now);
        self.advance(now);
        true
    }

    /// Shows the reference answer. Counts against the score once per formula.
    pub fn reveal(&mut self) {
        if !self.is_revealed {
            self.is_revealed = true;
            self.reveals += 1;
        }
    }

    /// Ends a sprint whose time is up. Call once per frame.
    pub fn tick(&mut self, now: f64) {
        if self.kind != SessionKind::Sprint || self.is_finished() {
            return;
        }
        if let Some(started_at) = self.started_at {
            if now - started_at >= SPRINT_SECONDS {
                self.finished_at = Some(started_at + SPRINT_SECONDS);
            }
        }
    }

    /// Seconds on the clock: 0 before the first keystroke, frozen once finished.
    pub fn elapsed(&self, now: f64) -> f64 {
        let Some(started_at) = self.started_at else {
            return 0.0;
        };
        let end = self.finished_at.unwrap_or(now);
        let elapsed = (end - started_at).max(0.0);
        match self.kind {
            SessionKind::Sprint => elapsed.min(SPRINT_SECONDS),
            SessionKind::Level(_) => elapsed,
        }
    }

    /// Speed over the solved formulas, counting their reference answers' length.
    pub fn wpm(&self, now: f64) -> f64 {
        let minutes = self.elapsed(now) / 60.0;
        if minutes <= 0.0 {
            return 0.0;
        }
        self.solved_chars as f64 / CHARS_PER_WORD / minutes
    }

    pub fn result(&self) -> Option<SessionResult> {
        let finished_at = self.finished_at?;
        let wpm = self.wpm(finished_at);
        let stars = match self.kind {
            SessionKind::Level(_) => stars_for(wpm, self.reveals),
            SessionKind::Sprint => 0,
        };
        Some(SessionResult {
            wpm,
            seconds: self.elapsed(finished_at),
            solved: self.solved,
            reveals: self.reveals,
            stars,
        })
    }

    fn advance(&mut self, now: f64) {
        self.input.clear();
        self.is_revealed = false;
        match self.queue.next(Some(&self.current.answer)) {
            Some(answer) => self.current = Exercise::new(answer),
            None => self.finished_at = Some(now),
        }
    }
}

/// Finishing earns one star; speed without peeking earns more.
fn stars_for(wpm: f64, reveals: usize) -> u8 {
    match (reveals, wpm) {
        (0, wpm) if wpm >= THREE_STAR_WPM => 3,
        (0, wpm) if wpm >= TWO_STAR_WPM => 2,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::super::generators;
    use super::*;

    fn type_answer(session: &mut Session, input: &str, now: f64) -> bool {
        session.input = input.to_string();
        session.check_input(now)
    }

    fn solve_level(index: usize, seconds_per_formula: f64) -> Session {
        let mut session = Session::level(index);
        let mut now = 0.0;
        while !session.is_finished() {
            let answer = session.answer().to_string();
            type_answer(&mut session, &answer[..1], now);
            now += seconds_per_formula;
            assert!(type_answer(&mut session, &answer, now), "{answer:?}");
        }
        session
    }

    #[test]
    fn every_reference_answer_solves_its_own_formula() {
        for (index, _) in LEVELS.iter().enumerate() {
            let session = solve_level(index, 1.0);
            assert_eq!(Some(session.solved()), session.total());
        }
    }

    #[test]
    fn accepts_other_spellings_that_render_the_same() {
        let cases = [
            ("x^2", "x sq"),
            ("a+b/c+d", "(a+b)/(c+d)"),
            ("a+b/c+d", "a+b over c+d"),
            ("sqrt x+1", "sqrt(x+1)"),
            ("p => q", "p implies q"),
            ("a <= b", "a le b"),
            ("(a+b)^2 = a^2 + 2a b + b^2", "(a+b)^2 = a^2 + 2ab + b^2"),
        ];
        for (answer, alternative) in cases {
            assert!(
                Exercise::new(answer.to_string()).is_solved_by(alternative),
                "{alternative:?} should solve {answer:?}"
            );
        }
    }

    #[test]
    fn rejects_input_that_renders_differently() {
        let cases = [
            ("x^2 + 1", "x^2+1"),
            ("a + b/c", "a+b/c"),
            ("sqrt x + 1", "sqrt x+1"),
            ("sin x", "sinx"),
        ];
        for (answer, wrong) in cases {
            assert!(
                !Exercise::new(answer.to_string()).is_solved_by(wrong),
                "{wrong:?} should not solve {answer:?}"
            );
        }
    }

    #[test]
    fn clock_starts_on_the_first_keystroke() {
        let mut session = Session::level(0);

        type_answer(&mut session, "a", 5.0);

        assert!(session.has_started());
        assert_eq!(session.elapsed(7.5), 2.5);
    }

    #[test]
    fn empty_input_does_not_start_the_clock() {
        let mut session = Session::level(0);

        type_answer(&mut session, "", 5.0);

        assert!(!session.has_started());
        assert_eq!(session.elapsed(9.0), 0.0);
    }

    #[test]
    fn solving_moves_on_and_clears_the_input() {
        let mut session = Session::level(0);
        let first = session.answer().to_string();

        assert!(type_answer(&mut session, &first, 1.0));

        assert_eq!(session.solved(), 1);
        assert!(session.input.is_empty());
        assert_ne!(session.answer(), first);
    }

    #[test]
    fn revealing_counts_once_per_formula_and_caps_stars() {
        let mut session = Session::level(0);
        session.reveal();
        session.reveal();
        let mut now = 0.0;
        while !session.is_finished() {
            now += 0.1;
            let answer = session.answer().to_string();
            type_answer(&mut session, &answer, now);
        }

        let result = session.result().unwrap();

        assert_eq!(result.reveals, 1);
        assert_eq!(result.stars, 1);
    }

    #[test]
    fn stars_rise_with_speed() {
        assert_eq!(stars_for(10.0, 0), 1);
        assert_eq!(stars_for(TWO_STAR_WPM, 0), 2);
        assert_eq!(stars_for(THREE_STAR_WPM, 0), 3);
        assert_eq!(stars_for(100.0, 2), 1);
    }

    #[test]
    fn wpm_counts_reference_characters_per_five() {
        let mut session = Session::level(0);
        type_answer(&mut session, "a", 0.0);
        // "a + b" is 5 characters: one word in 6 seconds is 10 WPM.
        type_answer(&mut session, "a+b", 6.0);

        assert!((session.wpm(6.0) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn sprint_ends_after_its_time_and_freezes_the_clock() {
        let mut session = Session::sprint(vec![generators::basics], 42);
        type_answer(&mut session, "z", 10.0);

        session.tick(10.0 + SPRINT_SECONDS - 0.1);
        assert!(!session.is_finished());
        session.tick(10.0 + SPRINT_SECONDS + 3.0);

        assert!(session.is_finished());
        assert_eq!(session.elapsed(500.0), SPRINT_SECONDS);
        assert_eq!(session.result().unwrap().stars, 0);
    }

    /// Only two formulas, so repeats would be common without the guard.
    fn x_or_y(random: &mut Random) -> String {
        random.pick(&["x", "y"]).to_string()
    }

    #[test]
    fn sprint_never_repeats_the_previous_formula() {
        let mut session = Session::sprint(vec![x_or_y], 7);
        for step in 0..50 {
            let previous = session.answer().to_string();
            type_answer(&mut session, &previous, step as f64);
            assert_ne!(session.answer(), previous);
        }
    }

    #[test]
    fn sprint_formulas_solve_themselves() {
        let all_levels = LEVELS.iter().map(|level| level.generate).collect();
        let mut session = Session::sprint(all_levels, 11);
        for step in 0..200 {
            let answer = session.answer().to_string();
            assert!(
                type_answer(&mut session, &answer, step as f64),
                "{answer:?}"
            );
        }
    }

    #[test]
    fn finished_session_ignores_further_input() {
        let mut session = solve_level(0, 1.0);
        let solved = session.solved();

        assert!(!type_answer(&mut session, "a + b", 99.0));
        assert_eq!(session.solved(), solved);
    }
}
