//! Fractions, scripts, limits, radicals and accents.

use egui::{pos2, Rect};
use qm_format::{AccentKind, AtomClass, AtomStyle, MathNode};

use super::list::{is_operator_symbol, wavy_line};
use super::style::constants::*;
use super::{MathBox, MathLayout, MathStyle};
use crate::paint::{Primitive, Role};

/// A radical sign is never shorter than the `√` glyph (1 em).
const MIN_RADICAL_HEIGHT: f32 = 1.0;
/// Bases no taller than a parenthesis glyph get character-style scripts.
const GLYPH_SIZED_ASCENT: f32 = 0.82;
const GLYPH_SIZED_DESCENT: f32 = 0.32;
/// Integral subscripts tuck under the slanted glyph (TeX's italic correction).
const INTEGRAL_SUBSCRIPT_TUCK: f32 = 0.25;

impl MathLayout<'_> {
    pub(super) fn fraction(
        &mut self,
        numerator: &[MathNode],
        denominator: &[MathNode],
        style: MathStyle,
    ) -> MathBox {
        let size = self.size(style);
        let num = self.list(numerator, style.numerator());
        let den = self.list(denominator, style.denominator());
        let rule = self.rule(FRACTION_RULE, size);
        let axis = AXIS_HEIGHT * size;
        let (shift_up, shift_down, gap) = if style.is_display() {
            (
                NUMERATOR_SHIFT_UP_DISPLAY,
                DENOMINATOR_SHIFT_DOWN_DISPLAY,
                FRACTION_GAP_MIN_DISPLAY,
            )
        } else {
            (NUMERATOR_SHIFT_UP, DENOMINATOR_SHIFT_DOWN, FRACTION_GAP_MIN)
        };
        let shift_up = (shift_up * size).max(axis + rule / 2.0 + gap * size + num.descent);
        let shift_down = (shift_down * size).max(gap * size + den.ascent - axis + rule / 2.0);

        let padding = NULL_DELIMITER_SPACE * size;
        let inner_width = num.width.max(den.width);
        let mut fraction = MathBox {
            width: inner_width + 2.0 * padding,
            ..MathBox::default()
        };
        fraction.place(&num, padding + (inner_width - num.width) / 2.0, -shift_up);
        fraction.place(&den, padding + (inner_width - den.width) / 2.0, shift_down);
        let rule_rect = Rect::from_min_max(
            pos2(padding, -axis - rule / 2.0),
            pos2(padding + inner_width, -axis + rule / 2.0),
        );
        fraction.drawing.push(Primitive::Rect {
            rect: rule_rect,
            role: Role::Text,
            radius: 0.0,
        });
        fraction.ascent = fraction.ascent.max(axis + rule / 2.0);
        fraction
    }

    pub(super) fn scripts(
        &mut self,
        base: &MathNode,
        sub: Option<&[MathNode]>,
        sup: Option<&[MathNode]>,
        style: MathStyle,
    ) -> MathBox {
        let base_box = self.node(base, style);
        let sup_box = sup.map(|s| self.list(s, style.superscript()));
        let sub_box = sub.map(|s| self.list(s, style.subscript()));
        let operator = match base {
            MathNode::Atom(atom) => Some(atom),
            _ => None,
        };
        let has_limits =
            operator.is_some_and(|a| a.class == AtomClass::LargeOperator { limits: true });
        if has_limits && style.is_display() {
            return self.limits(base_box, sub_box, sup_box, style);
        }
        let size = self.size(style);
        let is_glyph_sized = base_box.ascent <= GLYPH_SIZED_ASCENT * size
            && base_box.descent <= GLYPH_SIZED_DESCENT * size;
        let shape = match operator {
            Some(atom) if is_operator_symbol(atom) => BaseShape::Operator {
                is_integral: atom.text.contains('∫') || atom.text.contains('∮'),
            },
            Some(_) => BaseShape::Glyph,
            // `(a+b)^2` places its exponent like a plain `)` would.
            None if is_glyph_sized => BaseShape::Glyph,
            None => BaseShape::Compound,
        };
        self.attach_scripts(base_box, sub_box, sup_box, style, shape)
    }

    /// Scripts to the right of the base (OpenType MATH rules).
    fn attach_scripts(
        &mut self,
        base: MathBox,
        sub: Option<MathBox>,
        sup: Option<MathBox>,
        style: MathStyle,
        shape: BaseShape,
    ) -> MathBox {
        let size = self.size(style);
        let script_size = self.size(style.superscript());
        // TeX does not drop scripts relative to a single character's box.
        let (sup_drop, sub_drop) = match shape {
            BaseShape::Glyph => (0.0, 0.0),
            _ => (
                base.ascent - SUPERSCRIPT_BASELINE_DROP_MAX * script_size,
                base.descent + SUBSCRIPT_BASELINE_DROP_MIN * script_size,
            ),
        };
        let sup_shift_min = if style.cramped {
            SUPERSCRIPT_SHIFT_UP_CRAMPED
        } else {
            SUPERSCRIPT_SHIFT_UP
        } * size;
        let mut sup_shift = sup.as_ref().map_or(0.0, |s| {
            sup_shift_min
                .max(sup_drop)
                .max(s.descent + SUPERSCRIPT_BOTTOM_MIN * size)
        });
        let mut sub_shift = match (&sub, &sup) {
            (Some(s), None) => (SUBSCRIPT_SHIFT_DOWN * size)
                .max(sub_drop)
                .max(s.ascent - SUBSCRIPT_TOP_MAX * size),
            (Some(_), Some(_)) => (SUBSCRIPT_SHIFT_DOWN * size).max(sub_drop),
            _ => 0.0,
        };
        if let (Some(sub), Some(sup)) = (&sub, &sup) {
            let gap = (sup_shift - sup.descent) - (sub.ascent - sub_shift);
            let needed = SUB_SUPERSCRIPT_GAP_MIN * size - gap;
            if needed > 0.0 {
                let room_up = (SUPERSCRIPT_BOTTOM_MAX_WITH_SUBSCRIPT * size
                    - (sup_shift - sup.descent))
                    .max(0.0);
                let up = needed.min(room_up);
                sup_shift += up;
                sub_shift += needed - up;
            }
        }

        let (sup_x, sub_x) = match shape {
            BaseShape::Operator { is_integral: true } => {
                (base.width, base.width * (1.0 - INTEGRAL_SUBSCRIPT_TUCK))
            }
            _ => (base.width + base.italic_correction, base.width),
        };
        let mut result = MathBox::default();
        result.place(&base, 0.0, 0.0);
        let mut right = base.width;
        if let Some(sup) = &sup {
            result.place(sup, sup_x, -sup_shift);
            right = right.max(sup_x + sup.width);
        }
        if let Some(sub) = &sub {
            result.place(sub, sub_x, sub_shift);
            right = right.max(sub_x + sub.width);
        }
        let has_scripts = sup.is_some() || sub.is_some();
        result.width = right
            + if has_scripts {
                SPACE_AFTER_SCRIPT * size
            } else {
                0.0
            };
        result
    }

    /// Limits centred above and below a display-style operator.
    fn limits(
        &mut self,
        base: MathBox,
        sub: Option<MathBox>,
        sup: Option<MathBox>,
        style: MathStyle,
    ) -> MathBox {
        let size = self.size(style);
        let width = [Some(&base), sub.as_ref(), sup.as_ref()]
            .into_iter()
            .flatten()
            .map(|b| b.width)
            .fold(0.0, f32::max);
        let mut result = MathBox {
            width,
            ..MathBox::default()
        };
        result.place(&base, (width - base.width) / 2.0, 0.0);
        if let Some(sup) = &sup {
            let shift = base.ascent
                + (UPPER_LIMIT_GAP_MIN * size + sup.descent)
                    .max(UPPER_LIMIT_BASELINE_RISE_MIN * size);
            result.place(sup, (width - sup.width) / 2.0, -shift);
        }
        if let Some(sub) = &sub {
            let shift = base.descent
                + (LOWER_LIMIT_GAP_MIN * size + sub.ascent)
                    .max(LOWER_LIMIT_BASELINE_DROP_MIN * size);
            result.place(sub, (width - sub.width) / 2.0, shift);
        }
        result
    }

    pub(super) fn root(
        &mut self,
        index: Option<&[MathNode]>,
        radicand: &[MathNode],
        style: MathStyle,
    ) -> MathBox {
        let size = self.size(style);
        let body = self.list(radicand, style.cramped());
        let gap = if style.is_display() {
            RADICAL_VERTICAL_GAP_DISPLAY
        } else {
            RADICAL_VERTICAL_GAP
        } * size;
        let rule = self.rule(RADICAL_RULE, size);
        let top = body.ascent + gap + rule;
        let bottom = (-body.descent).min(top - MIN_RADICAL_HEIGHT * size);
        let sign_height = top - bottom;

        let degree = index
            .filter(|i| !i.is_empty())
            .map(|i| self.list(i, MathStyle::SCRIPT_SCRIPT));
        let kern_before = RADICAL_KERN_BEFORE_DEGREE * size;
        let sign_x = degree.as_ref().map_or(0.0, |d| {
            (kern_before + d.width + RADICAL_KERN_AFTER_DEGREE * size).max(0.0)
        });
        let sign = radical_sign(sign_height / size, size, rule);
        let body_x = sign_x + sign.width;
        let overbar_end = body_x + body.width + 0.05 * size;

        let mut result = MathBox {
            width: overbar_end,
            ..MathBox::default()
        };
        result.place(&sign, sign_x, -bottom);
        result.place(&body, body_x, 0.0);
        let overbar = Rect::from_min_max(
            pos2(body_x - rule * 0.3, -top),
            pos2(overbar_end, -top + rule),
        );
        result.drawing.push(Primitive::Rect {
            rect: overbar,
            role: Role::Text,
            radius: 0.0,
        });
        if let Some(degree) = &degree {
            let raise = bottom + RADICAL_DEGREE_BOTTOM_RAISE * sign_height + degree.descent;
            result.place(degree, kern_before, -raise);
        }
        result.ascent = result.ascent.max(top + RADICAL_EXTRA_ASCENDER * size);
        result.descent = result.descent.max(-bottom);
        result
    }

    pub(super) fn accent(
        &mut self,
        kind: AccentKind,
        body: &[MathNode],
        style: MathStyle,
    ) -> MathBox {
        let size = self.size(style);
        let base = self.list(body, style.cramped());
        let is_single_letter = matches!(body, [MathNode::Atom(a)] if a.text.chars().count() == 1);
        let skew = if matches!(body, [MathNode::Atom(a)] if a.style == AtomStyle::Italic) {
            0.06 * size
        } else {
            0.0
        };
        let centre = base.width / 2.0 + skew;
        let bottom = base.ascent.max(ACCENT_BASE_HEIGHT * size) + 0.06 * size;
        let stroke = self.rule(0.04, size);
        let natural = |minimum: f32| {
            if is_single_letter {
                minimum * size
            } else {
                (base.width * 0.9).max(minimum * size)
            }
        };

        let mut result = MathBox::default();
        result.place(&base, 0.0, 0.0);
        result.width = base.width;
        let top = match kind {
            AccentKind::Hat => {
                let width = natural(0.30);
                let height = ((0.10 + 0.04 * width / size).min(0.22)) * size;
                let points = vec![
                    pos2(centre - width / 2.0, -bottom),
                    pos2(centre, -bottom - height),
                    pos2(centre + width / 2.0, -bottom),
                ];
                result.drawing.push(Primitive::Polyline {
                    points,
                    width: stroke,
                    role: Role::Text,
                });
                bottom + height
            }
            AccentKind::Bar => {
                let width = natural(0.32);
                let rect = Rect::from_min_max(
                    pos2(centre - width / 2.0, -bottom - stroke),
                    pos2(centre + width / 2.0, -bottom),
                );
                result.drawing.push(Primitive::Rect {
                    rect,
                    role: Role::Text,
                    radius: 0.0,
                });
                bottom + stroke
            }
            AccentKind::Vector => {
                let width = if is_single_letter {
                    0.42 * size
                } else {
                    base.width.max(0.42 * size)
                };
                let y = -bottom - 0.07 * size;
                let (start, tip) = (pos2(centre - width / 2.0, y), pos2(centre + width / 2.0, y));
                let head = vec![
                    pos2(tip.x - 0.11 * size, y - 0.07 * size),
                    tip,
                    pos2(tip.x - 0.11 * size, y + 0.07 * size),
                ];
                result.drawing.push(Primitive::Polyline {
                    points: vec![start, tip],
                    width: stroke,
                    role: Role::Text,
                });
                result.drawing.push(Primitive::Polyline {
                    points: head,
                    width: stroke,
                    role: Role::Text,
                });
                bottom + 0.14 * size
            }
            AccentKind::Dot | AccentKind::DoubleDot => {
                let radius = 0.05 * size;
                let offsets: &[f32] = if kind == AccentKind::Dot {
                    &[0.0]
                } else {
                    &[-0.11, 0.11]
                };
                for offset in offsets {
                    let center = pos2(centre + offset * size, -bottom - radius);
                    result.drawing.push(Primitive::Circle {
                        center,
                        radius,
                        role: Role::Text,
                    });
                }
                bottom + 2.0 * radius
            }
            AccentKind::Tilde => {
                let width = natural(0.34);
                let amplitude = 0.035 * size;
                let y = -bottom - amplitude - stroke;
                let points = wavy_line(
                    centre - width / 2.0,
                    centre + width / 2.0,
                    y,
                    -amplitude,
                    width,
                );
                result.drawing.push(Primitive::Polyline {
                    points,
                    width: stroke * 1.1,
                    role: Role::Text,
                });
                bottom + 2.0 * amplitude + 2.0 * stroke
            }
        };
        result.ascent = result.ascent.max(top + stroke / 2.0);
        result
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BaseShape {
    /// A single letter or symbol: scripts don't drop with its height.
    Glyph,
    Operator {
        is_integral: bool,
    },
    Compound,
}

/// The `√` sign as variable-width strokes, `height_em` tall, with its top at
/// the box's ascent and its bottom on the baseline.
fn radical_sign(height_em: f32, size: f32, rule: f32) -> MathBox {
    let height = height_em * size;
    let width = (0.56 + 0.06 * (height_em - 1.0).clamp(0.0, 4.0)) * size;
    let tick = height.min(1.3 * size);
    let up = |x: f32, y: f32| pos2(x, -y);
    let a = up(0.0, 0.42 * tick);
    let b = up(0.16 * width, 0.50 * tick);
    let c = up(0.42 * width, 0.0);
    let d = up(width, height - rule / 2.0);
    let thin = rule * 0.55;
    let mut sign = MathBox {
        width,
        ascent: height,
        descent: 0.0,
        ..MathBox::default()
    };
    for (points, half_widths) in [
        (vec![a, b], vec![thin * 0.8, thin]),
        (vec![b, c], vec![0.030 * size, 0.042 * size]),
        (vec![c, d], vec![thin, thin]),
    ] {
        sign.drawing.push(Primitive::Ribbon {
            points,
            half_widths,
            role: Role::Text,
        });
    }
    sign
}
