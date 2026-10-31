//! Horizontal lists, atoms and the per-node dispatch.

use egui::{pos2, FontId, Pos2, Rect};
use qm_format::{Atom, AtomClass, MathNode, SpaceWidth};

use super::alphabet::styled_text;
use super::style::{
    constants, resolve_binary_operators, space_between, spacing_class, SpacingClass,
};
use super::{MathBox, MathBreak, MathLayout, MathStyle};
use crate::fonts;
use crate::paint::{Hotspot, HotspotKind, Primitive, Role};

/// How much larger big operators are drawn in display style (matching the
/// size of NewCMMath's display variants).
const DISPLAY_SUM_SCALE: f32 = 1.4;
const DISPLAY_INTEGRAL_SCALE: f32 = 1.8;
const INTEGRALS: &str = "∫∬∭∮∯∰∱∲∳⨌";

/// Errors are shown in monospace slightly smaller than the math around them.
const ERROR_FONT_SCALE: f32 = 0.9;
/// Nesting beyond this is drawn as an error instead of recursing further.
pub const MAX_DEPTH: usize = 64;

#[derive(Debug, Clone, Copy, Default)]
pub struct ListOptions {
    /// Lay out as if an ordinary atom came first (second cell of `x &= 1`).
    pub starts_after_ord: bool,
    /// Record break points after top-level relations and binary operators.
    pub breakable: bool,
}

impl MathLayout<'_> {
    pub fn list(&mut self, list: &[MathNode], style: MathStyle) -> MathBox {
        self.list_with(list, style, ListOptions::default())
    }

    pub fn list_with(
        &mut self,
        list: &[MathNode],
        style: MathStyle,
        options: ListOptions,
    ) -> MathBox {
        if let [single] = list {
            if !options.starts_after_ord {
                return self.node(single, style);
            }
        }
        let boxes: Vec<MathBox> = list.iter().map(|node| self.node(node, style)).collect();
        let mut classes: Vec<Option<SpacingClass>> = list.iter().map(spacing_class).collect();
        resolve_binary_operators(&mut classes, options.starts_after_ord);
        self.concatenate(&boxes, &classes, style, options)
    }

    /// Places boxes left to right with TeX's inter-atom glue between them.
    fn concatenate(
        &self,
        boxes: &[MathBox],
        classes: &[Option<SpacingClass>],
        style: MathStyle,
        options: ListOptions,
    ) -> MathBox {
        let em = self.size(style);
        let mut result = MathBox::default();
        let mut x = 0.0;
        let mut previous = options.starts_after_ord.then_some(SpacingClass::Ord);
        let mut pending_break: Option<f32> = None;
        for (index, (child, class)) in boxes.iter().zip(classes).enumerate() {
            if let Some(class) = *class {
                if let Some(previous) = previous {
                    x += space_between(previous, class, style) * em;
                }
                if let Some(end) = pending_break.take() {
                    if !matches!(
                        class,
                        SpacingClass::Rel | SpacingClass::Close | SpacingClass::Punct
                    ) {
                        result.breaks.push(MathBreak { end, resume: x });
                    }
                }
                previous = Some(class);
            }
            result.place(child, x, 0.0);
            x += child.width;
            let is_last = index + 1 == boxes.len();
            if options.breakable
                && !is_last
                && matches!(class, Some(SpacingClass::Rel | SpacingClass::Bin))
            {
                pending_break = Some(x);
            }
        }
        result.width = x;
        result
    }

    pub fn node(&mut self, node: &MathNode, style: MathStyle) -> MathBox {
        if self.depth >= MAX_DEPTH {
            return self.error("…", "formula is nested too deeply to display", style);
        }
        self.depth += 1;
        let laid_out = self.dispatch(node, style);
        self.depth -= 1;
        laid_out
    }

    fn dispatch(&mut self, node: &MathNode, style: MathStyle) -> MathBox {
        match node {
            MathNode::Atom(atom) => self.atom(atom, style),
            MathNode::Text(text) => self.text(text, style),
            MathNode::Fraction {
                numerator,
                denominator,
            } => self.fraction(numerator, denominator, style),
            MathNode::Scripts { base, sub, sup } => {
                self.scripts(base, sub.as_deref(), sup.as_deref(), style)
            }
            MathNode::Root { index, radicand } => self.root(index.as_deref(), radicand, style),
            MathNode::Fenced { open, close, body } => self.fenced(open, close, body, style),
            MathNode::Matrix { open, close, rows } => self.matrix(open, close, rows, style),
            MathNode::Accent { accent, body } => self.accent(*accent, body, style),
            MathNode::Group(list) => self.list(list, style),
            MathNode::Space(width) => MathBox::space(space_width(*width) * self.size(style)),
            MathNode::Error { source, message } => self.error(source, message, style),
        }
    }

    fn atom(&mut self, atom: &Atom, style: MathStyle) -> MathBox {
        let size = self.size(style);
        let text = styled_text(&atom.text, atom.style);
        if !is_operator_symbol(atom) {
            return self.glyph(&text, self.font(fonts::MATH, size), Role::Text);
        }
        let scale = if style.is_display() {
            display_operator_scale(&text)
        } else {
            1.0
        };
        let operator = self.glyph(&text, self.font(fonts::MATH, size * scale), Role::Text);
        // Big operators are centred on the math axis.
        let centre = (operator.ascent - operator.descent) / 2.0;
        operator.raised(constants::AXIS_HEIGHT * size - centre)
    }

    fn text(&mut self, text: &str, style: MathStyle) -> MathBox {
        let size = self.size(style);
        self.glyph(text, self.font(fonts::SERIF, size), Role::Text)
    }

    /// A box around one run of text, sized by its ink.
    pub(super) fn glyph(&mut self, text: &str, font: FontId, role: Role) -> MathBox {
        if text.is_empty() {
            return MathBox::default();
        }
        let measured = self.measure.measure(text, &font);
        let mut glyph = MathBox {
            width: measured.width,
            ascent: measured.ink_ascent(),
            descent: measured.ink_descent(),
            italic_correction: (measured.ink.max.x - measured.width).max(0.0),
            ..MathBox::default()
        };
        glyph.drawing.push(Primitive::Text {
            pos: Pos2::ZERO + measured.galley_offset,
            text: text.to_owned(),
            font,
            role,
            wrap_width: f32::INFINITY,
        });
        glyph
    }

    /// Unparseable source in the error colour with a wavy underline and a tooltip.
    pub(super) fn error(&mut self, source: &str, message: &str, style: MathStyle) -> MathBox {
        let size = self.size(style);
        let shown = if source.trim().is_empty() {
            "?"
        } else {
            source
        };
        let mut error = self.glyph(
            shown,
            self.font(fonts::MONO, size * ERROR_FONT_SCALE),
            Role::Error,
        );
        let amplitude = 0.035 * size;
        let line_y = error.descent.max(0.08 * size) + 0.1 * size;
        let points = wavy_line(0.0, error.width, line_y, amplitude, 0.22 * size);
        error.drawing.push(Primitive::Polyline {
            points,
            width: self.rule(0.05, size),
            role: Role::Error,
        });
        error.descent = line_y + amplitude * 2.0;
        error.ascent = error.ascent.max(0.6 * size);
        error.drawing.hotspots.push(Hotspot {
            rect: Rect::from_min_max(pos2(0.0, -error.ascent), pos2(error.width, error.descent)),
            kind: HotspotKind::Tooltip(message.to_owned()),
        });
        error
    }
}

