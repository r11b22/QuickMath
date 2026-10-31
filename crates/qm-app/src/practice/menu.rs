//! The practice menu: the sprint and the list of levels with their best results.

use egui::{Key, RichText, Sense};
use qm_core::PracticeProgress;
use qm_render::Theme;

use super::curriculum::LEVELS;
use super::is_unlocked;
use super::session::SPRINT_SECONDS;

const CONTENT_WIDTH: f32 = 560.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MenuAction {
    Back,
    PlayLevel(usize),
    PlaySprint,
}

pub(super) fn show_menu(
    ui: &mut egui::Ui,
    progress: &PracticeProgress,
    theme: &Theme,
) -> Option<MenuAction> {
    let mut action = None;
    if ui.input(|input| input.key_pressed(Key::Escape)) {
        action = Some(MenuAction::Back);
    }
    if ui.button("Back").clicked() {
        action = Some(MenuAction::Back);
    }
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.set_max_width(CONTENT_WIDTH);
                show_header(ui, theme);
                if sprint_card(ui, progress, theme).clicked() {
                    action = Some(MenuAction::PlaySprint);
                }
                ui.add_space(16.0);
                for index in 0..LEVELS.len() {
                    if level_card(ui, progress, index, theme).clicked() {
                        action = Some(MenuAction::PlayLevel(index));
                    }
                }
            });
        });
    action
}

fn show_header(ui: &mut egui::Ui, theme: &Theme) {
    ui.label(
        RichText::new("Practice")
            .size(32.0)
            .strong()
            .color(theme.text),
    );
    ui.label(
        RichText::new(
            "Type the formula you see. Any spelling that renders the same counts. \
             Finish a level to unlock the next; go fast without peeking for more stars.",
        )
        .color(theme.text_muted),
    );
    ui.add_space(20.0);
}

fn sprint_card(ui: &mut egui::Ui, progress: &PracticeProgress, theme: &Theme) -> egui::Response {
    let best = match progress.best_sprint_wpm {
        wpm if wpm > 0.0 => format!("Best {wpm:.0} WPM"),
        _ => "Not played yet".to_string(),
    };
    let description =
        format!("{SPRINT_SECONDS:.0} seconds of random formulas from every unlocked level.");
    card(ui, theme, true, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("⚡ Sprint")
                    .strong()
                    .size(17.0)
                    .color(theme.text),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(best).color(theme.accent_text));
            });
        });
        ui.label(RichText::new(description).color(theme.text_faint).small());
    })
}

fn level_card(
    ui: &mut egui::Ui,
    progress: &PracticeProgress,
    index: usize,
    theme: &Theme,
) -> egui::Response {
    let level = &LEVELS[index];
    let record = progress.level(level.id);
    let is_open = is_unlocked(progress, index);
    let title_color = if is_open {
        theme.text
    } else {
        theme.text_faint
    };
    card(ui, theme, is_open, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{}. {}", index + 1, level.title))
                    .strong()
                    .color(title_color),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !is_open {
                    ui.label(RichText::new("🔒").color(theme.text_faint));
                    return;
                }
                ui.label(RichText::new(star_text(record.stars)).color(theme.accent_text));
                if record.best_wpm > 0.0 {
                    ui.label(
                        RichText::new(format!("{:.0} WPM", record.best_wpm))
                            .color(theme.text_muted)
                            .small(),
                    );
                }
            });
        });
        ui.label(RichText::new(level.lesson).color(theme.text_faint).small());
    })
}

/// A clickable surface card; locked cards don't react to the pointer.
fn card(
    ui: &mut egui::Ui,
    theme: &Theme,
    is_enabled: bool,
    contents: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    let response = egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(14, 10))
        .corner_radius(crate::theme_style::CORNER_RADIUS)
        .fill(theme.surface)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), contents);
        })
        .response;
    if !is_enabled {
        return response;
    }
    let response = response.interact(Sense::click());
    if response.hovered() {
        ui.painter().rect_stroke(
            response.rect,
            crate::theme_style::CORNER_RADIUS,
            egui::Stroke::new(1.0, theme.accent),
            egui::StrokeKind::Inside,
        );
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Three star slots, filled up to `stars`.
pub(super) fn star_text(stars: u8) -> String {
    (0..3)
        .map(|slot| if slot < stars { '★' } else { '☆' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_text_fills_earned_slots() {
        assert_eq!(star_text(0), "☆☆☆");
        assert_eq!(star_text(2), "★★☆");
        assert_eq!(star_text(3), "★★★");
    }
}
