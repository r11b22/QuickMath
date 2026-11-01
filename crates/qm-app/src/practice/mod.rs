//! The notation practice game: a formula is shown rendered, the player types
//! QuickMath for it, and the next one appears as soon as the input renders
//! the same. Levels unlock one by one; the sprint generates random formulas
//! with the notation of every unlocked level.

mod curriculum;
mod generators;
mod menu;
mod play;
mod random;
mod session;

use std::time::{SystemTime, UNIX_EPOCH};

use qm_core::PracticeProgress;
use qm_format::MathList;
use qm_render::Theme;

use crate::toasts::Toasts;
use curriculum::LEVELS;
use generators::ExerciseGenerator;
use menu::MenuAction;
use play::{ResultsAction, RoundAction};
use session::{Session, SessionKind, SessionResult};

/// What the practice screen asks the app to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PracticeAction {
    Close,
}

/// A session in progress, with the level's tips parsed once.
struct Round {
    session: Session,
    tips: Vec<(&'static str, MathList)>,
}

impl Round {
    fn new(session: Session) -> Round {
        let tips = match session.kind() {
            SessionKind::Level(index) => LEVELS[index]
                .tips
                .iter()
                .map(|tip| (*tip, qm_format::parse_math(tip)))
                .collect(),
            SessionKind::Sprint => Vec::new(),
        };
        Round { session, tips }
    }
}

struct Results {
    kind: SessionKind,
    result: SessionResult,
    is_new_best: bool,
    /// The level the player may go on to, if it exists and is now unlocked.
    next_level: Option<usize>,
}

enum View {
    Menu,
    Playing(Box<Round>),
    Results(Results),
}

pub struct PracticeScreen {
    progress: PracticeProgress,
    view: View,
}

impl PracticeScreen {
    /// Opens the level menu with the saved progress.
    pub fn open() -> PracticeScreen {
        PracticeScreen {
            progress: PracticeProgress::load(),
            view: View::Menu,
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        toasts: &mut Toasts,
    ) -> Option<PracticeAction> {
        let panel_frame = egui::Frame::new()
            .fill(theme.background)
            .inner_margin(egui::Margin::same(24));
        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ui, |ui| self.show_view(ui, theme, toasts))
            .inner
    }

    fn show_view(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        toasts: &mut Toasts,
    ) -> Option<PracticeAction> {
        match &mut self.view {
            View::Menu => match menu::show_menu(ui, &self.progress, theme)? {
                MenuAction::Back => return Some(PracticeAction::Close),
                MenuAction::PlayLevel(index) => self.start(SessionKind::Level(index)),
                MenuAction::PlaySprint => self.start(SessionKind::Sprint),
            },
            View::Playing(round) => {
                if play::show_round(ui, round, theme) == Some(RoundAction::Quit) {
                    self.view = View::Menu;
                } else if let Some(result) = round.session.result() {
                    let kind = round.session.kind();
                    self.finish(kind, result, toasts);
                }
            }
            View::Results(results) => match play::show_results(ui, results, theme)? {
                ResultsAction::Retry => {
                    let kind = results.kind;
                    self.start(kind);
                }
                ResultsAction::Next(index) => self.start(SessionKind::Level(index)),
                ResultsAction::Menu => self.view = View::Menu,
            },
        }
        None
    }

    fn start(&mut self, kind: SessionKind) {
        let session = match kind {
            SessionKind::Level(index) => Session::level(index),
            SessionKind::Sprint => {
                Session::sprint(sprint_generators(&self.progress), random_seed())
            }
        };
        self.view = View::Playing(Box::new(Round::new(session)));
    }

    /// Records the result, saves progress and shows the results view.
    fn finish(&mut self, kind: SessionKind, result: SessionResult, toasts: &mut Toasts) {
        let (is_new_best, next_level) = match kind {
            SessionKind::Level(index) => {
                let is_new_best =
                    self.progress
                        .record_level(LEVELS[index].id, result.stars, result.wpm);
                (is_new_best, curriculum::next_level(index))
            }
            SessionKind::Sprint => (self.progress.record_sprint(result.wpm), None),
        };
        if let Err(error) = self.progress.save() {
            toasts.error(format!("Could not save practice progress: {error}"));
        }
        self.view = View::Results(Results {
            kind,
            result,
            is_new_best,
            next_level: next_level.filter(|&index| is_unlocked(&self.progress, index)),
        });
    }
}

/// The first level is always open; every other one opens once the level
/// before it has been finished.
fn is_unlocked(progress: &PracticeProgress, index: usize) -> bool {
    index == 0 || progress.level(LEVELS[index - 1].id).stars > 0
}

/// The exercise generators of the unlocked levels.
fn sprint_generators(progress: &PracticeProgress) -> Vec<ExerciseGenerator> {
    (0..LEVELS.len())
        .filter(|&index| is_unlocked(progress, index))
        .map(|index| LEVELS[index].generate)
        .collect()
}

fn random_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_first_level_is_open_at_the_start() {
        let progress = PracticeProgress::default();

        assert!(is_unlocked(&progress, 0));
        assert!(!is_unlocked(&progress, 1));
    }

    #[test]
    fn finishing_a_level_opens_the_next_one() {
        let mut progress = PracticeProgress::default();
        progress.record_level(LEVELS[0].id, 1, 12.0);

        assert!(is_unlocked(&progress, 1));
        assert!(!is_unlocked(&progress, 2));
    }

    #[test]
    fn sprint_generates_from_unlocked_levels_only() {
        let mut progress = PracticeProgress::default();
        assert_eq!(sprint_generators(&progress).len(), 1);

        progress.record_level(LEVELS[0].id, 2, 25.0);

        assert_eq!(sprint_generators(&progress).len(), 2);
    }
}