/// A single-symbol large operator (`∑`, `∫`), as opposed to `lim` or `max`.
pub fn is_operator_symbol(atom: &Atom) -> bool {
    matches!(atom.class, AtomClass::LargeOperator { .. })
        && atom.text.chars().count() == 1
        && !atom.text.chars().all(char::is_alphanumeric)
}

fn display_operator_scale(text: &str) -> f32 {
    if text.chars().all(|c| INTEGRALS.contains(c)) {
        DISPLAY_INTEGRAL_SCALE
    } else {
        DISPLAY_SUM_SCALE
    }
}

fn space_width(width: SpaceWidth) -> f32 {
    match width {
        SpaceWidth::Thin => 3.0 / 18.0,
        SpaceWidth::Medium => 4.0 / 18.0,
        SpaceWidth::Thick => 5.0 / 18.0,
        SpaceWidth::Quad => 1.0,
    }
}

/// A zig-zag-free sine wave from `x0` to `x1` around `y`.
pub fn wavy_line(x0: f32, x1: f32, y: f32, amplitude: f32, wavelength: f32) -> Vec<Pos2> {
    let steps = (((x1 - x0) / wavelength) * 8.0).ceil().clamp(2.0, 2000.0) as usize;
    (0..=steps)
        .map(|i| {
            let x = x0 + (x1 - x0) * i as f32 / steps as f32;
            let phase = (x - x0) / wavelength * std::f32::consts::TAU;
            pos2(x, y + amplitude * phase.sin())
        })
        .collect()
}
