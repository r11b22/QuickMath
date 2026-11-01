//! The slides pane: a slide deck (PDF) from the project shown one page at a
//! time next to the notes, with a deck picker and page buttons.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use egui::{Align, Layout, RichText};
use qm_core::{NoteSlides, Project, SlideImage};
use qm_render::Theme;

use crate::slide_renderer::{RenderEvent, SlideRenderer};

/// Pages are rendered at widths rounded up to a multiple of this, so resizing
/// the pane by a few pixels does not render the page again.
const RENDER_WIDTH_STEP: u32 = 256;
const MAX_RENDER_WIDTH: u32 = 4096;
/// Height / width assumed until the first page is rendered (a 16:9 slide).
const DEFAULT_PAGE_ASPECT: f32 = 9.0 / 16.0;
/// Rendered pages further than this from the current page are dropped.
const KEPT_PAGES_AROUND: usize = 2;
const PICKER_RESERVED_WIDTH: f32 = 130.0;

/// What the user did in the pane, for the workspace to handle.
#[derive(Debug, Clone, PartialEq)]
pub enum SlidesAction {
    PickDeck(PathBuf),
    /// The open deck could not be read or rendered.
    Failed(String),
}

/// The open deck, if any, and its rendered pages.
#[derive(Default)]
pub struct SlidesPane {
    deck: Option<OpenDeck>,
}

struct OpenDeck {
    path: PathBuf,
    page: usize,
    /// `None` until the render thread has opened the file.
    page_count: Option<usize>,
    /// Started the first time the pane is drawn, so a hidden pane costs nothing.
    renderer: Option<SlideRenderer>,
    failure: Option<String>,
    textures: HashMap<usize, PageTexture>,
    /// Requests sent and not answered yet, as `(page, width)`.
    requested: HashSet<(usize, u32)>,
}

struct PageTexture {
    /// The width that was asked for (see `RenderEvent::Rendered`).
    width: u32,
    texture: egui::TextureHandle,
}

impl SlidesPane {
    /// Shows `slides`. Reopening the deck already shown only changes the page.
    pub fn open(&mut self, slides: NoteSlides) {
        if let Some(deck) = &mut self.deck {
            if deck.path == slides.deck && deck.failure.is_none() {
                deck.go_to(slides.page);
                return;
            }
        }
        self.deck = Some(OpenDeck::new(slides));
    }

    pub fn close(&mut self) {
        self.deck = None;
    }

    /// The deck shown and its current page.
    pub fn current(&self) -> Option<NoteSlides> {
        self.deck.as_ref().map(|deck| NoteSlides {
            deck: deck.path.clone(),
            page: deck.page,
        })
    }

    /// Keeps showing the open deck after its file was renamed or moved.
    pub fn set_deck_path(&mut self, path: PathBuf) {
        if let Some(deck) = &mut self.deck {
            deck.path = path;
        }
    }

    pub fn next_page(&mut self) {
        if let Some(deck) = &mut self.deck {
            deck.go_to(deck.page + 1);
        }
    }

    pub fn previous_page(&mut self) {
        if let Some(deck) = &mut self.deck {
            deck.go_to(deck.page.saturating_sub(1));
        }
    }

    /// Draws the pane. `decks` are the PDFs in the project, for the picker.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        project: &Project,
        decks: &[PathBuf],
        theme: &Theme,
    ) -> Option<SlidesAction> {
        let failure = self.update(ui.ctx(), project);
        let mut action = failure.map(SlidesAction::Failed);
        ui.horizontal(|ui| {
            if let Some(picked) = self.show_picker(ui, project.root(), decks) {
                action = Some(SlidesAction::PickDeck(picked));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                self.show_page_buttons(ui, theme);
            });
        });
        ui.separator();
        match &mut self.deck {
            None => show_hint(
                ui,
                theme,
                "Choose a PDF above, or click one in the file tree.",
            ),
            Some(deck) => deck.show_page(ui, theme),
        }
        action
    }

    /// Starts the renderer if needed and collects its events. Returns a
    /// failure that just happened.
    fn update(&mut self, ctx: &egui::Context, project: &Project) -> Option<String> {
        let deck = self.deck.as_mut()?;
        if deck.renderer.is_none() && deck.failure.is_none() {
            deck.renderer = Some(SlideRenderer::start(project, &deck.path, ctx));
        }
        let events = deck.renderer.as_ref()?.poll();
        events.into_iter().find_map(|event| deck.apply(ctx, event))
    }

    fn show_picker(&self, ui: &mut egui::Ui, root: &Path, decks: &[PathBuf]) -> Option<PathBuf> {
        let current = self.deck.as_ref().map(|deck| deck.path.as_path());
        let selected_text = current.map_or("Choose slides…".to_string(), |path| {
            relative_name(root, path)
        });
        let mut picked = None;
        egui::ComboBox::from_id_salt("slides_deck_picker")
            .selected_text(selected_text)
            .width((ui.available_width() - PICKER_RESERVED_WIDTH).max(80.0))
            .truncate()
            .show_ui(ui, |ui| {
                if decks.is_empty() {
                    ui.label("No PDF files in this project");
                }
                for deck in decks {
                    let is_current = current == Some(deck.as_path());
                    let name = relative_name(root, deck);
                    if ui.selectable_label(is_current, name).clicked() && !is_current {
                        picked = Some(deck.clone());
                    }
                }
            });
        picked
    }

    /// Laid out right to left: next, page number, previous.
    fn show_page_buttons(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        let Some(deck) = &self.deck else {
            return;
        };
        let Some(page_count) = deck.page_count else {
            return;
        };
        let is_last = deck.page + 1 >= page_count;
        let next = ui
            .add_enabled(!is_last, egui::Button::new("▶"))
            .on_hover_text("Next slide (Ctrl+Page Down)");
        ui.label(
            RichText::new(format!("{} / {page_count}", deck.page + 1)).color(theme.text_muted),
        );
        let previous = ui
            .add_enabled(deck.page > 0, egui::Button::new("◀"))
            .on_hover_text("Previous slide (Ctrl+Page Up)");
        if next.clicked() {
            self.next_page();
        }
        if previous.clicked() {
            self.previous_page();
        }
    }
}

