//! Slide decks: PDF files in a project, rendered one page at a time so they
//! can be shown next to the notes taken during the lecture.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::{LoadPdfError, Pdf};
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{RenderCache, RenderSettings};

use crate::{CoreError, Result};

const SLIDES_EXTENSION: &str = "pdf";

/// The renderer works with 16-bit pixel sizes.
const MAX_PIXEL_SIZE: f32 = u16::MAX as f32;

/// Whether `path` names a slide deck (a PDF file, by extension, case-insensitively).
pub fn is_slide_deck(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case(SLIDES_EXTENSION))
}

/// One rendered page: `width × height` pixels, four bytes (premultiplied
/// RGBA) per pixel, row by row. Pages are drawn on a white background, so
/// every pixel is opaque.
#[derive(Debug, Clone, PartialEq)]
pub struct SlideImage {
    pub width: u32,
    pub height: u32,
    pub rgba_premultiplied: Vec<u8>,
}

/// An opened slide deck. Open one with [`crate::Project::open_slides`].
pub struct SlideDeck {
    path: PathBuf,
    pdf: Pdf,
}

impl SlideDeck {
    /// Parses the PDF `bytes` read from `path`.
    ///
    /// Fails with [`CoreError::Slides`] for files that are not a readable PDF,
    /// including password-protected ones.
    pub(crate) fn from_bytes(path: &Path, bytes: Vec<u8>) -> Result<SlideDeck> {
        let pdf = Pdf::new(bytes).map_err(|error| {
            let message = match error {
                LoadPdfError::Decryption(_) => "password-protected PDFs are not supported",
                LoadPdfError::Invalid => "not a readable PDF file",
            };
            slides_error(path, message)
        })?;
        Ok(SlideDeck {
            path: path.to_path_buf(),
            pdf,
        })
    }

    pub fn page_count(&self) -> usize {
        self.pdf.pages().len()
    }

    /// Renders page `index` (zero-based) `width` pixels wide, keeping its
    /// aspect ratio. The result may be a pixel narrower than asked because of
    /// rounding, and is capped at the renderer's maximum size.
    ///
    /// Fails with [`CoreError::Slides`] if the deck has no such page.
    pub fn render_page(&self, index: usize, width: u32) -> Result<SlideImage> {
        let pages = self.pdf.pages();
        let page = pages
            .get(index)
            .ok_or_else(|| slides_error(&self.path, &format!("has no page {}", index + 1)))?;
        let (page_width, page_height) = page.render_dimensions();
        let scale = fitting_scale(page_width, page_height, width);
        let render_settings = RenderSettings {
            x_scale: scale,
            y_scale: scale,
            bg_color: WHITE,
            ..RenderSettings::default()
        };
        // The cache borrows from the page, so it cannot outlive this call.
        let cache = RenderCache::new();
        let pixmap = hayro::render(
            page,
            &cache,
            &InterpreterSettings::default(),
            &render_settings,
        );
        Ok(SlideImage {
            width: u32::from(pixmap.width()),
            height: u32::from(pixmap.height()),
            rgba_premultiplied: pixmap.data_as_u8_slice().to_vec(),
        })
    }
}

/// The scale that makes a `page_width × page_height` page `width` pixels
/// wide, reduced if needed so neither side exceeds the renderer's limit.
fn fitting_scale(page_width: f32, page_height: f32, width: u32) -> f32 {
    if page_width <= 0.0 || page_height <= 0.0 {
        return 1.0;
    }
    let scale = width.max(1) as f32 / page_width;
    scale
        .min(MAX_PIXEL_SIZE / page_width)
        .min(MAX_PIXEL_SIZE / page_height)
}

fn slides_error(path: &Path, message: &str) -> CoreError {
    CoreError::Slides {
        path: path.to_path_buf(),
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_slide_deck_matches_pdf_extension_only() {
        assert!(is_slide_deck(Path::new("week1/lecture.pdf")));
        assert!(is_slide_deck(Path::new("LECTURE.PDF")));
        assert!(!is_slide_deck(Path::new("lecture.qm")));
        assert!(!is_slide_deck(Path::new("pdf")));
    }

    #[test]
    fn fitting_scale_reaches_the_requested_width() {
        assert_eq!(fitting_scale(400.0, 300.0, 800), 2.0);
    }

    #[test]
    fn fitting_scale_stays_within_the_renderer_limit() {
        let scale = fitting_scale(10.0, 100.0, 100_000);
        assert!(100.0 * scale <= MAX_PIXEL_SIZE);
    }

    #[test]
    fn fitting_scale_survives_empty_pages() {
        assert_eq!(fitting_scale(0.0, 0.0, 800), 1.0);
    }
}
