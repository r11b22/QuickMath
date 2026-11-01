//! An open project: file tree, open note, panes and dialogs, and the actions
//! that tie them together.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use egui::{Key, KeyboardShortcut, Modifiers, RichText};
use qm_core::{FileNode, NoteSlides, Project, WorkspaceState};
use qm_render::Theme;

use crate::editor::{EditorAction, EditorView};
use crate::export_dialog::{ExportDialog, ExportOutcome, ExportSource};
use crate::file_tree::{self, FileTreeAction, FileTreeState, TreeSelection};
use crate::help_panel::HelpPanel;
use crate::note::OpenNote;
use crate::outline;
use crate::preview_pane::{self, PreviewSync};
use crate::prompts::{self, Prompt, PromptOutcome};
use crate::settings_window::{SettingsModel, SettingsWindow, SpellingSettings};
use crate::slides_pane::{SlidesAction, SlidesPane};
use crate::spelling::SpellCheck;
use crate::status_bar;
use crate::toasts::Toasts;
use crate::toolbar::{self, ToolbarAction, ToolbarModel, ViewMode};

const SAVE_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
const TOGGLE_SLIDES_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::L);
const NEXT_SLIDE_SHORTCUT: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND, Key::PageDown);
const PREVIOUS_SLIDE_SHORTCUT: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND, Key::PageUp);
const SIDEBAR_DEFAULT_WIDTH: f32 = 250.0;
const OUTLINE_DEFAULT_HEIGHT: f32 = 220.0;
const HELP_DEFAULT_WIDTH: f32 = 340.0;
const MIN_PANE_WIDTH: f32 = 240.0;
const MIN_PANE_HEIGHT: f32 = 120.0;

/// Which panes are visible.
#[derive(Debug)]
struct PaneLayout {
    view_mode: ViewMode,
    is_sidebar_open: bool,
    is_help_open: bool,
    is_slides_open: bool,
}

/// Shared, read-mostly inputs for drawing one frame.
pub struct FrameContext<'a> {
    pub theme: &'a Theme,
    pub recent_projects: &'a [PathBuf],
    /// `None` while spell checking is off or its dictionaries are loading.
    pub spell_check: Option<SpellCheck<'a>>,
    pub spelling: SpellingSettings<'a>,
}

/// What the workspace passes up because only the app can handle it.
#[derive(Debug, Clone, PartialEq)]
pub enum WorkspaceAction {
    /// Theme, switching projects, quitting, spelling languages.
    Toolbar(ToolbarAction),
    Editor(EditorAction),
}

pub struct Workspace {
    project: Project,
    tree: Option<FileNode>,
    tree_state: FileTreeState,
    note: Option<OpenNote>,
    editor: EditorView,
    preview_sync: PreviewSync,
    layout: PaneLayout,
    prompt: Option<Prompt>,
    export: ExportDialog,
    settings: SettingsWindow,
    help: HelpPanel,
    slides: SlidesPane,
    /// The slide deck linked to each note, and its last viewed page.
    slide_links: BTreeMap<PathBuf, NoteSlides>,
    /// Every PDF in the project, for the slides pane's picker.
    slide_decks: Vec<PathBuf>,
}

impl Workspace {
    /// Opens the project at `root` and restores its last open note and expanded folders.
    pub fn open(root: &Path, toasts: &mut Toasts) -> qm_core::Result<Workspace> {
        let project = Project::open(root)?;
        let saved_state = project.load_workspace();
        let mut workspace = Workspace {
            project,
            tree: None,
            tree_state: FileTreeState {
                expanded: saved_state.expanded_folders.into_iter().collect(),
            },
            note: None,
            editor: EditorView::default(),
            preview_sync: PreviewSync::default(),
            layout: PaneLayout {
                view_mode: ViewMode::Split,
                is_sidebar_open: true,
                is_help_open: false,
                is_slides_open: false,
            },
            prompt: None,
            export: ExportDialog::default(),
            settings: SettingsWindow::default(),
            help: HelpPanel::default(),
            slides: SlidesPane::default(),
            slide_links: saved_state.note_slides,
            slide_decks: Vec::new(),
        };
        workspace.refresh_tree(toasts);
        if let Some(path) = saved_state.open_note.filter(|path| path.is_file()) {
            workspace.open_note(&path, toasts);
        }
        Ok(workspace)
    }