impl OpenDeck {
    fn new(slides: NoteSlides) -> OpenDeck {
        OpenDeck {
            path: slides.deck,
            page: slides.page,
            page_count: None,
            renderer: None,
            failure: None,
            textures: HashMap::new(),
            requested: HashSet::new(),
        }
    }

    /// Goes to `page`, kept inside the deck once its length is known.
    fn go_to(&mut self, page: usize) {
        self.page = match self.page_count {
            Some(count) => page.min(count.saturating_sub(1)),
            None => page,
        };
        let current = self.page;
        self.textures
            .retain(|&page, _| page.abs_diff(current) <= KEPT_PAGES_AROUND);
    }

    /// Applies one event from the renderer. Returns the message of a failure.
    fn apply(&mut self, ctx: &egui::Context, event: RenderEvent) -> Option<String> {
        match event {
            RenderEvent::Opened { page_count } => {
                self.page_count = Some(page_count);
                self.go_to(self.page);
            }
            RenderEvent::Rendered { page, width, image } => {
                self.requested.remove(&(page, width));
                if page.abs_diff(self.page) <= KEPT_PAGES_AROUND {
                    let texture = load_texture(ctx, &self.path, page, &image);
                    self.textures.insert(page, PageTexture { width, texture });
                }
            }
            RenderEvent::Failed(message) => {
                self.renderer = None;
                self.failure = Some(message.clone());
                return Some(message);
            }
        }
        None
    }

    fn show_page(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        if let Some(failure) = &self.failure {
            show_hint(ui, theme, &format!("Could not show the slides: {failure}"));
            return;
        }
        let Some(page_count) = self.page_count else {
            ui.centered_and_justified(|ui| ui.spinner());
            return;
        };
        if page_count == 0 {
            show_hint(ui, theme, "This PDF has no pages.");
            return;
        }
        let aspect = self
            .textures
            .values()
            .next()
            .map_or(DEFAULT_PAGE_ASPECT, |page| {
                let [width, height] = page.texture.size();
                height as f32 / width.max(1) as f32
            });
        let size = fit_size(ui.available_size(), aspect);
        let width = render_width(size.x * ui.pixels_per_point());
        self.request_if_missing(self.page, width);
        if self.page + 1 < page_count {
            self.request_if_missing(self.page + 1, width);
        }
        match self.textures.get(&self.page) {
            Some(page) => {
                ui.centered_and_justified(|ui| {
                    ui.add(egui::Image::new(&page.texture).fit_to_exact_size(size));
                });
            }
            None => {
                ui.centered_and_justified(|ui| ui.spinner());
            }
        }
    }

    /// Asks for `page` at `width` unless it is rendered at that width or on its way.
    fn request_if_missing(&mut self, page: usize, width: u32) {
        let is_rendered = self
            .textures
            .get(&page)
            .is_some_and(|texture| texture.width == width);
        let Some(renderer) = &self.renderer else {
            return;
        };
        if is_rendered || !self.requested.insert((page, width)) {
            return;
        }
        renderer.request(page, width);
    }
}

fn load_texture(
    ctx: &egui::Context,
    deck: &Path,
    page: usize,
    image: &SlideImage,
) -> egui::TextureHandle {
    let size = [image.width as usize, image.height as usize];
    let color_image = egui::ColorImage::from_rgba_premultiplied(size, &image.rgba_premultiplied);
    let name = format!("slide {} p{page}", deck.display());
    ctx.load_texture(name, color_image, egui::TextureOptions::LINEAR)
}

fn show_hint(ui: &mut egui::Ui, theme: &Theme, text: &str) {
    ui.centered_and_justified(|ui| {
        ui.label(RichText::new(text).color(theme.text_faint));
    });
}

/// The largest size with this `aspect` (height / width) that fits in `available`.
fn fit_size(available: egui::Vec2, aspect: f32) -> egui::Vec2 {
    let width = available.x.min(available.y / aspect).max(1.0);
    egui::vec2(width, width * aspect)
}

