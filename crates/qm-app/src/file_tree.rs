//! The sidebar's project file tree.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use egui::{Align2, Sense};
use qm_core::{FileKind, FileNode};
use qm_render::Theme;

/// Something the user asked for in the tree; applied by the workspace.
#[derive(Debug, Clone, PartialEq)]
pub enum FileTreeAction {
    OpenNote(PathBuf),
    /// Show this PDF in the slides pane.
    OpenSlides(PathBuf),
    NewNote {
        folder: PathBuf,
    },
    NewFolder {
        parent: PathBuf,
    },
    Rename(PathBuf),
    Delete(PathBuf),
}

const ROW_HEIGHT: f32 = 24.0;
const INDENT_PER_DEPTH: f32 = 14.0;

/// Which folders are expanded; persisted in the project's workspace state.
#[derive(Debug, Default)]
pub struct FileTreeState {
    pub expanded: HashSet<PathBuf>,
}

impl FileTreeState {
    fn toggle(&mut self, folder: &Path) {
        if !self.expanded.remove(folder) {
            self.expanded.insert(folder.to_path_buf());
        }
    }

    /// Expands every ancestor of `path` below `root`, so the open note is visible.
    pub fn reveal(&mut self, root: &Path, path: &Path) {
        for ancestor in path.ancestors().skip(1) {
            if ancestor == root || !ancestor.starts_with(root) {
                break;
            }
            self.expanded.insert(ancestor.to_path_buf());
        }
    }
}

/// Inputs that stay fixed while drawing one frame of the tree.
struct TreeView<'a> {
    theme: &'a Theme,
    selected: Option<&'a Path>,
    selected_slides: Option<&'a Path>,
}

/// Which entries the tree highlights: the open note and the slides shown.
#[derive(Clone, Copy, Default)]
pub struct TreeSelection<'a> {
    pub note: Option<&'a Path>,
    pub slides: Option<&'a Path>,
}

/// Draws the tree below `root` (the root folder itself is not shown).
pub fn show_file_tree(
    ui: &mut egui::Ui,
    root: &FileNode,
    state: &mut FileTreeState,
    theme: &Theme,
    selection: TreeSelection,
) -> Option<FileTreeAction> {
    let FileKind::Folder(children) = &root.kind else {
        return None;
    };
    let view = TreeView {
        theme,
        selected: selection.note,
        selected_slides: selection.slides,
    };
    let mut action = None;
    egui::ScrollArea::vertical()
        .id_salt("file_tree_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for child in children {
                show_node(ui, child, 0, state, &view, &mut action);
            }
            if children.is_empty() {
                ui.label(egui::RichText::new("No notes yet.").color(theme.text_faint));
            }
            let blank_area = ui.allocate_response(
                ui.available_size().max(egui::vec2(0.0, ROW_HEIGHT)),
                Sense::click(),
            );
            blank_area.context_menu(|ui| folder_menu(ui, &root.path, &mut action));
        });
    action
}

fn show_node(
    ui: &mut egui::Ui,
    node: &FileNode,
    depth: usize,
    state: &mut FileTreeState,
    view: &TreeView,
    action: &mut Option<FileTreeAction>,
) {
    match &node.kind {
        FileKind::Folder(children) => {
            let is_expanded = state.expanded.contains(&node.path);
            let icon = if is_expanded { "⏷ 🗁" } else { "⏵ 🗀" };
            let row = TreeRow {
                depth,
                icon,
                label: &node.name,
                color: view.theme.text,
                is_selected: false,
            };
            let response = row.show(ui, view.theme);
            if response.clicked() {
                state.toggle(&node.path);
            }
            response.context_menu(|ui| {
                folder_menu(ui, &node.path, action);
                ui.separator();
                item_menu(ui, &node.path, action);
            });
            if is_expanded {
                for child in children {
                    show_node(ui, child, depth + 1, state, view, action);
                }
            }
        }
        FileKind::Note => {
            let is_selected = view.selected == Some(node.path.as_path());
            let label = note_display_name(&node.name);
            let response = TreeRow {
                depth,
                icon: "🗋",
                label,
                color: view.theme.text,
                is_selected,
            }
            .show(ui, view.theme);
            if response.clicked() {
                *action = Some(FileTreeAction::OpenNote(node.path.clone()));
            }
            response.context_menu(|ui| {
                let folder = node.path.parent().unwrap_or(&node.path);
                folder_menu(ui, folder, action);
                ui.separator();
                item_menu(ui, &node.path, action);
            });
        }
        FileKind::Other if qm_core::is_slide_deck(&node.path) => {
            let response = TreeRow {
                depth,
                icon: "🗎",
                label: &node.name,
                color: view.theme.text_muted,
                is_selected: view.selected_slides == Some(node.path.as_path()),
            }
            .show(ui, view.theme)
            .on_hover_text("Show in the slides pane");
            if response.clicked() {
                *action = Some(FileTreeAction::OpenSlides(node.path.clone()));
            }
        }
        FileKind::Other => {
            let row = TreeRow {
                depth,
                icon: "🗎",
                label: &node.name,
                color: view.theme.text_faint,
                is_selected: false,
            };
            row.show(ui, view.theme);
        }
    }
}

