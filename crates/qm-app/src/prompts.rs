//! Small modal dialogs: naming new notes/folders, renaming, confirming deletes,
//! and offering to create the target of an unresolved wikilink.

use std::path::PathBuf;

use egui::{Key, RichText};
use qm_render::Theme;

#[derive(Debug, Clone, PartialEq)]
pub enum Prompt {
    NewNote { folder: PathBuf, name: String },
    NewFolder { parent: PathBuf, name: String },
    Rename { path: PathBuf, name: String },
    ConfirmDelete { path: PathBuf },
    CreateLinkedNote { target: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PromptOutcome {
    Pending,
    Cancelled,
    Confirmed,
}

impl Prompt {
    /// A rename prompt prefilled with the current name (notes without `.qm`;
    /// the backend keeps the extension).
    pub fn rename(path: PathBuf) -> Prompt {
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let name = crate::file_tree::note_display_name(&file_name).to_string();
        Prompt::Rename { path, name }
    }

    fn title(&self) -> &'static str {
        match self {
            Prompt::NewNote { .. } => "New note",
            Prompt::NewFolder { .. } => "New folder",
            Prompt::Rename { .. } => "Rename",
            Prompt::ConfirmDelete { .. } => "Move to trash?",
            Prompt::CreateLinkedNote { .. } => "Note not found",
        }
    }

    fn confirm_label(&self) -> &'static str {
        match self {
            Prompt::NewNote { .. } | Prompt::NewFolder { .. } | Prompt::CreateLinkedNote { .. } => {
                "Create"
            }
            Prompt::Rename { .. } => "Rename",
            Prompt::ConfirmDelete { .. } => "Move to trash",
        }
    }

    fn name_mut(&mut self) -> Option<&mut String> {
        match self {
            Prompt::NewNote { name, .. }
            | Prompt::NewFolder { name, .. }
            | Prompt::Rename { name, .. } => Some(name),
            Prompt::ConfirmDelete { .. } | Prompt::CreateLinkedNote { .. } => None,
        }
    }

    fn can_confirm(&self) -> bool {
        match self {
            Prompt::NewNote { name, .. }
            | Prompt::NewFolder { name, .. }
            | Prompt::Rename { name, .. } => !name.trim().is_empty(),
            Prompt::ConfirmDelete { .. } | Prompt::CreateLinkedNote { .. } => true,
        }
    }

    fn message(&self) -> Option<String> {
        match self {
            Prompt::ConfirmDelete { path } => {
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                Some(format!(
                    "“{name}” will be moved to the project's trash folder (.quickmath/trash)."
                ))
            }
            Prompt::CreateLinkedNote { target } => Some(format!(
                "There is no note called “{target}” yet. Create it in the project folder?"
            )),
            _ => None,
        }
    }
}

/// Draws `prompt` as a modal. Enter confirms, Escape or clicking outside cancels.
pub fn show_prompt(ctx: &egui::Context, prompt: &mut Prompt, theme: &Theme) -> PromptOutcome {
    let mut outcome = PromptOutcome::Pending;
    let modal = egui::Modal::new(egui::Id::new("prompt_modal")).show(ctx, |ui| {
        ui.set_width(360.0);
        ui.heading(RichText::new(prompt.title()).color(theme.text));
        if let Some(message) = prompt.message() {
            ui.label(RichText::new(message).color(theme.text_muted));
        }
        if let Some(name) = prompt.name_mut() {
            let response = ui.add(egui::TextEdit::singleline(name).desired_width(f32::INFINITY));
            if !response.has_focus() && !response.lost_focus() {
                response.request_focus();
            }
            if response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
                outcome = PromptOutcome::Confirmed;
            }
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (fill, text_color) = confirm_colors(prompt, theme);
                let confirm = egui::Button::new(
                    RichText::new(prompt.confirm_label())
                        .color(text_color)
                        .strong(),
                )
                .fill(fill);
                if ui.add_enabled(prompt.can_confirm(), confirm).clicked() {
                    outcome = PromptOutcome::Confirmed;
                }
                if ui.button("Cancel").clicked() {
                    outcome = PromptOutcome::Cancelled;
                }
            });
        });
    });
    if outcome == PromptOutcome::Confirmed && !prompt.can_confirm() {
        outcome = PromptOutcome::Pending;
    }
    if outcome == PromptOutcome::Pending && modal.should_close() {
        outcome = PromptOutcome::Cancelled;
    }
    outcome
}

/// Destructive confirmations are drawn in the error colour instead of the accent.
fn confirm_colors(prompt: &Prompt, theme: &Theme) -> (egui::Color32, egui::Color32) {
    if matches!(prompt, Prompt::ConfirmDelete { .. }) {
        (theme.error, theme.background)
    } else {
        (theme.accent, theme.on_accent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename_prefills_the_name_without_extension() {
        let prompt = Prompt::rename(PathBuf::from("/vault/week 1.qm"));
        assert_eq!(
            prompt,
            Prompt::Rename {
                path: PathBuf::from("/vault/week 1.qm"),
                name: "week 1".into()
            }
        );
    }

    #[test]
    fn blank_names_cannot_be_confirmed() {
        let blank = Prompt::NewNote {
            folder: PathBuf::from("/vault"),
            name: "  ".into(),
        };
        assert!(!blank.can_confirm());
        assert!(Prompt::ConfirmDelete {
            path: PathBuf::from("/vault/a.qm")
        }
        .can_confirm());
    }
}
