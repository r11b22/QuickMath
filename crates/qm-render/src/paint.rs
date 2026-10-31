//! Theme-independent drawing lists produced by layout and painted each frame.
//!
//! Layout stores *roles* instead of colours so cached layouts survive theme
//! switches, and stores text as strings so painting always fetches fresh
//! galleys from egui's own cache (cached galleys would go stale when egui
//! rebuilds its font atlas).

use egui::{Color32, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke, Vec2};
use qm_format::CalloutKind;

use crate::theme::Theme;

/// What a primitive is coloured as; resolved against the [`Theme`] when painting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Text,
    Muted,
    Faint,
    Link,
    AccentSoft,
    Error,
    Border,
    SurfaceRaised,
    CalloutFill(CalloutKind),
    CalloutBar(CalloutKind),
}

impl Role {
    pub fn color(self, theme: &Theme) -> Color32 {
        match self {
            Role::Text => theme.text,
            Role::Muted => theme.text_muted,
            Role::Faint => theme.text_faint,
            Role::Link => theme.accent_text,
            Role::AccentSoft => theme.accent_soft,
            Role::Error => theme.error,
            Role::Border => theme.border,
            Role::SurfaceRaised => theme.surface_raised,
            Role::CalloutFill(kind) => callout_fill(kind, theme),
            Role::CalloutBar(kind) => callout_hue(kind, theme),
        }
    }
}

/// The colour each callout kind is tinted with, taken from the theme.
fn callout_hue(kind: CalloutKind, theme: &Theme) -> Color32 {
    match kind {
        CalloutKind::Definition => theme.accent,
        CalloutKind::Theorem | CalloutKind::Lemma | CalloutKind::Corollary => theme.syntax.callout,
        CalloutKind::Proof => theme.text_faint,
        CalloutKind::Example => theme.syntax.math_keyword,
        CalloutKind::Note => theme.syntax.math_operator,
        CalloutKind::Warning => theme.syntax.math_number,
    }
}

fn callout_fill(kind: CalloutKind, theme: &Theme) -> Color32 {
    if kind == CalloutKind::Definition {
        return theme.accent_soft;
    }
    let strength = if theme.is_dark { 0.14 } else { 0.08 };
    theme
        .background
        .lerp_to_gamma(callout_hue(kind, theme), strength)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    /// A single-line (or `wrap_width`-wrapped) run of text; `pos` is the galley's top-left.
    Text {
        pos: Pos2,
        text: String,
        font: FontId,
        role: Role,
        wrap_width: f32,
    },
    Rect {
        rect: Rect,
        role: Role,
        radius: f32,
    },
    /// A stroke whose width varies along its centre line (delimiters, radicals).
    Ribbon {
        points: Vec<Pos2>,
        half_widths: Vec<f32>,
        role: Role,
    },
    Polyline {
        points: Vec<Pos2>,
        width: f32,
        role: Role,
    },
    Circle {
        center: Pos2,
        radius: f32,
        role: Role,
    },
}

impl Primitive {
    fn translated(&self, offset: Vec2) -> Primitive {
        let mut moved = self.clone();
        match &mut moved {
            Primitive::Text { pos, .. } => *pos += offset,
            Primitive::Rect { rect, .. } => *rect = rect.translate(offset),
            Primitive::Ribbon { points, .. } | Primitive::Polyline { points, .. } => {
                points.iter_mut().for_each(|p| *p += offset);
            }
            Primitive::Circle { center, .. } => *center += offset,
        }
        moved
    }

    /// Horizontal anchor used when a drawing is split into pieces.
    fn left(&self) -> f32 {
        match self {
            Primitive::Text { pos, .. } => pos.x,
            Primitive::Rect { rect, .. } => rect.left(),
            Primitive::Ribbon { points, .. } | Primitive::Polyline { points, .. } => {
                points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min)
            }
            Primitive::Circle { center, .. } => center.x,
        }
    }
}

/// An interactive area: an error tooltip or a clickable wikilink.
#[derive(Debug, Clone, PartialEq)]
pub struct Hotspot {
    pub rect: Rect,
    pub kind: HotspotKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HotspotKind {
    Tooltip(String),
    WikiLink(String),
}

/// Primitives and hotspots in some local coordinate system.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Drawing {
    pub primitives: Vec<Primitive>,
    pub hotspots: Vec<Hotspot>,
}

impl Drawing {
    pub fn push(&mut self, primitive: Primitive) {
        self.primitives.push(primitive);
    }

    /// Appends `other` shifted by `offset`.
    pub fn append(&mut self, other: &Drawing, offset: Vec2) {
        self.primitives
            .extend(other.primitives.iter().map(|p| p.translated(offset)));
        self.hotspots.extend(other.hotspots.iter().map(|h| Hotspot {
            rect: h.rect.translate(offset),
            kind: h.kind.clone(),
        }));
    }

    /// The part of this drawing whose primitives start within `x_range`,
    /// shifted left by `shift`.
    pub fn slice_x(&self, x_range: std::ops::Range<f32>, shift: f32) -> Drawing {
        let offset = Vec2::new(-shift, 0.0);
        let contains = |x: f32| x >= x_range.start - 1e-3 && x < x_range.end - 1e-3;
        Drawing {
            primitives: self
                .primitives
                .iter()
                .filter(|p| contains(p.left()))
                .map(|p| p.translated(offset))
                .collect(),
            hotspots: self
                .hotspots
                .iter()
                .filter(|h| contains(h.rect.left()))
                .map(|h| Hotspot {
                    rect: h.rect.translate(offset),
                    kind: h.kind.clone(),
                })
                .collect(),
        }
    }

