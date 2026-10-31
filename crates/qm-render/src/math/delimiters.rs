//! Stretchy delimiters, fenced expressions and matrices.
//!
//! Delimiters up to the font's own glyph size use the glyph; taller ones are
//! drawn as variable-width paths so they stay crisp and thin at any height.

use egui::{pos2, Pos2, Rect};
use qm_format::{MathList, MathNode};

use super::style::constants::{
    AXIS_HEIGHT, DELIMITER_FACTOR, DELIMITER_SHORTFALL, NULL_DELIMITER_SPACE,
};
use super::{MathBox, MathLayout, MathStyle};
use crate::fonts;
use crate::paint::{Primitive, Role};

/// Glyphs are used while they are at least this fraction of the needed height.
const GLYPH_TOLERANCE: f32 = 1.02;
/// Samples along each curved delimiter.
const CURVE_SAMPLES: usize = 40;

/// Array struts (TeX: 0.7 and 0.3 of a 1.2em baseline skip).
const STRUT_ASCENT: f32 = 0.84;
const STRUT_DESCENT: f32 = 0.36;
/// amsmath's `cases` uses `\arraystretch` 1.2.
const CASES_STRETCH: f32 = 1.2;
const MATRIX_COLUMN_GAP: f32 = 1.0;
/// Space kept between rows whose cells outgrow the strut (TeX's
/// `\lineskip` idea; plain TeX arrays let such rows touch).
const MATRIX_ROW_GAP: f32 = 0.2;
/// Breathing room between a matrix and its delimiters.
const MATRIX_DELIMITER_PADDING: f32 = 0.08;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Paren(Side),
    Bracket(Side),
    Floor(Side),
    Ceiling(Side),
    Brace(Side),
    Angle(Side),
    Bar,
    DoubleBar,
}

fn shape_of(symbol: &str) -> Option<Shape> {
    use Side::*;
    Some(match symbol {
        "(" => Shape::Paren(Left),
        ")" => Shape::Paren(Right),
        "[" => Shape::Bracket(Left),
        "]" => Shape::Bracket(Right),
        "⌊" => Shape::Floor(Left),
        "⌋" => Shape::Floor(Right),
        "⌈" => Shape::Ceiling(Left),
        "⌉" => Shape::Ceiling(Right),
        "{" => Shape::Brace(Left),
        "}" => Shape::Brace(Right),
        "⟨" | "〈" | "<" => Shape::Angle(Left),
        "⟩" | "〉" | ">" => Shape::Angle(Right),
        "|" | "∣" => Shape::Bar,
        "‖" | "||" | "∥" => Shape::DoubleBar,
        _ => return None,
    })
}