    pub fn window_title(&self) -> String {
        match &self.note {
            Some(note) => format!("QuickMath — {} — {}", self.project.name(), note.title()),
            None => format!("QuickMath — {}", self.project.name()),
        }
    }

    /// Saves the note and workspace state before the project closes or the app exits.
    /// Returns `false` if the note could not be saved (the error is reported).
    pub fn prepare_close(&mut self, toasts: &mut Toasts) -> bool {
        let is_saved = self.save_note(toasts);
        self.persist_state(toasts);
        is_saved
    }

    /// Draws the whole workspace. Returns the actions that concern the app.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        frame: &FrameContext,
        toasts: &mut Toasts,
    ) -> Option<WorkspaceAction> {
        if ui.input_mut(|input| input.consume_shortcut(&SAVE_SHORTCUT)) {
            self.save_note(toasts);
        }
        // Before the editor draws, so these keys never reach the text field.
        self.handle_slide_shortcuts(ui);
        let toolbar_action = self
            .show_toolbar(ui, frame)
            .and_then(|action| self.handle_toolbar_action(action, toasts));
        self.show_status_bar(ui, frame.theme);
        self.show_sidebar(ui, frame.theme, toasts);
        self.show_help_panel(ui, frame.theme);
        let editor_action = self.show_centre(ui, frame, toasts);
        self.remember_slides();
        self.show_dialogs(ui.ctx(), frame.theme, toasts);
        let settings_action = self
            .show_settings(ui.ctx(), frame)
            .and_then(|action| self.handle_toolbar_action(action, toasts));
        self.autosave(ui.ctx(), toasts);
        toolbar_action
            .or(settings_action)
            .map(WorkspaceAction::Toolbar)
            .or(editor_action.map(WorkspaceAction::Editor))
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui, frame: &FrameContext) -> Option<ToolbarAction> {
        let theme = frame.theme;
        let bar_frame = egui::Frame::new()
            .fill(theme.surface)
            .inner_margin(egui::Margin::symmetric(10, 6));
        let project_name = self.project.name();
        egui::Panel::top("toolbar")
            .frame(bar_frame)
            .show(ui, |ui| {
                let model = ToolbarModel {
                    project_name: &project_name,
                    recent_projects: frame.recent_projects,
                    view_mode: &mut self.layout.view_mode,
                    is_help_open: self.layout.is_help_open,
                    is_slides_open: self.layout.is_slides_open,
                };
                toolbar::show_toolbar(ui, model, theme)
            })
            .inner
    }

    fn show_settings(
        &mut self,
        ctx: &egui::Context,
        frame: &FrameContext,
    ) -> Option<ToolbarAction> {
        let model = SettingsModel {
            view_mode: &mut self.layout.view_mode,
            is_sidebar_open: self.layout.is_sidebar_open,
            is_help_open: self.layout.is_help_open,
            is_slides_open: self.layout.is_slides_open,
            spelling: frame.spelling,
        };
        self.settings.show(ctx, model, frame.theme)
    }

    /// Handles actions that stay inside the workspace; passes the rest on.
    fn handle_toolbar_action(
        &mut self,
        action: ToolbarAction,
        toasts: &mut Toasts,
    ) -> Option<ToolbarAction> {
        match action {
            ToolbarAction::Save => {
                self.save_note(toasts);
            }
            ToolbarAction::ToggleSidebar => {
                self.layout.is_sidebar_open = !self.layout.is_sidebar_open
            }
            ToolbarAction::ToggleHelp => self.layout.is_help_open = !self.layout.is_help_open,
            ToolbarAction::ToggleSlides => self.layout.is_slides_open = !self.layout.is_slides_open,
            ToolbarAction::OpenExport => self.export.open(),
            ToolbarAction::OpenSettings => self.settings.open(),
            other => return Some(other),
        }
        None
    }

    fn show_status_bar(&self, ui: &mut egui::Ui, theme: &Theme) {
        let bar_frame = egui::Frame::new()
            .fill(theme.surface)
            .inner_margin(egui::Margin::symmetric(12, 4));
        egui::Panel::bottom("status_bar")
            .frame(bar_frame)
            .show(ui, |ui| {
                status_bar::show_status_bar(ui, self.note.as_ref(), self.project.root(), theme);
            });
    }

    fn show_sidebar(&mut self, ui: &mut egui::Ui, theme: &Theme, toasts: &mut Toasts) {
        let sidebar_frame = egui::Frame::new()
            .fill(theme.surface)
            .inner_margin(egui::Margin::symmetric(8, 8));
        let mut is_open = self.layout.is_sidebar_open;
        egui::Panel::left("sidebar")
            .frame(sidebar_frame)
            .resizable(true)
            .default_size(SIDEBAR_DEFAULT_WIDTH)
            .min_size(160.0)
            .show_collapsible(ui, &mut is_open, |ui| {
                self.show_outline_section(ui, theme);
                self.show_files_section(ui, theme, toasts);
            });
        self.layout.is_sidebar_open = is_open;
    }

    fn show_files_section(&mut self, ui: &mut egui::Ui, theme: &Theme, toasts: &mut Toasts) {
        let mut action = None;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("FILES")
                    .small()
                    .strong()
                    .color(theme.text_faint),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let folder = self.default_new_item_folder();
                if ui.small_button("⟳").on_hover_text("Refresh").clicked() {
                    self.refresh_tree(toasts);
                }
                if ui.small_button("🗀+").on_hover_text("New folder").clicked() {
                    action = Some(FileTreeAction::NewFolder {
                        parent: folder.clone(),
                    });
                }
                if ui.small_button("🗋+").on_hover_text("New note").clicked() {
                    action = Some(FileTreeAction::NewNote { folder });
                }
            });
        });
        match &self.tree {
            Some(tree) => {
                let current_slides = self.slides.current();
                let selection = TreeSelection {
                    note: self.note.as_ref().map(OpenNote::path),
                    slides: current_slides.as_ref().map(|slides| slides.deck.as_path()),
                };
                if let Some(tree_action) =
                    file_tree::show_file_tree(ui, tree, &mut self.tree_state, theme, selection)
                {
                    action = Some(tree_action);
                }
            }
            None => {
                ui.label(RichText::new("Could not read the project folder.").color(theme.error));
            }
        }
        if let Some(action) = action {
            self.apply_tree_action(action, toasts);
        }
    }

    fn show_outline_section(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        let outline_frame = egui::Frame::new()
            .fill(theme.surface)
            .inner_margin(egui::Margin {
                top: 8,
                ..egui::Margin::ZERO
            });
        egui::Panel::bottom("outline")
            .frame(outline_frame)
            .resizable(true)
            .default_size(OUTLINE_DEFAULT_HEIGHT)
            .min_size(60.0)
            .show(ui, |ui| {
                ui.label(
                    RichText::new("OUTLINE")
                        .small()
                        .strong()
                        .color(theme.text_faint),
                );
                let Some(note) = &mut self.note else {
                    ui.label(
                        RichText::new("No note open.")
                            .color(theme.text_faint)
                            .small(),
                    );
                    return;
                };
                let entries = outline::extract_outline(note.document());
                if let Some(line) = outline::show_outline(ui, &entries, theme) {
                    note.requests.jump_to_line = Some(line);
                    self.preview_sync.request_scroll(line);
                }
            });
    }

    fn show_help_panel(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        let help_frame = egui::Frame::new()
            .fill(theme.surface)
            .inner_margin(egui::Margin::same(12));
        let mut is_open = self.layout.is_help_open;
        let inserted = egui::Panel::right("help_panel")
            .frame(help_frame)
            .resizable(true)
            .default_size(HELP_DEFAULT_WIDTH)
            .min_size(220.0)
            .show_collapsible(ui, &mut is_open, |ui| self.help.show(ui, theme))
            .and_then(|response| response.inner);
        self.layout.is_help_open = is_open;
        if let (Some(snippet), Some(note)) = (inserted, &mut self.note) {
            note.requests.insertion = Some(snippet.to_string());
            if !self.layout.view_mode.shows_editor() {
                self.layout.view_mode = ViewMode::Split;
            }
        }
    }

    fn show_centre(
        &mut self,
        ui: &mut egui::Ui,
        frame: &FrameContext,
        toasts: &mut Toasts,
    ) -> Option<EditorAction> {
        let theme = frame.theme;
        let spell_check = frame.spell_check;
        let is_slides_open = self.layout.is_slides_open;
        let slides_frame = egui::Frame::new()
            .fill(theme.surface)
            .inner_margin(egui::Margin::same(8));
        let mut slides_action = None;
        let Some(note) = &mut self.note else {
            if is_slides_open {
                slides_action = show_slides_below(ui, slides_frame, |ui| {
                    self.slides
                        .show(ui, &self.project, &self.slide_decks, theme)
                });
            }
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(theme.background))
                .show(ui, |ui| {
                    if show_no_note_placeholder(ui, theme) {
                        let folder = self.project.root().to_path_buf();
                        self.prompt = Some(Prompt::NewNote {
                            folder,
                            name: String::new(),
                        });
                    }
                });
            if let Some(action) = slides_action {
                self.apply_slides_action(action, toasts);
            }
            return None;
        };
        let editor_frame = egui::Frame::new().fill(theme.background);
        let preview_frame = egui::Frame::new().fill(theme.surface);
        let mut clicked_wikilink = None;
        let mut editor_action = None;
        let view_mode = self.layout.view_mode;
        // With the preview hidden, the slides take the space to the right of the editor.
        let is_editor_beside = match view_mode {
            ViewMode::Split => true,
            ViewMode::EditorOnly => is_slides_open,
            ViewMode::PreviewOnly => false,
        };
        if is_editor_beside {
            let width = ui.available_width();
            egui::Panel::left("editor_pane")
                .frame(editor_frame)
                .resizable(true)
                .default_size(width / 2.0)
                .size_range(MIN_PANE_WIDTH..=(width - MIN_PANE_WIDTH).max(MIN_PANE_WIDTH))
                .show(ui, |ui| {
                    editor_action = self.editor.show(ui, note, theme, spell_check);
                });
        }
        match view_mode {
            ViewMode::EditorOnly if is_slides_open => {
                egui::CentralPanel::default()
                    .frame(slides_frame)
                    .show(ui, |ui| {
                        slides_action =
                            self.slides
                                .show(ui, &self.project, &self.slide_decks, theme);
                    });
            }
            ViewMode::EditorOnly => {
                egui::CentralPanel::default()
                    .frame(editor_frame)
                    .show(ui, |ui| {
                        editor_action = self.editor.show(ui, note, theme, spell_check);
                    });
            }
            ViewMode::Split | ViewMode::PreviewOnly => {
                if is_slides_open {
                    slides_action = show_slides_below(ui, slides_frame, |ui| {
                        self.slides
                            .show(ui, &self.project, &self.slide_decks, theme)
                    });
                }
                egui::CentralPanel::default()
                    .frame(preview_frame)
                    .show(ui, |ui| {
                        clicked_wikilink =
                            preview_pane::show_preview(ui, note, &mut self.preview_sync, theme);
                    });
            }
        }
        if let Some(action) = slides_action {
            self.apply_slides_action(action, toasts);
        }
        if let Some(target) = clicked_wikilink {
            self.follow_wikilink(&target, toasts);
        }
        editor_action
    }

    fn show_dialogs(&mut self, ctx: &egui::Context, theme: &Theme, toasts: &mut Toasts) {
        let source = ExportSource {
            project: &self.project,
            note: self
                .note
                .as_ref()
                .map(|note| (note.document(), note.title())),
        };
        self.export.show(ctx, &source, theme);
        match self.export.poll(ctx) {
            Some(ExportOutcome::Succeeded(destination)) => {
                toasts.success(format!("Exported to {}", destination.display()));
            }
            Some(ExportOutcome::Failed(message)) => {
                toasts.error(format!("Export failed: {message}"))
            }
            None => {}
        }

        let Some(prompt) = &mut self.prompt else {
            return;
        };
        match prompts::show_prompt(ctx, prompt, theme) {
            PromptOutcome::Pending => {}
            PromptOutcome::Cancelled => self.prompt = None,
            PromptOutcome::Confirmed => {
                if let Some(prompt) = self.prompt.take() {
                    self.apply_prompt(prompt, toasts);
                }
            }
        }
    }

    fn autosave(&mut self, ctx: &egui::Context, toasts: &mut Toasts) {
        let Some(note) = &mut self.note else {
            return;
        };
        let now = ctx.input(|input| input.time);
        if let Some(Err(error)) = note.autosave_if_due(&self.project, now) {
            toasts.error(format!(
                "Autosave failed: {error}. Your text is kept; press Ctrl+S to retry."
            ));
        }
        if let Some(seconds) = note.seconds_until_autosave(now) {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(seconds));
        }
    }

    // ----- actions -----

    fn handle_slide_shortcuts(&mut self, ui: &mut egui::Ui) {
        let (toggle, next, previous) = ui.input_mut(|input| {
            (
                input.consume_shortcut(&TOGGLE_SLIDES_SHORTCUT),
                input.consume_shortcut(&NEXT_SLIDE_SHORTCUT),
                input.consume_shortcut(&PREVIOUS_SLIDE_SHORTCUT),
            )
        });
        if toggle {
            self.layout.is_slides_open = !self.layout.is_slides_open;
        }
        if !self.layout.is_slides_open {
            return;
        }
        if next {
            self.slides.next_page();
        }
        if previous {
            self.slides.previous_page();
        }
    }

    fn apply_slides_action(&mut self, action: SlidesAction, toasts: &mut Toasts) {
        match action {
            SlidesAction::PickDeck(deck) => self.show_slides(deck),
            SlidesAction::Failed(message) => {
                toasts.error(format!("Could not show the slides: {message}"))
            }
        }
    }

    /// Opens `deck` in the slides pane and links it to the open note. A deck
    /// that is already shown stays on its page; another starts at the first.
    fn show_slides(&mut self, deck: PathBuf) {
        let is_shown = self
            .slides
            .current()
            .is_some_and(|current| current.deck == deck);
        if !is_shown {
            self.slides.open(NoteSlides { deck, page: 0 });
        }
        self.layout.is_slides_open = true;
        self.remember_slides();
    }

    /// Links the deck and page shown to the open note, so they come back with it.
    fn remember_slides(&mut self) {
        let (Some(note), Some(current)) = (&self.note, self.slides.current()) else {
            return;
        };
        if self.slide_links.get(note.path()) != Some(&current) {
            self.slide_links.insert(note.path().to_path_buf(), current);
        }
    }

    /// Shows the slides linked to the open note, or none.
    fn restore_slides(&mut self) {
        let link = self
            .note
            .as_ref()
            .and_then(|note| self.slide_links.get(note.path()));
        match link {
            Some(link) => self.slides.open(link.clone()),
            None => self.slides.close(),
        }
    }

    /// Saves the open note. Returns `false` (and reports) if saving failed.
    pub fn save_note(&mut self, toasts: &mut Toasts) -> bool {
        let Some(note) = &mut self.note else {
            return true;
        };
        match note.save(&self.project) {
            Ok(()) => true,
            Err(error) => {
                toasts.error(format!("Could not save {}: {error}", note.title()));
                false
            }
        }
    }

    fn persist_state(&self, toasts: &mut Toasts) {
        let state = WorkspaceState {
            open_note: self.note.as_ref().map(|note| note.path().to_path_buf()),
            expanded_folders: self.tree_state.expanded.iter().cloned().collect(),
            note_slides: self.slide_links.clone(),
        };
        if let Err(error) = self.project.save_workspace(&state) {
            toasts.error(format!("Could not remember the workspace layout: {error}"));
        }
    }

    fn refresh_tree(&mut self, toasts: &mut Toasts) {
        match self.project.file_tree() {
            Ok(tree) => {
                self.slide_decks = tree.slide_decks();
                self.tree = Some(tree);
            }
            Err(error) => {
                toasts.error(format!("Could not read the project: {error}"));
                self.tree = None;
                self.slide_decks.clear();
            }
        }
    }

    /// Switches to the note at `path`, saving the current one first. If that
    /// save fails the switch is refused, so no text is ever lost.
    fn open_note(&mut self, path: &Path, toasts: &mut Toasts) {
        if self.note.as_ref().is_some_and(|note| note.path() == path) {
            return;
        }
        if !self.save_note(toasts) {
            return;
        }
        self.remember_slides();
        match OpenNote::load(&self.project, path) {
            Ok(note) => {
                self.tree_state.reveal(self.project.root(), path);
                self.note = Some(note);
                self.restore_slides();
                self.persist_state(toasts);
            }
            Err(error) => toasts.error(format!("Could not open note: {error}")),
        }
    }

    fn default_new_item_folder(&self) -> PathBuf {
        self.note
            .as_ref()
            .and_then(|note| note.path().parent())
            .map_or_else(|| self.project.root().to_path_buf(), Path::to_path_buf)
    }

    fn apply_tree_action(&mut self, action: FileTreeAction, toasts: &mut Toasts) {
        match action {
            FileTreeAction::OpenNote(path) => self.open_note(&path, toasts),
            FileTreeAction::OpenSlides(deck) => self.show_slides(deck),
            FileTreeAction::NewNote { folder } => {
                self.prompt = Some(Prompt::NewNote {
                    folder,
                    name: String::new(),
                })
            }
            FileTreeAction::NewFolder { parent } => {
                self.prompt = Some(Prompt::NewFolder {
                    parent,
                    name: String::new(),
                });
            }
            FileTreeAction::Rename(path) => self.prompt = Some(Prompt::rename(path)),
            FileTreeAction::Delete(path) => self.prompt = Some(Prompt::ConfirmDelete { path }),
        }
    }

    fn apply_prompt(&mut self, prompt: Prompt, toasts: &mut Toasts) {
        match prompt {
            Prompt::NewNote { folder, name } => self.create_note(&folder, name.trim(), toasts),
            Prompt::NewFolder { parent, name } => {
                if let Err(error) = self.project.create_folder(&parent, name.trim()) {
                    toasts.error(format!("Could not create folder: {error}"));
                }
                self.tree_state.expanded.insert(parent);
                self.refresh_tree(toasts);
            }
            Prompt::Rename { path, name } => self.rename(&path, name.trim(), toasts),
            Prompt::ConfirmDelete { path } => self.trash(&path, toasts),
            Prompt::CreateLinkedNote { target } => {
                let (folder, name) = linked_note_location(self.project.root(), &target);
                self.create_note(&folder, &name, toasts);
            }
        }
    }

    fn create_note(&mut self, folder: &Path, name: &str, toasts: &mut Toasts) {
        match self.project.create_note(folder, name) {
            Ok(path) => {
                self.refresh_tree(toasts);
                self.open_note(&path, toasts);
            }
            Err(error) => toasts.error(format!("Could not create note: {error}")),
        }
    }

    fn rename(&mut self, path: &Path, new_name: &str, toasts: &mut Toasts) {
        // Save first: writing the note after the rename would recreate the old file.
        if !self.save_note(toasts) {
            return;
        }
        match self.project.rename(path, new_name) {
            Ok(new_path) => {
                if let Some(note) = &mut self.note {
                    if let Some(updated) = moved_path(note.path(), path, &new_path) {
                        note.set_path(updated);
                    }
                }
                self.follow_rename_in_slides(path, &new_path);
                self.refresh_tree(toasts);
                self.persist_state(toasts);
            }
            Err(error) => toasts.error(format!("Could not rename: {error}")),
        }
    }

    /// Keeps slides links and the shown deck pointing at renamed notes and decks.
    fn follow_rename_in_slides(&mut self, old: &Path, new: &Path) {
        let links = std::mem::take(&mut self.slide_links);
        self.slide_links = moved_slide_links(links, old, new);
        let moved_deck = self
            .slides
            .current()
            .and_then(|current| moved_path(&current.deck, old, new));
        if let Some(deck) = moved_deck {
            self.slides.set_deck_path(deck);
        }
    }

    fn trash(&mut self, path: &Path, toasts: &mut Toasts) {
        let is_open_note_affected = self
            .note
            .as_ref()
            .is_some_and(|note| note.path().starts_with(path));
        if is_open_note_affected && !self.save_note(toasts) {
            return;
        }
        match self.project.trash(path) {
            Ok(()) => {
                if is_open_note_affected {
                    self.note = None;
                }
                self.refresh_tree(toasts);
                self.persist_state(toasts);
            }
            Err(error) => toasts.error(format!("Could not move to trash: {error}")),
        }
    }

    fn follow_wikilink(&mut self, target: &str, toasts: &mut Toasts) {
        match self.project.resolve_wikilink(target) {
            Some(path) => self.open_note(&path, toasts),
            None => {
                self.prompt = Some(Prompt::CreateLinkedNote {
                    target: target.to_string(),
                })
            }
        }
    }
}

