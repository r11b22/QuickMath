//! Playing a round (target formula, input, live rendering of the input) and
//! the results that follow it.

use std::time::Duration;

use egui::{FontId, Id, Key, RichText, TextStyle};
use qm_format::MathList;
use qm_render::Theme;

use super::curriculum::LEVELS;
use super::menu::star_text;
use super::session::{Session, SessionKind, SPRINT_SECONDS, THREE_STAR_WPM, TWO_STAR_WPM};
use super::{Results, Round};

const CONTENT_WIDTH: f32 = 720.0;
const TARGET_MATH_SIZE: f32 = 30.0;
const INPUT_MATH_SIZE: f32 = 18.0;
const INPUT_FONT_SIZE: f32 = 20.0;
/// How long the ✔ stays after a solved formula.
const SOLVED_FLASH_SECONDS: f64 = 0.6;
/// Keeps the clock ticking on screen while a round is running.
const CLOCK_REPAINT_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RoundAction {
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResultsAction {
    Retry,
    /// Play the level with this index.
    Next(usize),
    Menu,
}

pub(super) fn show_round(
    ui: &mut egui::Ui,
    round: &mut Round,
    theme: &Theme,
) -> Option<RoundAction> {
    let now = ui.input(|input| input.time);
    round.session.tick(now);
    if round.session.has_started() && !round.session.is_finished() {
        ui.ctx().request_repaint_after(CLOCK_REPAINT_INTERVAL);
    }
    if ui.input(|input| input.key_pressed(Key::Escape)) || ui.button("Levels").clicked() {
        return Some(RoundAction::Quit);
    }
    ui.vertical_centered(|ui| {
        ui.set_max_width(CONTENT_WIDTH);
        show_status_line(ui, &round.session, now, theme);
        ui.add_space(12.0);
        show_lesson(ui, round, theme);
        ui.add_space(28.0);
        show_target(ui, &round.session, now, theme);
        ui.add_space(28.0);
        show_input(ui, &mut round.session, now, theme);
    });
    None
}

fn show_status_line(ui: &mut egui::Ui, session: &Session, now: f64, theme: &Theme) {
    let title = match session.kind() {
        SessionKind::Level(index) => LEVELS[index].title,
        SessionKind::Sprint => "Sprint",
    };
    let progress = match session.total() {
        Some(total) => format!("{} / {total}", session.solved()),
        None => format!("{} solved", session.solved()),
    };
    let clock = match session.kind() {
        SessionKind::Sprint => format!("{:.0} s left", SPRINT_SECONDS - session.elapsed(now)),
        SessionKind::Level(_) => format!("{:.1} s", session.elapsed(now)),
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).strong().size(18.0).color(theme.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let stats = format!("{progress}   ·   {clock}   ·   {:.0} WPM", session.wpm(now));
            ui.label(RichText::new(stats).monospace().color(theme.text_muted));
        });
    });
}

fn show_lesson(ui: &mut egui::Ui, round: &Round, theme: &Theme) {
    let SessionKind::Level(index) = round.session.kind() else {
        return;
    };
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(14, 10))
        .corner_radius(crate::theme_style::CORNER_RADIUS)
        .fill(theme.surface)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.label(RichText::new(LEVELS[index].lesson).color(theme.text_muted));
                show_tips(ui, &round.tips, theme);
            });
        });
}

/// Each tip as its input next to its rendering, kept together when wrapping.
fn show_tips(ui: &mut egui::Ui, tips: &[(&'static str, MathList)], theme: &Theme) {
    if tips.is_empty() {
        return;
    }
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        for (input, math) in tips {
            ui.horizontal(|ui| {
                ui.label(RichText::new(*input).monospace().color(theme.syntax.math));
                qm_render::show_math(ui, math, false, theme);
            });
            ui.add_space(18.0);
        }
    });
}

fn show_target(ui: &mut egui::Ui, session: &Session, now: f64, theme: &Theme) {
    let is_flashing = session
        .last_solved_at()
        .is_some_and(|solved_at| now - solved_at < SOLVED_FLASH_SECONDS);
    let check = if is_flashing { "✔" } else { " " };
    ui.label(RichText::new(check).size(22.0).color(theme.accent_text));
    with_math_size(ui, TARGET_MATH_SIZE, |ui| {
        qm_render::show_math(ui, session.target(), true, theme);
    });
    if is_flashing {
        let remaining = SOLVED_FLASH_SECONDS - (now - session.last_solved_at().unwrap_or(now));
        ui.ctx()
            .request_repaint_after(Duration::from_secs_f64(remaining.max(0.0)));
    }
}

