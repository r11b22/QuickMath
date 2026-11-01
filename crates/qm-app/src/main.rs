//! QuickMath: a desktop editor for taking math notes quickly during lectures.

mod app;
mod autosave;
mod completion_popup;
mod editor;
mod export_dialog;
mod file_tree;
mod help_panel;
mod highlight;
mod misspellings;
mod note;
mod outline;
mod practice;
mod preview_pane;
mod prompts;
mod settings_window;
mod spelling;
mod status_bar;
mod text_metrics;
mod theme_style;
mod toasts;
mod toolbar;
mod welcome;
mod workspace;

const DEFAULT_WINDOW_SIZE: [f32; 2] = [1400.0, 860.0];
const MIN_WINDOW_SIZE: [f32; 2] = [760.0, 480.0];

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("QuickMath")
            .with_app_id("quick-math")
            .with_inner_size(DEFAULT_WINDOW_SIZE)
            .with_min_inner_size(MIN_WINDOW_SIZE),
        ..eframe::NativeOptions::default()
    };
    eframe::run_native(
        "QuickMath",
        options,
        Box::new(|creation| Ok(Box::new(app::QuickMathApp::new(creation)))),
    )
}