/// Where `note_path` lives after `old` was renamed to `new`, if it was affected.
fn moved_path(note_path: &Path, old: &Path, new: &Path) -> Option<PathBuf> {
    note_path.strip_prefix(old).ok().map(|relative| {
        if relative.as_os_str().is_empty() {
            new.to_path_buf()
        } else {
            new.join(relative)
        }
    })
}

/// `links` with every note and deck path inside `old` moved to `new`.
fn moved_slide_links(
    links: BTreeMap<PathBuf, NoteSlides>,
    old: &Path,
    new: &Path,
) -> BTreeMap<PathBuf, NoteSlides> {
    links
        .into_iter()
        .map(|(note, mut slides)| {
            if let Some(deck) = moved_path(&slides.deck, old, new) {
                slides.deck = deck;
            }
            let note = moved_path(&note, old, new).unwrap_or(note);
            (note, slides)
        })
        .collect()
}

/// Draws the slides pane in a resizable strip along the bottom of the space left in `ui`.
fn show_slides_below<R>(
    ui: &mut egui::Ui,
    frame: egui::Frame,
    show_slides: impl FnOnce(&mut egui::Ui) -> Option<R>,
) -> Option<R> {
    let height = ui.available_height();
    egui::Panel::bottom("slides_pane")
        .frame(frame)
        .resizable(true)
        .default_size(height / 2.0)
        .size_range(MIN_PANE_HEIGHT..=(height - MIN_PANE_HEIGHT).max(MIN_PANE_HEIGHT))
        .show(ui, show_slides)
        .inner
}

