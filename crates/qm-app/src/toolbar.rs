//! The top toolbar: app menu, project name, view toggles, theme, export, help.

use std::path::PathBuf;

use egui::{Align, Layout, RichText};
use qm_render::Theme;

/// How the centre area is split between the editor and the preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    EditorOnly,
    #[default]
    Split,
    PreviewOnly,
}

impl ViewMode {
    pub fn shows_editor(self) -> bool {
        self != ViewMode::PreviewOnly
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToolbarAction {
    OpenProject,
    NewProject,
    OpenRecent(PathBuf),
    CloseProject,
    Save,
    ToggleSidebar,
    ToggleTheme,
    ToggleHelp,
    OpenExport,
    Quit,
}

/// What the toolbar displays; `view_mode` is edited in place by the toggles.
pub struct ToolbarModel<'a> {
    pub project_name: &'a str,
    pub recent_projects: &'a [PathBuf],
    pub view_mode: &'a mut ViewMode,
    pub is_help_open: bool,
}

pub fn show_toolbar(
    ui: &mut egui::Ui,
    model: ToolbarModel,
    theme: &Theme,
) -> Option<ToolbarAction> {
    let mut action = None;
    ui.horizontal_centered(|ui| {
        app_menu(ui, model.recent_projects, &mut action);
        if ui.button("◧").on_hover_text("Toggle sidebar").clicked() {
            action = Some(ToolbarAction::ToggleSidebar);
        }
        ui.add_space(8.0);
        ui.label(
            RichText::new(model.project_name)
                .strong()
                .size(15.0)
                .color(theme.text),
        );

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .selectable_label(model.is_help_open, "？")
                .on_hover_text("Notation cheat sheet")
                .clicked()
            {
                action = Some(ToolbarAction::ToggleHelp);
            }
            let theme_icon = if theme.is_dark { "☀" } else { "🌙" };
            if ui
                .button(theme_icon)
                .on_hover_text("Toggle light/dark theme")
                .clicked()
            {
                action = Some(ToolbarAction::ToggleTheme);
            }
            let export = egui::Button::new(RichText::new("Export").color(theme.on_accent).strong())
                .fill(theme.accent);
            if ui.add(export).clicked() {
                action = Some(ToolbarAction::OpenExport);
            }
            ui.add_space(12.0);
            view_mode_toggle(ui, model.view_mode);
        });
    });
    action
}

fn app_menu(ui: &mut egui::Ui, recent_projects: &[PathBuf], action: &mut Option<ToolbarAction>) {
    ui.menu_button("☰", |ui| {
        menu_item(ui, "Save   (Ctrl+S)", ToolbarAction::Save, action);
        ui.separator();
        menu_item(
            ui,
            "Open project folder…",
            ToolbarAction::OpenProject,
            action,
        );
        menu_item(ui, "New project…", ToolbarAction::NewProject, action);
        ui.menu_button("Recent projects", |ui| {
            if recent_projects.is_empty() {
                ui.label("No recent projects");
            }
            for root in recent_projects {
                if ui.button(root.display().to_string()).clicked() {
                    *action = Some(ToolbarAction::OpenRecent(root.clone()));
                }
            }
        });
        menu_item(ui, "Close project", ToolbarAction::CloseProject, action);
        ui.separator();
        menu_item(ui, "Quit", ToolbarAction::Quit, action);
    });
}

fn menu_item(
    ui: &mut egui::Ui,
    label: &str,
    chosen: ToolbarAction,
    action: &mut Option<ToolbarAction>,
) {
    if ui.button(label).clicked() {
        *action = Some(chosen);
    }
}

/// Overleaf-style segmented control: Editor / Split / Preview.
fn view_mode_toggle(ui: &mut egui::Ui, view_mode: &mut ViewMode) {
    // Laid out right-to-left, so list the segments in reverse.
    let segments = [
        (ViewMode::PreviewOnly, "Preview"),
        (ViewMode::Split, "Split"),
        (ViewMode::EditorOnly, "Editor"),
    ];
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (mode, label) in segments {
            ui.selectable_value(view_mode, mode, label);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_preview_mode_hides_the_editor() {
        assert!(ViewMode::Split.shows_editor());
        assert!(ViewMode::EditorOnly.shows_editor());
        assert!(!ViewMode::PreviewOnly.shows_editor());
    }
}
