//! The welcome screen shown when no project is open.

use std::path::{Path, PathBuf};

use egui::{RichText, Sense};
use qm_render::Theme;

#[derive(Debug, Clone, PartialEq)]
pub enum WelcomeAction {
    OpenProjectDialog,
    OpenPractice,
    OpenProject(PathBuf),
    /// Create `parent/name` and open it as a project.
    CreateProject {
        parent: PathBuf,
        name: String,
    },
}

#[derive(Default)]
struct NewProjectForm {
    parent: Option<PathBuf>,
    name: String,
}

#[derive(Default)]
pub struct WelcomeScreen {
    new_project: Option<NewProjectForm>,
}

const CONTENT_WIDTH: f32 = 460.0;

impl WelcomeScreen {
    /// A welcome screen with the "New project" form already open.
    pub fn with_new_project_form() -> WelcomeScreen {
        WelcomeScreen {
            new_project: Some(NewProjectForm::default()),
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        recent_projects: &[PathBuf],
        theme: &Theme,
    ) -> Option<WelcomeAction> {
        let mut action = None;
        ui.vertical_centered(|ui| {
            ui.set_max_width(CONTENT_WIDTH);
            ui.add_space((ui.available_height() * 0.12).max(24.0));
            ui.label(
                RichText::new("QuickMath")
                    .size(40.0)
                    .strong()
                    .color(theme.text),
            );
            ui.label(
                RichText::new("Fast math notes for lectures, with a live preview.")
                    .size(15.0)
                    .color(theme.text_muted),
            );
            ui.add_space(28.0);
            self.show_buttons(ui, theme, &mut action);
            if let Some(form) = &mut self.new_project {
                ui.add_space(12.0);
                if let Some(created) = show_new_project_form(ui, form, theme) {
                    action = Some(created);
                }
            }
            ui.add_space(28.0);
            show_recent_projects(ui, recent_projects, theme, &mut action);
        });
        if matches!(action, Some(WelcomeAction::CreateProject { .. })) {
            self.new_project = None;
        }
        action
    }

    fn show_buttons(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        action: &mut Option<WelcomeAction>,
    ) {
        let open = egui::Button::new(
            RichText::new("🗁  Open project folder…")
                .color(theme.on_accent)
                .strong(),
        )
        .fill(theme.accent)
        .min_size(egui::vec2(220.0, 36.0));
        if ui.add(open).clicked() {
            *action = Some(WelcomeAction::OpenProjectDialog);
        }
        let new_project = egui::Button::new("➕  New project…").min_size(egui::vec2(220.0, 36.0));
        if ui.add(new_project).clicked() {
            self.new_project = Some(NewProjectForm::default());
        }
        let practice = egui::Button::new("🎯  Practice notation").min_size(egui::vec2(220.0, 36.0));
        if ui.add(practice).clicked() {
            *action = Some(WelcomeAction::OpenPractice);
        }
    }
}

fn show_new_project_form(
    ui: &mut egui::Ui,
    form: &mut NewProjectForm,
    theme: &Theme,
) -> Option<WelcomeAction> {
    let mut action = None;
    egui::Frame::group(ui.style())
        .fill(theme.surface)
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(CONTENT_WIDTH - 28.0);
            ui.label(RichText::new("New project").strong());
            ui.horizontal(|ui| {
                ui.label("Location:");
                let location = form
                    .parent
                    .as_ref()
                    .map_or("not chosen".to_string(), |parent| {
                        parent.display().to_string()
                    });
                ui.label(RichText::new(location).color(theme.text_muted));
                if ui.button("Choose…").clicked() {
                    form.parent = rfd::FileDialog::new()
                        .set_title("Choose where to create the project")
                        .pick_folder();
                }
            });
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.add(egui::TextEdit::singleline(&mut form.name).hint_text("e.g. Linear Algebra"));
            });
            let validation = validate_project_name(&form.name);
            if let (Err(message), false) = (&validation, form.name.is_empty()) {
                ui.label(RichText::new(message).color(theme.error).small());
            }
            let can_create = validation.is_ok() && form.parent.is_some();
            let create_clicked = ui
                .add_enabled(can_create, egui::Button::new("Create project"))
                .clicked();
            if let (true, Some(parent)) = (create_clicked, form.parent.clone()) {
                action = Some(WelcomeAction::CreateProject {
                    parent,
                    name: form.name.trim().to_string(),
                });
            }
        });
    action
}

fn show_recent_projects(
    ui: &mut egui::Ui,
    recent_projects: &[PathBuf],
    theme: &Theme,
    action: &mut Option<WelcomeAction>,
) {
    if recent_projects.is_empty() {
        return;
    }
    ui.label(
        RichText::new("Recent projects")
            .strong()
            .color(theme.text_muted),
    );
    ui.add_space(4.0);
    for root in recent_projects {
        if recent_row(ui, root, theme).clicked() {
            *action = Some(WelcomeAction::OpenProject(root.clone()));
        }
    }
}

fn recent_row(ui: &mut egui::Ui, root: &Path, theme: &Theme) -> egui::Response {
    let name = root.file_name().map_or_else(
        || root.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    let response = egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(12, 8))
        .corner_radius(crate::theme_style::CORNER_RADIUS)
        .fill(theme.surface)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(name).color(theme.text));
            ui.label(
                RichText::new(root.display().to_string())
                    .color(theme.text_faint)
                    .small(),
            );
        })
        .response
        .interact(Sense::click());
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

/// Checks that `name` can be used as a single folder name.
pub fn validate_project_name(name: &str) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Enter a name.".to_string());
    }
    if name == "." || name == ".." || name.starts_with('.') {
        return Err("The name cannot start with a dot.".to_string());
    }
    if name.contains(['/', '\\', ':']) {
        return Err("The name cannot contain / \\ or :".to_string());
    }
    Ok(())
}

/// Creates the project folder `parent/name` (it must not exist yet).
pub fn create_project_folder(parent: &Path, name: &str) -> Result<PathBuf, String> {
    validate_project_name(name)?;
    let root = parent.join(name.trim());
    if root.exists() {
        return Err(format!("{} already exists.", root.display()));
    }
    std::fs::create_dir_all(&root)
        .map_err(|error| format!("Could not create {}: {error}", root.display()))?;
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_ordinary_names() {
        assert!(validate_project_name("Linear Algebra 2026").is_ok());
        assert!(validate_project_name("  Analyse  ").is_ok());
    }

    #[test]
    fn rejects_empty_hidden_and_path_like_names() {
        for name in ["", "   ", ".", "..", ".hidden", "a/b", "a\\b", "c:"] {
            assert!(
                validate_project_name(name).is_err(),
                "{name:?} should be rejected"
            );
        }
    }

    #[test]
    fn create_project_folder_refuses_existing_folders() {
        let parent = std::env::temp_dir();
        let name = format!("qm-app-test-{}", std::process::id());
        let created = create_project_folder(&parent, &name).expect("first creation succeeds");
        assert!(created.is_dir());
        assert!(create_project_folder(&parent, &name).is_err());
        std::fs::remove_dir(&created).expect("cleanup");
    }
}