fn folder_menu(ui: &mut egui::Ui, folder: &Path, action: &mut Option<FileTreeAction>) {
    if ui.button("🗋  New note here").clicked() {
        *action = Some(FileTreeAction::NewNote {
            folder: folder.to_path_buf(),
        });
    }
    if ui.button("🗀  New folder here").clicked() {
        *action = Some(FileTreeAction::NewFolder {
            parent: folder.to_path_buf(),
        });
    }
}

fn item_menu(ui: &mut egui::Ui, path: &Path, action: &mut Option<FileTreeAction>) {
    if ui.button("✏  Rename…").clicked() {
        *action = Some(FileTreeAction::Rename(path.to_path_buf()));
    }
    if ui.button("🗑  Delete…").clicked() {
        *action = Some(FileTreeAction::Delete(path.to_path_buf()));
    }
}

/// Note file names are shown without the `.qm` extension, like Obsidian.
pub fn note_display_name(file_name: &str) -> &str {
    file_name
        .strip_suffix(&format!(".{}", qm_format::FILE_EXTENSION))
        .unwrap_or(file_name)
}

/// One full-width, clickable row of the tree.
struct TreeRow<'a> {
    depth: usize,
    icon: &'a str,
    label: &'a str,
    color: egui::Color32,
    is_selected: bool,
}

impl TreeRow<'_> {
    fn show(self, ui: &mut egui::Ui, theme: &Theme) -> egui::Response {
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let painter = ui.painter_at(rect);
        if let Some(fill) = self.background(&response, theme) {
            painter.rect_filled(rect, crate::theme_style::CORNER_RADIUS, fill);
        }
        let font = egui::TextStyle::Body.resolve(ui.style());
        let left = rect.left() + 6.0 + self.depth as f32 * INDENT_PER_DEPTH;
        let icon_pos = egui::pos2(left, rect.center().y);
        let icon_rect = painter.text(
            icon_pos,
            Align2::LEFT_CENTER,
            self.icon,
            font.clone(),
            theme.text_faint,
        );
        let label_color = if self.is_selected {
            theme.accent_text
        } else {
            self.color
        };
        let label_pos = egui::pos2(icon_rect.right() + 6.0, rect.center().y);
        painter.text(
            label_pos,
            Align2::LEFT_CENTER,
            self.label,
            font,
            label_color,
        );
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    fn background(&self, response: &egui::Response, theme: &Theme) -> Option<egui::Color32> {
        if self.is_selected {
            Some(theme.accent_soft)
        } else if response.hovered() {
            Some(theme.surface_raised)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_display_name_strips_only_the_qm_extension() {
        assert_eq!(note_display_name("Lecture 1.qm"), "Lecture 1");
        assert_eq!(note_display_name("figure.png"), "figure.png");
        assert_eq!(note_display_name("qm"), "qm");
    }

    #[test]
    fn reveal_expands_ancestors_inside_the_project_only() {
        let mut state = FileTreeState::default();
        let root = Path::new("/vault");
        state.reveal(root, Path::new("/vault/analysis/week1/limits.qm"));
        assert!(state.expanded.contains(Path::new("/vault/analysis")));
        assert!(state.expanded.contains(Path::new("/vault/analysis/week1")));
        assert!(!state.expanded.contains(root));
        assert_eq!(state.expanded.len(), 2);
    }

    #[test]
    fn toggle_flips_expansion() {
        let mut state = FileTreeState::default();
        let folder = Path::new("/vault/a");
        state.toggle(folder);
        assert!(state.expanded.contains(folder));
        state.toggle(folder);
        assert!(!state.expanded.contains(folder));
    }
}
