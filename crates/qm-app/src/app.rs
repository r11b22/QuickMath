//! Top-level app state: settings, theme, spell checker, and which screen is showing.

use std::path::{Path, PathBuf};

use qm_core::AppSettings;
use qm_render::Theme;

use crate::editor::EditorAction;
use crate::practice::{PracticeAction, PracticeScreen};
use crate::settings_window::SpellingSettings;
use crate::spelling::Spelling;
use crate::theme_style;
use crate::toasts::Toasts;
use crate::toolbar::ToolbarAction;
use crate::welcome::{self, WelcomeAction, WelcomeScreen};
use crate::workspace::{FrameContext, Workspace, WorkspaceAction};

enum Screen {
    Welcome(WelcomeScreen),
    Workspace(Box<Workspace>),
}

pub struct QuickMathApp {
    settings: AppSettings,
    theme: Theme,
    spelling: Spelling,
    screen: Screen,
    /// The practice game, shown instead of `screen` while open.
    practice: Option<Box<PracticeScreen>>,
    toasts: Toasts,
    window_title: String,
    /// Set after a close attempt failed to save; the next close quits anyway.
    is_close_forced: bool,
}

impl QuickMathApp {
    pub fn new(creation: &eframe::CreationContext<'_>) -> QuickMathApp {
        let ctx = &creation.egui_ctx;
        let mut fonts = egui::FontDefinitions::default();
        qm_render::install_fonts(&mut fonts);
        ctx.set_fonts(fonts);

        let settings = AppSettings::load();
        theme_style::apply_theme(ctx, settings.dark_mode);
        let mut toasts = Toasts::default();
        let spelling = Spelling::start(&settings.spelling_languages, &mut toasts);
        let mut app = QuickMathApp {
            theme: theme_style::palette(settings.dark_mode),
            settings,
            spelling,
            screen: Screen::Welcome(WelcomeScreen::default()),
            practice: None,
            toasts,
            window_title: String::new(),
            is_close_forced: false,
        };
        app.reopen_last_project();
        app
    }

    fn reopen_last_project(&mut self) {
        let last = self
            .settings
            .open_project
            .clone()
            .filter(|root| root.is_dir());
        if let Some(root) = last {
            self.open_project(&root);
        }
    }

    /// Closes the current project (saving it) and opens `root`.
    fn open_project(&mut self, root: &Path) {
        if !self.close_project() {
            return;
        }
        match Workspace::open(root, &mut self.toasts) {
            Ok(workspace) => {
                self.screen = Screen::Workspace(Box::new(workspace));
                self.settings.remember_project(root);
                self.settings.open_project = Some(root.to_path_buf());
                self.save_settings();
            }
            Err(error) => self
                .toasts
                .error(format!("Could not open {}: {error}", root.display())),
        }
    }

    /// Returns to the welcome screen. Refuses (returning `false`) if the open note can't be saved.
    fn close_project(&mut self) -> bool {
        if let Screen::Workspace(workspace) = &mut self.screen {
            if !workspace.prepare_close(&mut self.toasts) {
                return false;
            }
            self.screen = Screen::Welcome(WelcomeScreen::default());
        }
        true
    }

    fn save_settings(&mut self) {
        if let Err(error) = self.settings.save() {
            self.toasts
                .error(format!("Could not save settings: {error}"));
        }
    }

    fn toggle_theme(&mut self, ctx: &egui::Context) {
        self.settings.dark_mode = !self.settings.dark_mode;
        self.theme = theme_style::palette(self.settings.dark_mode);
        theme_style::apply_theme(ctx, self.settings.dark_mode);
        self.save_settings();
    }

    fn toggle_spelling_language(&mut self, code: &str) {
        self.settings.toggle_spelling_language(code);
        self.spelling
            .use_languages(&self.settings.spelling_languages, &mut self.toasts);
        self.save_settings();
    }

    fn handle_workspace_action(&mut self, ctx: &egui::Context, action: WorkspaceAction) {
        match action {
            WorkspaceAction::Toolbar(action) => self.handle_toolbar_action(ctx, action),
            WorkspaceAction::Editor(EditorAction::AddToDictionary(word)) => {
                self.spelling.add_word(&word, &mut self.toasts)
            }
        }
    }