    pub fn hotspot_at(&self, pos: Pos2) -> Option<&Hotspot> {
        self.hotspots.iter().rev().find(|h| h.rect.contains(pos))
    }

    /// Paints everything shifted by `offset`.
    pub fn paint(&self, painter: &Painter, offset: Vec2, theme: &Theme) {
        for primitive in &self.primitives {
            paint_primitive(painter, &primitive.translated(offset), theme);
        }
    }
}

fn paint_primitive(painter: &Painter, primitive: &Primitive, theme: &Theme) {
    match primitive {
        Primitive::Text {
            pos,
            text,
            font,
            role,
            wrap_width,
        } => {
            let galley = painter.layout(
                text.clone(),
                font.clone(),
                Color32::PLACEHOLDER,
                *wrap_width,
            );
            painter.galley(*pos, galley, role.color(theme));
        }
        Primitive::Rect { rect, role, radius } => {
            painter.rect_filled(*rect, *radius, role.color(theme));
        }
        Primitive::Ribbon {
            points,
            half_widths,
            role,
        } => {
            let feather = 1.0 / painter.pixels_per_point();
            painter.add(Shape::mesh(ribbon_mesh(
                points,
                half_widths,
                role.color(theme),
                feather,
            )));
        }
        Primitive::Polyline {
            points,
            width,
            role,
        } => {
            painter.add(Shape::line(
                points.clone(),
                Stroke::new(*width, role.color(theme)),
            ));
        }
        Primitive::Circle {
            center,
            radius,
            role,
        } => {
            painter.circle_filled(*center, *radius, role.color(theme));
        }
    }
}

/// Tessellates a variable-width stroke with a one-pixel anti-aliasing fringe
/// on both sides (epaint's own path fill only handles convex shapes).
pub fn ribbon_mesh(points: &[Pos2], half_widths: &[f32], color: Color32, feather: f32) -> Mesh {
    let mut mesh = Mesh::default();
    let count = points.len().min(half_widths.len());
    if count < 2 {
        return mesh;
    }
    for i in 0..count {
        let before = points[i.saturating_sub(1)];
        let after = points[(i + 1).min(count - 1)];
        let normal = (after - before).normalized().rot90();
        let half = half_widths[i].max(0.0);
        // Strokes thinner than a pixel fade out instead of disappearing.
        let coverage = ((2.0 * half) / feather).min(1.0);
        let inner = (half - feather * 0.5).max(0.0);
        let outer = half.max(feather * 0.5) + feather * 0.5;
        let solid = color.gamma_multiply(coverage);
        let p = points[i];
        mesh.colored_vertex(p - normal * outer, Color32::TRANSPARENT);
        mesh.colored_vertex(p - normal * inner, solid);
        mesh.colored_vertex(p + normal * inner, solid);
        mesh.colored_vertex(p + normal * outer, Color32::TRANSPARENT);
    }
    for i in 0..count as u32 - 1 {
        let a = 4 * i;
        let b = a + 4;
        for strip in 0..3 {
            mesh.add_triangle(a + strip, a + strip + 1, b + strip);
            mesh.add_triangle(a + strip + 1, b + strip + 1, b + strip);
        }
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, vec2};

    #[test]
    fn ribbon_has_four_vertices_per_point_and_eighteen_indices_per_segment() {
        let mesh = ribbon_mesh(
            &[pos2(0.0, 0.0), pos2(0.0, 10.0), pos2(2.0, 20.0)],
            &[1.0, 2.0, 1.0],
            Color32::RED,
            1.0,
        );
        assert_eq!(mesh.vertices.len(), 12);
        assert_eq!(mesh.indices.len(), 36);
    }

    #[test]
    fn degenerate_ribbon_is_empty() {
        assert!(ribbon_mesh(&[pos2(1.0, 1.0)], &[1.0], Color32::RED, 1.0).is_empty());
    }

    #[test]
    fn slice_keeps_primitives_starting_in_range_and_rebases_them() {
        let mut drawing = Drawing::default();
        for x in [0.0, 10.0, 20.0] {
            drawing.push(Primitive::Rect {
                rect: Rect::from_min_size(pos2(x, 0.0), vec2(5.0, 5.0)),
                role: Role::Text,
                radius: 0.0,
            });
        }
        let slice = drawing.slice_x(10.0..20.0, 10.0);
        assert_eq!(slice.primitives.len(), 1);
        let Primitive::Rect { rect, .. } = &slice.primitives[0] else {
            panic!("expected rect")
        };
        assert_eq!(rect.left(), 0.0);
    }

    #[test]
    fn every_role_resolves_in_both_themes() {
        for theme in [Theme::light(), Theme::dark()] {
            assert_eq!(Role::Text.color(&theme), theme.text);
            assert_ne!(
                Role::CalloutFill(CalloutKind::Theorem).color(&theme),
                theme.background
            );
            assert_eq!(
                Role::CalloutFill(CalloutKind::Definition).color(&theme),
                theme.accent_soft
            );
        }
    }
}