impl MathLayout<'_> {
    pub(super) fn fenced(
        &mut self,
        open: &str,
        close: &str,
        body: &[MathNode],
        style: MathStyle,
    ) -> MathBox {
        let inner = self.list(body, style);
        self.wrap_in_delimiters(open, close, &inner, style)
    }

    fn wrap_in_delimiters(
        &mut self,
        open: &str,
        close: &str,
        inner: &MathBox,
        style: MathStyle,
    ) -> MathBox {
        let size = self.size(style);
        let axis = AXIS_HEIGHT * size;
        let half = (inner.ascent - axis).max(inner.descent + axis);
        let required = (2.0 * half * DELIMITER_FACTOR).max(2.0 * half - DELIMITER_SHORTFALL * size);
        let left = self.delimiter(open, required, style);
        let right = self.delimiter(close, required, style);
        let mut result = MathBox::default();
        result.place(&left, 0.0, 0.0);
        result.place(inner, left.width, 0.0);
        result.place(&right, left.width + inner.width, 0.0);
        result.width = left.width + inner.width + right.width;
        result
    }

    /// A delimiter at least `required` tall, centred on the math axis.
    fn delimiter(&mut self, symbol: &str, required: f32, style: MathStyle) -> MathBox {
        let size = self.size(style);
        if symbol.is_empty() || symbol == "." {
            return MathBox::space(NULL_DELIMITER_SPACE * size);
        }
        let glyph = self.glyph(symbol, self.font(fonts::MATH, size), Role::Text);
        if required <= glyph.height() * GLYPH_TOLERANCE || glyph.height() <= 0.0 {
            return glyph;
        }
        let axis = AXIS_HEIGHT * size;
        let (top, bottom) = (axis + required / 2.0, axis - required / 2.0);
        if let Some(shape) = shape_of(symbol) {
            return draw_shape(shape, top, bottom, size, self.rule(0.04, size));
        }
        // Unknown delimiter: scale the glyph up uniformly.
        let scaled = self.glyph(
            symbol,
            self.font(fonts::MATH, size * required / glyph.height()),
            Role::Text,
        );
        let centre = (scaled.ascent - scaled.descent) / 2.0;
        scaled.raised(axis - centre)
    }

    pub(super) fn matrix(
        &mut self,
        open: &str,
        close: &str,
        rows: &[Vec<MathList>],
        style: MathStyle,
    ) -> MathBox {
        let size = self.size(style);
        let cell_style = if style.is_script() {
            style
        } else {
            MathStyle::TEXT
        };
        let is_cases = open == "{" && close.is_empty();
        let cells: Vec<Vec<MathBox>> = rows
            .iter()
            .map(|row| row.iter().map(|cell| self.list(cell, cell_style)).collect())
            .collect();
        let column_widths = column_widths(&cells);
        let column_gap = MATRIX_COLUMN_GAP * size;
        let content_width = column_widths.iter().sum::<f32>()
            + column_gap * column_widths.len().saturating_sub(1) as f32;
        let stretch = if is_cases { CASES_STRETCH } else { 1.0 };
        let rows = row_layout(
            &cells,
            (
                STRUT_ASCENT * stretch * size,
                STRUT_DESCENT * stretch * size,
            ),
            MATRIX_ROW_GAP * size,
        );
        let total_height = rows
            .last()
            .map_or(0.0, |(baseline, _, descent)| baseline + descent);

        let has_delimiters = !(open.is_empty() && close.is_empty());
        let padding = if has_delimiters && !is_cases {
            MATRIX_DELIMITER_PADDING * size
        } else {
            0.0
        };
        let top = AXIS_HEIGHT * size + total_height / 2.0;
        let mut inner = MathBox {
            width: content_width + 2.0 * padding,
            ascent: top,
            descent: total_height - top,
            ..MathBox::default()
        };
        for (row, (baseline, _, _)) in cells.iter().zip(&rows) {
            let mut x = padding;
            for (cell, width) in row.iter().zip(&column_widths) {
                let offset = if is_cases {
                    0.0
                } else {
                    (width - cell.width) / 2.0
                };
                inner.place(cell, x + offset, baseline - top);
                x += width + column_gap;
            }
        }
        if !has_delimiters {
            return inner;
        }
        self.wrap_in_delimiters(open, close, &inner, style)
    }
}

fn column_widths(cells: &[Vec<MathBox>]) -> Vec<f32> {
    let mut widths = vec![0.0f32; cells.iter().map(Vec::len).max().unwrap_or(0)];
    for row in cells {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = width.max(cell.width);
        }
    }
    widths
}

/// Each row's (baseline from the top, ascent, descent). Rows are at least a
/// strut tall; rows whose cells outgrow the strut keep `overflow_gap` from
/// their neighbours.
fn row_layout(
    cells: &[Vec<MathBox>],
    strut: (f32, f32),
    overflow_gap: f32,
) -> Vec<(f32, f32, f32)> {
    let mut rows = Vec::with_capacity(cells.len());
    let mut bottom = 0.0;
    let mut previous_overflows = false;
    for (index, row) in cells.iter().enumerate() {
        let ascent = row.iter().map(|c| c.ascent).fold(strut.0, f32::max);
        let descent = row.iter().map(|c| c.descent).fold(strut.1, f32::max);
        let overflows = ascent > strut.0 + 0.01 || descent > strut.1 + 0.01;
        let gap = if index > 0 && (overflows || previous_overflows) {
            overflow_gap
        } else {
            0.0
        };
        let baseline = bottom + gap + ascent;
        rows.push((baseline, ascent, descent));
        bottom = baseline + descent;
        previous_overflows = overflows;
    }
    rows
}

