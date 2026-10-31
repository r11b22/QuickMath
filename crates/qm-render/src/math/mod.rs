//! A TeX-inspired math typesetter: [`MathNode`]s become [`MathBox`]es
//! (width / ascent / descent plus a drawing), which are then painted.
//!
//! All coordinates are relative to the left end of the box's baseline with y
//! growing downwards, so "up" distances such as ascent are subtracted.

pub mod alphabet;
mod constructs;
mod delimiters;
mod display;
mod list;
pub mod style;

use egui::{vec2, FontId, Vec2};
use qm_format::MathNode;

use crate::measure::TextMeasure;
use crate::paint::Drawing;
pub use style::MathStyle;

/// A laid-out piece of math.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MathBox {
    pub width: f32,
    /// Extent above the baseline.
    pub ascent: f32,
    /// Extent below the baseline.
    pub descent: f32,
    pub drawing: Drawing,
    /// How far the ink of a slanted glyph overhangs its advance; superscripts
    /// are moved right by this much (TeX's italic correction).
    pub italic_correction: f32,
    /// Where an inline formula may be broken across lines (after top-level
    /// relations and binary operators).
    pub breaks: Vec<MathBreak>,
}

/// A permitted line break inside a formula.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MathBreak {
    /// x where the material before the break ends.
    pub end: f32,
    /// x where the material after the break starts (the glue in between is
    /// dropped when the break is taken).
    pub resume: f32,
}

impl MathBox {
    /// A box with no ink and the given width.
    pub fn space(width: f32) -> MathBox {
        MathBox {
            width,
            ..MathBox::default()
        }
    }

    pub fn height(&self) -> f32 {
        self.ascent + self.descent
    }

    /// Draws `child` with its baseline origin at `(x, y)` (y downwards) and
    /// grows this box's vertical extent to include it. Width is not changed.
    pub fn place(&mut self, child: &MathBox, x: f32, y: f32) {
        self.drawing.append(&child.drawing, vec2(x, y));
        self.ascent = self.ascent.max(child.ascent - y);
        self.descent = self.descent.max(child.descent + y);
    }

    /// Moves the content up by `shift` (down when negative).
    pub fn raised(self, shift: f32) -> MathBox {
        let mut raised = MathBox {
            width: self.width,
            italic_correction: self.italic_correction,
            ..MathBox::default()
        };
        raised.drawing.append(&self.drawing, Vec2::new(0.0, -shift));
        raised.ascent = self.ascent + shift;
        raised.descent = self.descent - shift;
        raised.breaks = self.breaks;
        raised
    }

    /// Splits the box at its break points into pieces, each paired with the
    /// glue that follows it (zero for the last piece).
    pub fn split_at_breaks(&self) -> Vec<(MathBox, f32)> {
        let mut pieces = Vec::with_capacity(self.breaks.len() + 1);
        let mut start = 0.0;
        for brk in &self.breaks {
            pieces.push((self.piece(start, brk.end), brk.resume - brk.end));
            start = brk.resume;
        }
        pieces.push((self.piece(start, self.width.max(start)), 0.0));
        pieces
    }

    fn piece(&self, start: f32, end: f32) -> MathBox {
        let lower = if start <= 0.0 {
            f32::NEG_INFINITY
        } else {
            start
        };
        let upper = if end >= self.width {
            f32::INFINITY
        } else {
            end
        };
        MathBox {
            width: end - start,
            ascent: self.ascent,
            descent: self.descent,
            drawing: self.drawing.slice_x(lower..upper, start),
            italic_correction: 0.0,
            breaks: Vec::new(),
        }
    }
}

/// Lays out math at a given base (text) font size.
pub struct MathLayout<'m> {
    measure: &'m mut dyn TextMeasure,
    base_size: f32,
    /// Current nesting depth, to cut off pathological input.
    depth: usize,
}

impl<'m> MathLayout<'m> {
    pub fn new(measure: &'m mut dyn TextMeasure, base_size: f32) -> Self {
        MathLayout {
            measure,
            base_size,
            depth: 0,
        }
    }

    /// Inline math (`$…$`): text style, with break points for line wrapping.
    pub fn inline(&mut self, math: &[MathNode]) -> MathBox {
        self.list_with(
            math,
            MathStyle::TEXT,
            list::ListOptions {
                breakable: true,
                ..Default::default()
            },
        )
    }

    /// A single expression in display style.
    pub fn display_expression(&mut self, math: &[MathNode]) -> MathBox {
        self.list(math, MathStyle::DISPLAY)
    }

    pub fn display(&mut self, math: &qm_format::DisplayMath) -> MathBox {
        display::layout_display(self, math)
    }

    /// Font size of `style`.
    fn size(&self, style: MathStyle) -> f32 {
        self.base_size * style.scale()
    }

    /// A rule thickness of `em` at `size`, but never thinner than one pixel.
    fn rule(&self, em: f32, size: f32) -> f32 {
        (em * size).max(1.0 / self.measure.pixels_per_point())
    }

    fn font(&self, family: &str, size: f32) -> FontId {
        FontId::new(size, crate::fonts::family(family))
    }
}

#[cfg(test)]
mod tests;