/// Where to create the note for an unresolved `[[target]]`: `dir/name`
/// goes into `dir` below the project root, a bare name into the root.
fn linked_note_location(root: &Path, target: &str) -> (PathBuf, String) {
    let target = target.split('#').next().unwrap_or(target).trim();
    match target.rsplit_once('/') {
        Some((folder, name)) => (root.join(folder), name.trim().to_string()),
        None => (root.to_path_buf(), target.to_string()),
    }
}

/// Shown in the centre when no note is open. Returns whether "New note" was clicked.
fn show_no_note_placeholder(ui: &mut egui::Ui, theme: &Theme) -> bool {
    let mut clicked = false;
    ui.vertical_centered(|ui| {
        ui.add_space(ui.available_height() * 0.3);
        ui.label(
            RichText::new("No note open")
                .size(20.0)
                .color(theme.text_muted),
        );
        ui.label(
            RichText::new("Pick a note in the sidebar, or start a new one.")
                .color(theme.text_faint),
        );
        ui.add_space(12.0);
        let button =
            egui::Button::new(RichText::new("🗋  New note").color(theme.on_accent).strong())
                .fill(theme.accent);
        clicked = ui.add(button).clicked();
    });
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linked_note_location_uses_folder_part_and_drops_heading() {
        let root = Path::new("/v");
        assert_eq!(
            linked_note_location(root, "Limits"),
            (PathBuf::from("/v"), "Limits".into())
        );
        assert_eq!(
            linked_note_location(root, "week1/Limits#Epsilon"),
            (PathBuf::from("/v/week1"), "Limits".into())
        );
    }

    #[test]
    fn moved_path_follows_the_renamed_note_itself() {
        let moved = moved_path(
            Path::new("/v/a.qm"),
            Path::new("/v/a.qm"),
            Path::new("/v/b.qm"),
        );
        assert_eq!(moved, Some(PathBuf::from("/v/b.qm")));
    }

    #[test]
    fn moved_path_follows_a_renamed_parent_folder() {
        let moved = moved_path(
            Path::new("/v/week1/limits.qm"),
            Path::new("/v/week1"),
            Path::new("/v/week 1"),
        );
        assert_eq!(moved, Some(PathBuf::from("/v/week 1/limits.qm")));
    }

    #[test]
    fn moved_slide_links_follow_renamed_notes_and_decks() {
        let links = BTreeMap::from([
            (
                PathBuf::from("/v/week1/notes.qm"),
                NoteSlides {
                    deck: PathBuf::from("/v/week1/slides.pdf"),
                    page: 3,
                },
            ),
            (
                PathBuf::from("/v/other.qm"),
                NoteSlides {
                    deck: PathBuf::from("/v/other.pdf"),
                    page: 0,
                },
            ),
        ]);

        let moved = moved_slide_links(links, Path::new("/v/week1"), Path::new("/v/week 1"));

        assert_eq!(
            moved.get(Path::new("/v/week 1/notes.qm")),
            Some(&NoteSlides {
                deck: PathBuf::from("/v/week 1/slides.pdf"),
                page: 3
            })
        );
        assert_eq!(
            moved
                .get(Path::new("/v/other.qm"))
                .map(|slides| &slides.deck),
            Some(&PathBuf::from("/v/other.pdf"))
        );
    }

    #[test]
    fn moved_path_ignores_unrelated_renames() {
        assert_eq!(
            moved_path(
                Path::new("/v/a.qm"),
                Path::new("/v/b.qm"),
                Path::new("/v/c.qm")
            ),
            None
        );
        // A sibling whose name merely starts the same is not inside the folder.
        assert_eq!(
            moved_path(
                Path::new("/v/week10/x.qm"),
                Path::new("/v/week1"),
                Path::new("/v/w")
            ),
            None
        );
    }
}