fn show_input(ui: &mut egui::Ui, session: &mut Session, now: f64, theme: &Theme) {
    let id = Id::new("practice_input");
    let response = ui.add(
        egui::TextEdit::singleline(&mut session.input)
            .id(id)
            .font(FontId::monospace(INPUT_FONT_SIZE))
            .hint_text("Type the formula…")
            .desired_width(f32::INFINITY),
    );
    if response.changed() {
        session.check_input(now);
    }
    // Enter means "I'm stuck": show the answer and keep typing.
    if response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
        session.reveal();
    }
    if !response.has_focus() {
        response.request_focus();
    }
    ui.add_space(8.0);
    if session.is_revealed() {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Answer:").color(theme.text_faint));
            ui.label(
                RichText::new(session.answer())
                    .monospace()
                    .size(INPUT_FONT_SIZE)
                    .color(theme.syntax.math),
            );
        });
    } else {
        ui.label(
            RichText::new("Enter shows the answer  ·  Esc back to levels")
                .color(theme.text_faint)
                .small(),
        );
    }
    ui.add_space(12.0);
    let input_math = qm_format::parse_math(&session.input);
    with_math_size(ui, INPUT_MATH_SIZE, |ui| {
        qm_render::show_math(ui, &input_math, true, theme);
    });
}

/// `show_math` sizes math relative to the body text, so scale that for `contents`.
fn with_math_size(ui: &mut egui::Ui, size: f32, contents: impl FnOnce(&mut egui::Ui)) {
    ui.scope(|ui| {
        ui.style_mut()
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(size));
        contents(ui);
    });
}

pub(super) fn show_results(
    ui: &mut egui::Ui,
    results: &Results,
    theme: &Theme,
) -> Option<ResultsAction> {
    let mut action = keyboard_results_action(ui, results);
    ui.vertical_centered(|ui| {
        ui.set_max_width(CONTENT_WIDTH);
        ui.add_space((ui.available_height() * 0.12).max(24.0));
        show_result_summary(ui, results, theme);
        ui.add_space(24.0);
        ui.horizontal(|ui| {
            if ui.button("Retry  (Enter)").clicked() {
                action = Some(ResultsAction::Retry);
            }
            if let Some(next) = results.next_level {
                if ui.button("Next level  (N)").clicked() {
                    action = Some(ResultsAction::Next(next));
                }
            }
            if ui.button("Levels  (Esc)").clicked() {
                action = Some(ResultsAction::Menu);
            }
        });
    });
    action
}

fn keyboard_results_action(ui: &egui::Ui, results: &Results) -> Option<ResultsAction> {
    ui.input(|input| {
        if input.key_pressed(Key::Enter) {
            Some(ResultsAction::Retry)
        } else if input.key_pressed(Key::Escape) {
            Some(ResultsAction::Menu)
        } else if input.key_pressed(Key::N) {
            results.next_level.map(ResultsAction::Next)
        } else {
            None
        }
    })
}

fn show_result_summary(ui: &mut egui::Ui, results: &Results, theme: &Theme) {
    let result = &results.result;
    let heading = match results.kind {
        SessionKind::Level(index) => format!("{} complete", LEVELS[index].title),
        SessionKind::Sprint => "Time's up".to_string(),
    };
    ui.label(RichText::new(heading).size(28.0).strong().color(theme.text));
    if let SessionKind::Level(_) = results.kind {
        ui.label(
            RichText::new(star_text(result.stars))
                .size(40.0)
                .color(theme.accent_text),
        );
    }
    ui.label(
        RichText::new(format!("{:.0} WPM", result.wpm))
            .size(22.0)
            .color(theme.text),
    );
    if results.is_new_best {
        ui.label(
            RichText::new("New personal best!")
                .strong()
                .color(theme.accent_text),
        );
    }
    let details = format!(
        "{} formulas in {:.1} s  ·  {} answers shown",
        result.solved, result.seconds, result.reveals
    );
    ui.label(RichText::new(details).color(theme.text_muted));
    if result.stars == 1 {
        ui.label(
            RichText::new(star_hint(result.reveals))
                .color(theme.text_faint)
                .small(),
        );
    }
}

fn star_hint(reveals: usize) -> String {
    if reveals > 0 {
        return "Finish without showing an answer to earn more stars.".to_string();
    }
    format!("Go faster for more stars: {TWO_STAR_WPM:.0} WPM for two, {THREE_STAR_WPM:.0} WPM for three.")
}
