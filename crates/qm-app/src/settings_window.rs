//! The settings window: theme, pane layout and spell checking languages.
//!
//! Every change applies at once. The theme and layout can also be changed
//! from the toolbar; this window gathers every setting in one place.

use egui::RichText;
use qm_core::Language;
use qm_render::Theme;

use crate::toolbar::{ToolbarAction, ViewMode};

const WINDOW_WIDTH: f32 = 380.0;

/// What the spell check section offers and which languages are on.
#[derive(Clone, Copy)]
pub struct SpellingSettings<'a> {
    pub available: &'a [Language],
    /// Codes of the languages that are on.
    pub enabled: &'a [String],
}

/// What the window displays; `view_mode` is edited in place, as in the toolbar.
pub struct SettingsModel<'a> {
    pub view_mode: &'a mut ViewMode,
    pub is_sidebar_open: bool,
    pub is_help_open: bool,
    pub is_slides_open: bool,
    pub spelling: SpellingSettings<'a>,
}

#[derive(Default)]
pub struct SettingsWindow {
    is_open: bool,
}

impl SettingsWindow {
    pub fn open(&mut self) {
        self.is_open = true;
    }

    /// Draws the window when open. Returns the setting the user changed, as
    /// the same action the toolbar would give for it.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        mut model: SettingsModel,
        theme: &Theme,
    ) -> Option<ToolbarAction> {
        if !self.is_open {
            return None;
        }
        let mut action = None;
        let modal = egui::Modal::new(egui::Id::new("settings_window")).show(ctx, |ui| {
            ui.set_width(WINDOW_WIDTH);
            ui.heading(RichText::new("Settings").color(theme.text));
            ui.add_space(8.0);
            appearance_section(ui, theme, &mut action);
            ui.add_space(8.0);
            layout_section(ui, &mut model, &mut action);
            ui.add_space(8.0);
            spelling_section(ui, model.spelling, theme, &mut action);
            ui.add_space(12.0);
            ui.separator();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    self.is_open = false;
                }
            });
        });
        if modal.should_close() {
            self.is_open = false;
        }
        action
    }
}

fn appearance_section(ui: &mut egui::Ui, theme: &Theme, action: &mut Option<ToolbarAction>) {
    ui.label(RichText::new("Appearance").strong());
    let mut is_dark = theme.is_dark;
    if ui.checkbox(&mut is_dark, "Dark theme").clicked() {
        *action = Some(ToolbarAction::ToggleTheme);
    }
}

fn layout_section(
    ui: &mut egui::Ui,
    model: &mut SettingsModel,
    action: &mut Option<ToolbarAction>,
) {
    ui.label(RichText::new("Layout").strong());
    ui.horizontal(|ui| {
        ui.radio_value(model.view_mode, ViewMode::EditorOnly, "Editor");
        ui.radio_value(model.view_mode, ViewMode::Split, "Split");
        ui.radio_value(model.view_mode, ViewMode::PreviewOnly, "Preview");
    });
    let mut is_sidebar_open = model.is_sidebar_open;
    if ui.checkbox(&mut is_sidebar_open, "Show sidebar").clicked() {
        *action = Some(ToolbarAction::ToggleSidebar);
    }
    let mut is_help_open = model.is_help_open;
    if ui.checkbox(&mut is_help_open, "Show cheat sheet").clicked() {
        *action = Some(ToolbarAction::ToggleHelp);
    }
    let mut is_slides_open = model.is_slides_open;
    if ui.checkbox(&mut is_slides_open, "Show slides").clicked() {
        *action = Some(ToolbarAction::ToggleSlides);
    }
}