    fn handle_toolbar_action(&mut self, ctx: &egui::Context, action: ToolbarAction) {
        match action {
            ToolbarAction::OpenProject => self.pick_and_open_project(),
            ToolbarAction::NewProject => {
                if self.close_project() {
                    self.screen = Screen::Welcome(WelcomeScreen::with_new_project_form());
                }
            }
            ToolbarAction::OpenRecent(root) => self.open_project(&root),
            ToolbarAction::CloseProject => {
                if self.close_project() {
                    self.settings.open_project = None;
                    self.save_settings();
                }
            }
            ToolbarAction::ToggleTheme => self.toggle_theme(ctx),
            ToolbarAction::OpenPractice => self.open_practice(),
            ToolbarAction::ToggleSpellingLanguage(code) => self.toggle_spelling_language(&code),
            ToolbarAction::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            // Handled inside the workspace.
            ToolbarAction::Save
            | ToolbarAction::ToggleSidebar
            | ToolbarAction::ToggleHelp
            | ToolbarAction::OpenExport
            | ToolbarAction::OpenSettings => {}
        }
    }

    fn handle_welcome_action(&mut self, action: WelcomeAction) {
        match action {
            WelcomeAction::OpenProjectDialog => self.pick_and_open_project(),
            WelcomeAction::OpenPractice => self.open_practice(),
            WelcomeAction::OpenProject(root) => self.open_project(&root),
            WelcomeAction::CreateProject { parent, name } => {
                match welcome::create_project_folder(&parent, &name) {
                    Ok(root) => self.open_project(&root),
                    Err(message) => self.toasts.error(message),
                }
            }
        }
    }

    /// Shows the practice game over the current screen. The open note is
    /// saved first, because the workspace (and its autosave) pauses meanwhile.
    fn open_practice(&mut self) {
        if let Screen::Workspace(workspace) = &mut self.screen {
            workspace.save_note(&mut self.toasts);
        }
        self.practice = Some(Box::new(PracticeScreen::open()));
    }

    fn pick_and_open_project(&mut self) {
        if let Some(root) = rfd::FileDialog::new()
            .set_title("Open project folder")
            .pick_folder()
        {
            self.open_project(&root);
        }
    }

    /// Saves everything when the window is closing. If saving fails, the
    /// first close is cancelled so the user can react; a second one quits.
    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.viewport().close_requested()) {
            return;
        }
        let is_saved = match &mut self.screen {
            Screen::Workspace(workspace) => workspace.prepare_close(&mut self.toasts),
            Screen::Welcome(_) => true,
        };
        self.save_settings();
        if !is_saved && !self.is_close_forced {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.is_close_forced = true;
            self.toasts.error(
                "Your note could not be saved. Close the window again to quit without saving.",
            );
        }
    }

    fn update_window_title(&mut self, ctx: &egui::Context) {
        let title = match (&self.practice, &self.screen) {
            (Some(_), _) => "QuickMath — Practice".to_string(),
            (None, Screen::Workspace(workspace)) => workspace.window_title(),
            (None, Screen::Welcome(_)) => "QuickMath".to_string(),
        };
        if title != self.window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
    }

    fn show_screen(&mut self, ui: &mut egui::Ui) {
        if let Some(practice) = &mut self.practice {
            if practice.show(ui, &self.theme, &mut self.toasts) == Some(PracticeAction::Close) {
                self.practice = None;
            }
            return;
        }
        let recent_projects: Vec<PathBuf> = self.settings.recent_projects.clone();
        match &mut self.screen {
            Screen::Workspace(workspace) => {
                let frame = FrameContext {
                    theme: &self.theme,
                    recent_projects: &recent_projects,
                    spell_check: self.spelling.current(),
                    spelling: SpellingSettings {
                        available: self.spelling.available(),
                        enabled: &self.settings.spelling_languages,
                    },
                };
                if let Some(action) = workspace.show(ui, &frame, &mut self.toasts) {
                    self.handle_workspace_action(ui.ctx(), action);
                }
            }
            Screen::Welcome(welcome) => {
                let panel_frame = egui::Frame::new()
                    .fill(self.theme.background)
                    .inner_margin(egui::Margin::same(24));
                let action = egui::CentralPanel::default()
                    .frame(panel_frame)
                    .show(ui, |ui| welcome.show(ui, &recent_projects, &self.theme))
                    .inner;
                if let Some(action) = action {
                    self.handle_welcome_action(action);
                }
            }
        }
    }
}

impl eframe::App for QuickMathApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.spelling.poll(ui.ctx(), &mut self.toasts);
        self.show_screen(ui);
        self.toasts.show(ui.ctx(), &self.theme);
        self.update_window_title(ui.ctx());
        self.handle_close_request(ui.ctx());
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.theme.background.to_normalized_gamma_f32()
    }
}