/// Draws `shape` spanning `top`..`bottom` (distances above the baseline).
fn draw_shape(shape: Shape, top: f32, bottom: f32, size: f32, rule: f32) -> MathBox {
    let height = top - bottom;
    let height_em = height / size;
    let mut drawing = ShapeDrawing {
        top,
        bottom,
        size,
        rule,
        primitives: Vec::new(),
    };
    let width = match shape {
        Shape::Paren(_) => drawing.paren(height_em),
        Shape::Bracket(_) => drawing.bracket(true, true),
        Shape::Floor(_) => drawing.bracket(false, true),
        Shape::Ceiling(_) => drawing.bracket(true, false),
        Shape::Brace(_) => drawing.brace(height_em),
        Shape::Angle(_) => drawing.angle(height_em),
        Shape::Bar => drawing.bars(1),
        Shape::DoubleBar => drawing.bars(2),
    };
    let is_right = matches!(
        shape,
        Shape::Paren(Side::Right)
            | Shape::Bracket(Side::Right)
            | Shape::Floor(Side::Right)
            | Shape::Ceiling(Side::Right)
            | Shape::Brace(Side::Right)
            | Shape::Angle(Side::Right)
    );
    let mut result = MathBox {
        width,
        ascent: top,
        descent: -bottom,
        ..MathBox::default()
    };
    for primitive in drawing.primitives {
        result.drawing.push(if is_right {
            mirror(primitive, width)
        } else {
            primitive
        });
    }
    result
}

/// Flips a left-hand delimiter primitive into its right-hand counterpart.
fn mirror(primitive: Primitive, width: f32) -> Primitive {
    let flip = |p: Pos2| pos2(width - p.x, p.y);
    match primitive {
        Primitive::Ribbon {
            points,
            half_widths,
            role,
        } => Primitive::Ribbon {
            points: points.into_iter().map(flip).collect(),
            half_widths,
            role,
        },
        Primitive::Polyline {
            points,
            width: w,
            role,
        } => Primitive::Polyline {
            points: points.into_iter().map(flip).collect(),
            width: w,
            role,
        },
        Primitive::Rect { rect, role, radius } => Primitive::Rect {
            rect: Rect::from_two_pos(flip(rect.min), flip(rect.max)),
            role,
            radius,
        },
        other => other,
    }
}

/// Builds left-hand delimiter shapes in box coordinates (y down, baseline 0).
struct ShapeDrawing {
    top: f32,
    bottom: f32,
    size: f32,
    rule: f32,
    primitives: Vec<Primitive>,
}

impl ShapeDrawing {
    fn y(&self, up: f32) -> f32 {
        -up
    }

    fn paren(&mut self, height_em: f32) -> f32 {
        let size = self.size;
        let bulge = (0.13 + 0.075 * height_em).min(0.5) * size;
        let thick = ((0.034 + 0.006 * height_em).min(0.06) * size).max(self.rule / 2.0);
        let thin = (0.012 * size).max(self.rule / 4.0);
        let padding = 0.08 * size;
        // Taller parentheses get straighter middles, like TeX's extensible ones.
        let flatness = 2.0 + (height_em - 1.0).max(0.0) * 0.8;
        let mut points = Vec::with_capacity(CURVE_SAMPLES + 1);
        let mut half_widths = Vec::with_capacity(CURVE_SAMPLES + 1);
        for i in 0..=CURVE_SAMPLES {
            let t = i as f32 / CURVE_SAMPLES as f32;
            let s = (2.0 * t - 1.0).abs();
            let x = padding + thick + bulge * s.powf(flatness);
            let up = self.top - t * (self.top - self.bottom);
            points.push(pos2(x, self.y(up)));
            half_widths.push(thin + (thick - thin) * (1.0 - s.powf(flatness)));
        }
        self.primitives.push(Primitive::Ribbon {
            points,
            half_widths,
            role: Role::Text,
        });
        padding + bulge + 2.0 * thick + padding * 0.6
    }