/// The width in pixels to render a page shown `display_width` pixels wide.
fn render_width(display_width: f32) -> u32 {
    let width = display_width.max(1.0).ceil() as u32;
    width
        .div_ceil(RENDER_WIDTH_STEP)
        .saturating_mul(RENDER_WIDTH_STEP)
        .min(MAX_RENDER_WIDTH)
}

/// `path` relative to the project root with `/` separators, as shown in the picker.
fn relative_name(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slides(deck: &str, page: usize) -> NoteSlides {
        NoteSlides {
            deck: PathBuf::from(deck),
            page,
        }
    }

    fn opened_pane(deck: &str, page: usize, page_count: usize) -> SlidesPane {
        let mut pane = SlidesPane::default();
        pane.open(slides(deck, page));
        let ctx = egui::Context::default();
        let deck = pane.deck.as_mut().unwrap();
        deck.apply(&ctx, RenderEvent::Opened { page_count });
        pane
    }

    #[test]
    fn page_buttons_stay_inside_the_deck() {
        let mut pane = opened_pane("/v/a.pdf", 0, 3);

        pane.previous_page();
        assert_eq!(pane.current(), Some(slides("/v/a.pdf", 0)));
        pane.next_page();
        pane.next_page();
        pane.next_page();
        assert_eq!(pane.current(), Some(slides("/v/a.pdf", 2)));
    }

    #[test]
    fn a_saved_page_past_the_end_moves_to_the_last_page() {
        let pane = opened_pane("/v/a.pdf", 9, 4);

        assert_eq!(pane.current(), Some(slides("/v/a.pdf", 3)));
    }

    #[test]
    fn reopening_the_same_deck_keeps_its_page_count() {
        let mut pane = opened_pane("/v/a.pdf", 0, 5);

        pane.open(slides("/v/a.pdf", 7));

        assert_eq!(pane.current(), Some(slides("/v/a.pdf", 4)));
    }

    #[test]
    fn opening_another_deck_starts_fresh() {
        let mut pane = opened_pane("/v/a.pdf", 0, 2);

        pane.open(slides("/v/b.pdf", 6));

        assert_eq!(pane.current(), Some(slides("/v/b.pdf", 6)));
        assert_eq!(pane.deck.as_ref().unwrap().page_count, None);
    }

    #[test]
    fn a_failure_is_reported_once_and_kept_for_display() {
        let mut pane = opened_pane("/v/a.pdf", 0, 2);
        let ctx = egui::Context::default();
        let deck = pane.deck.as_mut().unwrap();

        let reported = deck.apply(&ctx, RenderEvent::Failed("broken".to_string()));

        assert_eq!(reported, Some("broken".to_string()));
        assert_eq!(deck.failure.as_deref(), Some("broken"));
        assert!(deck.renderer.is_none());
    }

    #[test]
    fn reopening_a_failed_deck_tries_again() {
        let mut pane = opened_pane("/v/a.pdf", 0, 2);
        let ctx = egui::Context::default();
        let deck = pane.deck.as_mut().unwrap();
        deck.apply(&ctx, RenderEvent::Failed("missing".to_string()));

        pane.open(slides("/v/a.pdf", 0));

        assert_eq!(pane.deck.as_ref().unwrap().failure, None);
    }

    #[test]
    fn rendered_pages_far_from_the_current_page_are_not_kept() {
        let mut pane = opened_pane("/v/a.pdf", 0, 10);
        let ctx = egui::Context::default();
        let image = SlideImage {
            width: 1,
            height: 1,
            rgba_premultiplied: vec![255; 4],
        };
        let deck = pane.deck.as_mut().unwrap();
        for page in [0, 1, 7] {
            let event = RenderEvent::Rendered {
                page,
                width: 256,
                image: image.clone(),
            };
            deck.apply(&ctx, event);
        }
        assert_eq!(deck.textures.len(), 2);

        deck.go_to(5);

        assert!(deck.textures.is_empty());
    }

    #[test]
    fn render_width_rounds_up_to_steps_and_is_capped() {
        assert_eq!(render_width(0.0), RENDER_WIDTH_STEP);
        assert_eq!(render_width(256.0), 256);
        assert_eq!(render_width(257.0), 512);
        assert_eq!(render_width(1.0e9), MAX_RENDER_WIDTH);
    }

    #[test]
    fn fit_size_is_limited_by_the_tighter_side() {
        let wide_pane = fit_size(egui::vec2(800.0, 300.0), 0.5);
        assert_eq!(wide_pane, egui::vec2(600.0, 300.0));
        let tall_pane = fit_size(egui::vec2(400.0, 900.0), 0.5);
        assert_eq!(tall_pane, egui::vec2(400.0, 200.0));
    }

    #[test]
    fn relative_name_uses_slashes_below_the_root() {
        let name = relative_name(Path::new("/v"), Path::new("/v/week 1/slides.pdf"));
        assert_eq!(name, "week 1/slides.pdf");
    }
}
