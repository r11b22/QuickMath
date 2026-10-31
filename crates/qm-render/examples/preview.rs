//! Interactive preview of `docs/examples/lecture.qm` (or the hand-built sample).
//!
//! ```sh
//! cargo run -p qm-render --example preview
//! ```

mod shared;

use eframe::egui;
use qm_format::Document;
use qm_render::{PreviewOptions, PreviewResponse, Theme};

const LECTURE_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/examples/lecture.qm"
);

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([960.0, 900.0]),
        ..Default::default()
    };
    eframe::run_native(
        "QuickMath preview",
        options,
        Box::new(|creation| {
            let mut fonts = egui::FontDefinitions::default();
            qm_render::install_fonts(&mut fonts);
            creation.egui_ctx.set_fonts(fonts);
            Ok(Box::new(PreviewApp::new()))
        }),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    Lecture,
    Sample,
}

struct PreviewApp {
    source: Source,
    document: Document,
    is_dark: bool,
    last_response: PreviewResponse,
    scroll_line: String,
}

impl PreviewApp {
    fn new() -> Self {
        let mut app = PreviewApp {
            source: Source::Lecture,
            document: Document::default(),
            is_dark: false,
            last_response: PreviewResponse::default(),
            scroll_line: String::new(),
        };
        app.reload();
        app
    }

    fn reload(&mut self) {
        self.document = match (self.source, std::fs::read_to_string(LECTURE_PATH)) {
            (Source::Lecture, Ok(text)) => qm_format::parse_document(&text),
            _ => shared::sample_document(),
        };
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) -> Option<usize> {
        let mut scroll_to = None;
        ui.horizontal(|ui| {
            let before = self.source;
            ui.selectable_value(&mut self.source, Source::Lecture, "lecture.qm");
            ui.selectable_value(&mut self.source, Source::Sample, "hand-built sample");
            if before != self.source || ui.button("Reload").clicked() {
                self.reload();
            }
            ui.checkbox(&mut self.is_dark, "Dark");
            ui.label("Scroll to line:");
            if ui.text_edit_singleline(&mut self.scroll_line).lost_focus() {
                scroll_to = self.scroll_line.trim().parse().ok();
            }
            ui.label(format!("last: {:?}", self.last_response));
        });
        scroll_to
    }
}

impl eframe::App for PreviewApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let scroll_to_line = egui::Panel::top("toolbar")
            .show(ui, |ui| self.toolbar(ui))
            .inner;
        let theme = if self.is_dark {
            Theme::dark()
        } else {
            Theme::light()
        };
        egui::CentralPanel::default().show(ui, |ui| {
            let response = qm_render::show_document(
                ui,
                &self.document,
                &theme,
                &PreviewOptions { scroll_to_line },
            );
            if response != PreviewResponse::default() {
                self.last_response = response;
            }
        });
    }
}