    fn bracket(&mut self, top_arm: bool, bottom_arm: bool) -> f32 {
        let size = self.size;
        let stem = self.rule.max(0.042 * size);
        let arm = self.rule.max(0.04 * size);
        let padding = 0.12 * size;
        let arm_length = 0.22 * size;
        let (top, bottom) = (self.y(self.top), self.y(self.bottom));
        let mut rect = |min: Pos2, max: Pos2| {
            self.primitives.push(Primitive::Rect {
                rect: Rect::from_min_max(min, max),
                role: Role::Text,
                radius: 0.0,
            })
        };
        rect(pos2(padding, top), pos2(padding + stem, bottom));
        if top_arm {
            rect(pos2(padding, top), pos2(padding + arm_length, top + arm));
        }
        if bottom_arm {
            rect(
                pos2(padding, bottom - arm),
                pos2(padding + arm_length, bottom),
            );
        }
        padding + arm_length + padding * 0.4
    }

    fn brace(&mut self, height_em: f32) -> f32 {
        let size = self.size;
        let depth = (0.24 + 0.04 * height_em).min(0.45) * size;
        let padding = 0.08 * size;
        let (right, stem, tip) = (padding + depth, padding + depth / 2.0, padding);
        let curl = (0.12 * (self.top - self.bottom)).min(0.3 * size);
        let middle = (self.top + self.bottom) / 2.0;
        let thick = (0.04 * size).max(self.rule / 2.0);
        let thin = (0.01 * size).max(self.rule / 4.0);
        for direction in [1.0f32, -1.0] {
            let end_up = if direction > 0.0 {
                self.top
            } else {
                self.bottom
            };
            // From the outer end, curl into the stem, run to the middle, curl out to the tip.
            let end = pos2(right, self.y(end_up));
            let stem_start = pos2(stem, self.y(end_up - direction * curl));
            let stem_end = pos2(stem, self.y(middle + direction * curl));
            let point = pos2(tip, self.y(middle));
            let mut points = Vec::new();
            let mut half_widths = Vec::new();
            let mut push_curve = |from: Pos2, control: Pos2, to: Pos2, w0: f32, w1: f32| {
                for i in 0..=CURVE_SAMPLES / 2 {
                    let t = i as f32 / (CURVE_SAMPLES / 2) as f32;
                    let a = from.lerp(control, t);
                    let b = control.lerp(to, t);
                    points.push(a.lerp(b, t));
                    half_widths.push(w0 + (w1 - w0) * t);
                }
            };
            push_curve(end, pos2(stem, end.y), stem_start, thin, thick);
            push_curve(stem_end, pos2(stem, point.y), point, thick, thin);
            self.primitives.push(Primitive::Ribbon {
                points,
                half_widths,
                role: Role::Text,
            });
        }
        right + padding
    }

    fn angle(&mut self, height_em: f32) -> f32 {
        let size = self.size;
        let depth = (0.18 + 0.05 * height_em).min(0.45) * size;
        let padding = 0.1 * size;
        let middle = (self.top + self.bottom) / 2.0;
        let points = vec![
            pos2(padding + depth, self.y(self.top)),
            pos2(padding, self.y(middle)),
            pos2(padding + depth, self.y(self.bottom)),
        ];
        self.primitives.push(Primitive::Polyline {
            points,
            width: self.rule.max(0.04 * size),
            role: Role::Text,
        });
        padding + depth + padding
    }

    fn bars(&mut self, count: usize) -> f32 {
        let size = self.size;
        let width = self.rule.max(0.04 * size);
        let padding = 0.12 * size;
        let spacing = 0.12 * size;
        for i in 0..count {
            let x = padding + i as f32 * spacing;
            let rect = Rect::from_min_max(
                pos2(x, self.y(self.top)),
                pos2(x + width, self.y(self.bottom)),
            );
            self.primitives.push(Primitive::Rect {
                rect,
                role: Role::Text,
                radius: 0.0,
            });
        }
        2.0 * padding + width + (count - 1) as f32 * spacing
    }
}
