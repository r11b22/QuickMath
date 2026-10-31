//! Text measurement: the only place layout talks to egui's font system.

use egui::epaint::text::FontsView;
use egui::{pos2, vec2, Color32, FontId, Rect, Vec2};

/// Size and ink extents of a run of text.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measured {
    /// Advance width.
    pub width: f32,
    /// Bounding box of the drawn glyphs, relative to the left end of the
    /// baseline (y grows downwards). Zero-sized for blank text.
    pub ink: Rect,
    /// Where the galley's top-left corner sits relative to the baseline origin.
    pub galley_offset: Vec2,
}

impl Measured {
    /// Height of the ink above the baseline (never negative).
    pub fn ink_ascent(&self) -> f32 {
        (-self.ink.min.y).max(0.0)
    }

    /// Depth of the ink below the baseline (never negative).
    pub fn ink_descent(&self) -> f32 {
        self.ink.max.y.max(0.0)
    }
}

/// Measures text for layout. Implemented for egui's [`FontsView`]; layout code
/// only sees this trait so it can run headless.
pub trait TextMeasure {
    /// Measures `text` laid out on one line.
    fn measure(&mut self, text: &str, font: &FontId) -> Measured;

    /// Size of `text` wrapped at `wrap_width` (used for code blocks).
    fn measure_wrapped(&mut self, text: &str, font: &FontId, wrap_width: f32) -> Vec2;

    /// Physical pixels per point, so hairlines can be kept at least one pixel thick.
    fn pixels_per_point(&self) -> f32;
}

/// Measures through egui so widths match exactly what [`crate::paint`] draws.
pub struct EguiMeasure<'v, 'f> {
    fonts: &'v mut FontsView<'f>,
    pixels_per_point: f32,
}

impl<'v, 'f> EguiMeasure<'v, 'f> {
    pub fn new(fonts: &'v mut FontsView<'f>, pixels_per_point: f32) -> Self {
        EguiMeasure {
            fonts,
            pixels_per_point,
        }
    }
}

impl TextMeasure for EguiMeasure<'_, '_> {
    fn measure(&mut self, text: &str, font: &FontId) -> Measured {
        let galley = self.fonts.layout(
            text.to_owned(),
            font.clone(),
            Color32::PLACEHOLDER,
            f32::INFINITY,
        );
        let Some(first_row) = galley.rows.first() else {
            return Measured {
                width: 0.0,
                ink: Rect::ZERO,
                galley_offset: Vec2::ZERO,
            };
        };
        let baseline = first_row.pos.y
            + first_row
                .row
                .glyphs
                .first()
                .map_or(font.size * 0.8, |g| g.pos.y);
        let ink = first_row
            .row
            .glyphs
            .iter()
            .filter(|g| !g.uv_rect.is_nothing())
            .map(|g| {
                Rect::from_min_size(
                    first_row.pos + g.pos.to_vec2() + g.uv_rect.offset,
                    g.uv_rect.size,
                )
            })
            .reduce(|a, b| a.union(b))
            .map_or(Rect::ZERO, |r| r.translate(vec2(0.0, -baseline)));
        Measured {
            width: galley.size().x,
            ink,
            galley_offset: pos2(0.0, -baseline).to_vec2(),
        }
    }

    fn measure_wrapped(&mut self, text: &str, font: &FontId, wrap_width: f32) -> Vec2 {
        self.fonts
            .layout(
                text.to_owned(),
                font.clone(),
                Color32::PLACEHOLDER,
                wrap_width,
            )
            .size()
    }

    fn pixels_per_point(&self) -> f32 {
        self.pixels_per_point
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    //! A headless egui context with the renderer's fonts installed.

    use super::*;

    /// Runs `f` with egui's fonts (renderer fonts installed), no window needed.
    pub fn with_fonts<R>(f: impl FnOnce(&mut FontsView<'_>, f32) -> R) -> R {
        let ctx = egui::Context::default();
        let mut fonts = egui::FontDefinitions::default();
        crate::install_fonts(&mut fonts);
        ctx.set_fonts(fonts);
        // The first pass applies the new font definitions.
        let _ = ctx.run_ui(egui::RawInput::default(), |_| {});
        let mut f = Some(f);
        let mut result = None;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let ppp = ui.ctx().pixels_per_point();
            if let Some(f) = f.take() {
                result = Some(ui.ctx().fonts_mut(|fonts| f(fonts, ppp)));
            }
        });
        result.expect("the test closure ran")
    }

    /// Runs `f` with a measurer backed by real egui fonts.
    pub fn with_measure<R>(f: impl FnOnce(&mut dyn TextMeasure) -> R) -> R {
        with_fonts(|fonts, ppp| f(&mut EguiMeasure::new(fonts, ppp)))
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::with_measure;
    use crate::fonts;
    use egui::FontId;

    #[test]
    fn letters_have_ink_above_baseline_and_descenders_below() {
        with_measure(|m| {
            let font = FontId::new(20.0, fonts::family(fonts::MATH));
            let x = m.measure("x", &font);
            let p = m.measure("p", &font);
            assert!(x.width > 5.0 && x.width < 15.0, "x width {}", x.width);
            assert!(
                x.ink_ascent() > 6.0 && x.ink_ascent() < 11.0,
                "x height {}",
                x.ink_ascent()
            );
            assert!(x.ink_descent() < 1.5);
            assert!(p.ink_descent() > 3.0);
        });
    }

    #[test]
    fn blank_text_has_width_but_no_ink() {
        with_measure(|m| {
            let space = m.measure(" ", &FontId::new(20.0, fonts::family(fonts::SERIF)));
            assert!(space.width > 2.0);
            assert_eq!(space.ink_ascent(), 0.0);
            assert_eq!(
                m.measure("", &FontId::new(20.0, fonts::family(fonts::SERIF)))
                    .width,
                0.0
            );
        });
    }

    /// egui's `has_glyph` reports false negatives for the primary face, so a
    /// glyph counts as present when it differs from the replacement glyph
    /// drawn for an unassigned private-use code point.
    fn assert_glyphs_present(m: &mut dyn super::TextMeasure, family: &str, text: &str) {
        let font = FontId::new(20.0, fonts::family(family));
        let missing = m.measure("\u{E000}", &font);
        for c in text.chars() {
            let measured = m.measure(&c.to_string(), &font);
            assert!(
                measured.ink_ascent() > 0.0 || measured.ink_descent() > 0.0,
                "{c} has no ink in {family}"
            );
            assert_ne!(
                measured, missing,
                "{c} ({:X}) is missing from {family}",
                c as u32
            );
        }
    }

    #[test]
    fn math_font_covers_math_alphanumerics_and_symbols() {
        with_measure(|m| {
            let italic: String = ('a'..='z')
                .chain('A'..='Z')
                .map(crate::math::alphabet::italic)
                .collect();
            let bold: String = ('A'..='Z')
                .chain('a'..='z')
                .chain('0'..='9')
                .map(crate::math::alphabet::bold)
                .collect();
            let greek: String = ('α'..='ω').map(crate::math::alphabet::italic).collect();
            for text in [
                italic.as_str(),
                bold.as_str(),
                greek.as_str(),
                "∑∫∏√∞≤≥≠≈∈⊂∪∩∀∃∂∇→⇒↦×·±ℝℕℤℚℂ⟨⟩‖∎…⋯•–∗",
            ] {
                assert_glyphs_present(m, fonts::MATH, text);
            }
            assert_glyphs_present(m, fonts::SERIF, "Hello“world”—façade");
            assert_glyphs_present(m, fonts::MONO, "fn_main(){}");
        });
    }
}