/// One checkbox per language; several can be on at once.
fn spelling_section(
    ui: &mut egui::Ui,
    spelling: SpellingSettings,
    theme: &Theme,
    action: &mut Option<ToolbarAction>,
) {
    ui.label(RichText::new("Spell check").strong());
    for language in spelling.available {
        let code = language.code();
        let mut is_on = spelling.enabled.iter().any(|enabled| enabled == code);
        if ui.checkbox(&mut is_on, code).clicked() {
            *action = Some(ToolbarAction::ToggleSpellingLanguage(code.to_string()));
        }
    }
    if let Some(folder) = Language::user_folder() {
        ui.label(
            RichText::new(format!(
                "More languages: add Hunspell .aff and .dic files to {} and restart.",
                folder.display()
            ))
            .small()
            .color(theme.text_faint),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Modifiers, PointerButton, RawInput, Shape};

    /// Drives the real window headlessly, one frame per call.
    struct Harness {
        ctx: egui::Context,
        window: SettingsWindow,
        view_mode: ViewMode,
        languages: Vec<Language>,
        enabled: Vec<String>,
        /// Every label drawn last frame, with where it was drawn.
        labels: Vec<(String, egui::Rect)>,
        last_action: Option<ToolbarAction>,
    }

    impl Harness {
        fn open() -> Harness {
            let mut harness = Harness {
                ctx: egui::Context::default(),
                window: SettingsWindow::default(),
                view_mode: ViewMode::Split,
                languages: Language::available_in(&[]),
                enabled: vec!["en_US".to_string()],
                labels: Vec::new(),
                last_action: None,
            };
            harness.window.open();
            // egui sizes a new modal on its first frame without drawing it.
            harness.frame(Vec::new());
            harness.frame(Vec::new());
            harness
        }

        fn frame(&mut self, events: Vec<Event>) {
            let input = RawInput {
                events,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 800.0),
                )),
                ..RawInput::default()
            };
            let theme = Theme::light();
            let Harness {
                ctx,
                window,
                view_mode,
                languages,
                enabled,
                ..
            } = self;
            let mut action = None;
            let output = ctx.run_ui(input, |ui| {
                let model = SettingsModel {
                    view_mode,
                    is_sidebar_open: true,
                    is_help_open: false,
                    is_slides_open: false,
                    spelling: SpellingSettings {
                        available: languages,
                        enabled,
                    },
                };
                action = window.show(ui.ctx(), model, &theme);
            });
            self.last_action = action;
            self.labels = output
                .shapes
                .into_iter()
                .filter_map(|clipped| match clipped.shape {
                    Shape::Text(text) => Some((
                        text.galley.text().to_string(),
                        text.galley.rect.translate(text.pos.to_vec2()),
                    )),
                    _ => None,
                })
                .collect();
        }

        fn click_label(&mut self, label: &str) {
            let position = self
                .labels
                .iter()
                .find(|(text, _)| text == label)
                .unwrap_or_else(|| panic!("{label:?} was not drawn"))
                .1
                .center();
            let button = |pressed| Event::PointerButton {
                pos: position,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            };
            self.frame(vec![Event::PointerMoved(position)]);
            self.frame(vec![button(true)]);
            self.frame(vec![button(false)]);
        }
    }

    #[test]
    fn closed_window_draws_nothing() {
        let mut harness = Harness::open();
        harness.window = SettingsWindow::default();
        harness.frame(Vec::new());
        assert!(harness.labels.is_empty());
    }

    #[test]
    fn theme_checkbox_toggles_the_theme() {
        let mut harness = Harness::open();
        harness.click_label("Dark theme");
        assert_eq!(harness.last_action, Some(ToolbarAction::ToggleTheme));
    }

    #[test]
    fn pane_checkboxes_toggle_their_pane() {
        let mut harness = Harness::open();
        harness.click_label("Show sidebar");
        assert_eq!(harness.last_action, Some(ToolbarAction::ToggleSidebar));
        harness.click_label("Show cheat sheet");
        assert_eq!(harness.last_action, Some(ToolbarAction::ToggleHelp));
        harness.click_label("Show slides");
        assert_eq!(harness.last_action, Some(ToolbarAction::ToggleSlides));
    }

    #[test]
    fn view_mode_is_changed_in_place() {
        let mut harness = Harness::open();
        harness.click_label("Preview");
        assert_eq!(harness.view_mode, ViewMode::PreviewOnly);
        assert_eq!(harness.last_action, None);
    }

    #[test]
    fn language_checkbox_toggles_that_language() {
        let mut harness = Harness::open();
        harness.click_label("nl_NL");
        assert_eq!(
            harness.last_action,
            Some(ToolbarAction::ToggleSpellingLanguage("nl_NL".to_string()))
        );
    }

    #[test]
    fn close_button_closes_the_window() {
        let mut harness = Harness::open();
        harness.click_label("Close");
        harness.frame(Vec::new());
        assert!(!harness.labels.iter().any(|(text, _)| text == "Settings"));
    }
}
