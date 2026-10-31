//! The document preview: a scrollable "paper page" with cached block layouts.

mod blocks;
mod cache;
mod flow;
mod hash;

use std::sync::Arc;

use egui::mutex::Mutex;
use egui::{
    pos2, vec2, Align, CursorIcon, Pos2, Rect, Sense, Shadow, Stroke, StrokeKind, Ui, Vec2,
};
use qm_format::{Document, MathNode};

use crate::math::MathLayout;
use crate::measure::EguiMeasure;
use crate::paint::{Drawing, HotspotKind};
use crate::theme::Theme;
use crate::{PreviewOptions, PreviewResponse};
use cache::{LayoutCache, LayoutParams, PageLayout};

/// Body text size on the page, in points.
pub const BODY_FONT_SIZE: f32 = 17.0;
const MAX_PAGE_WIDTH: f32 = 800.0;
const PAGE_MARGIN: f32 = 24.0;
const NARROW_PAGE_MARGIN: f32 = 8.0;
const NARROW_WIDTH: f32 = 480.0;
const PAGE_PADDING_FRACTION: f32 = 0.08;
const PAGE_PADDING_RANGE: (f32, f32) = (16.0, 64.0);
const PAGE_RADIUS: f32 = 4.0;
/// `show_math` draws math this much larger than the ui's body text.
const UI_MATH_SCALE: f32 = 1.2;
/// Blocks this close to the viewport are painted too (avoids pop-in).
const PAINT_MARGIN: f32 = 64.0;

type SharedCache = Arc<Mutex<LayoutCache>>;

/// Where the page and its content sit for a given available width.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PageGeometry {
    margin: f32,
    page_width: f32,
    padding: f32,
}

impl PageGeometry {
    fn new(available_width: f32) -> PageGeometry {
        let margin = if available_width < NARROW_WIDTH {
            NARROW_PAGE_MARGIN
        } else {
            PAGE_MARGIN
        };
        let page_width = (available_width - 2.0 * margin).clamp(1.0, MAX_PAGE_WIDTH);
        let padding =
            (page_width * PAGE_PADDING_FRACTION).clamp(PAGE_PADDING_RANGE.0, PAGE_PADDING_RANGE.1);
        PageGeometry {
            margin,
            page_width,
            padding,
        }
    }

    fn content_width(&self) -> f32 {
        // Rounded so sub-pixel resizes don't invalidate the cache.
        (self.page_width - 2.0 * self.padding).max(1.0).round()
    }
}

pub fn show_document(
    ui: &mut Ui,
    document: &Document,
    theme: &Theme,
    options: &PreviewOptions,
) -> PreviewResponse {
    let id = ui.id().with("qm-preview");
    let cache: SharedCache = ui
        .ctx()
        .data_mut(|data| data.get_temp_mut_or_default::<SharedCache>(id).clone());
    ui.painter().rect_filled(ui.max_rect(), 0.0, theme.surface);
    egui::ScrollArea::vertical()
        .id_salt(id)
        .auto_shrink(false)
        .show(ui, |ui| {
            let geometry = PageGeometry::new(ui.available_width());
            let params = LayoutParams {
                width: geometry.content_width(),
                font_size: BODY_FONT_SIZE,
                pixels_per_point: ui.ctx().pixels_per_point(),
            };
            let page = ui.ctx().fonts_mut(|fonts| {
                cache.lock().layout_page(
                    document,
                    params,
                    &mut EguiMeasure::new(fonts, params.pixels_per_point),
                )
            });
            show_page(ui, &page, geometry, theme, options)
        })
        .inner
}

fn show_page(
    ui: &mut Ui,
    page: &PageLayout,
    geometry: PageGeometry,
    theme: &Theme,
    options: &PreviewOptions,
) -> PreviewResponse {
    let page_height = page.height + 2.0 * geometry.padding;
    let (outer, response) = ui.allocate_exact_size(
        vec2(ui.available_width(), page_height + 2.0 * geometry.margin),
        Sense::click(),
    );
    let page_rect = Rect::from_min_size(
        pos2(
            outer.center().x - geometry.page_width / 2.0,
            outer.top() + geometry.margin,
        ),
        vec2(geometry.page_width, page_height),
    );
    let origin = page_rect.min + Vec2::splat(geometry.padding);
    paint_page(ui, page, page_rect, origin, theme);

    if let Some(line) = options.scroll_to_line {
        if let Some(rect) = rect_for_line(page, line) {
            let rect = rect.translate(origin.to_vec2());
            let align = if rect.height() < ui.clip_rect().height() * 0.5 {
                Align::Center
            } else {
                Align::Min
            };
            ui.scroll_to_rect(rect, Some(align));
        }
    }

    let mut result = PreviewResponse::default();
    if response.double_clicked() {
        result.clicked_line = response
            .interact_pointer_pos()
            .map(|pos| line_at(page, pos - origin.to_vec2()));
    }
    let Some(pointer) = response.hover_pos() else {
        return result;
    };
    match hotspot_at(page, pointer - origin.to_vec2()) {
        Some(HotspotKind::WikiLink(target)) => {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            if response.clicked() {
                result.clicked_wikilink = Some(target);
            }
        }
        Some(HotspotKind::Tooltip(message)) => {
            response.on_hover_text_at_pointer(message);
        }
        None => {}
    }
    result
}

fn paint_page(ui: &Ui, page: &PageLayout, page_rect: Rect, origin: Pos2, theme: &Theme) {
    let painter = ui.painter();
    let shadow_strength = if theme.is_dark { 0.45 } else { 0.07 };
    let shadow = Shadow {
        offset: [0, 2],
        blur: 14,
        spread: 0,
        color: theme.text.gamma_multiply(shadow_strength),
    };
    painter.add(shadow.as_shape(page_rect, PAGE_RADIUS));
    painter.rect_filled(page_rect, PAGE_RADIUS, theme.background);
    painter.rect_stroke(
        page_rect,
        PAGE_RADIUS,
        Stroke::new(1.0, theme.border),
        StrokeKind::Inside,
    );

    let visible = ui.clip_rect().expand(PAINT_MARGIN);
    if let Some(title) = &page.title {
        title.drawing.paint(painter, origin.to_vec2(), theme);
    }
    for block in visible_blocks(page, visible, origin) {
        block
            .layout
            .drawing
            .paint(painter, origin.to_vec2() + vec2(0.0, block.top), theme);
    }
}

/// Blocks whose rows intersect `visible` (in screen coordinates).
fn visible_blocks(
    page: &PageLayout,
    visible: Rect,
    origin: Pos2,
) -> impl Iterator<Item = &cache::PlacedBlock> {
    let (top, bottom) = (visible.top() - origin.y, visible.bottom() - origin.y);
    let first = page
        .blocks
        .partition_point(|b| b.top + b.layout.size.y < top);
    page.blocks[first..]
        .iter()
        .take_while(move |b| b.top <= bottom)
}

/// The source line under `pos` (page-content coordinates): the innermost
/// block region containing it, else the nearest block above.
fn line_at(page: &PageLayout, pos: Pos2) -> usize {
    let index = page.blocks.partition_point(|b| b.top <= pos.y);
    let Some(block) = index.checked_sub(1).map(|i| &page.blocks[i]) else {
        return 0;
    };
    let local = pos - vec2(0.0, block.top);
    let innermost = block
        .layout
        .regions
        .iter()
        .filter(|r| r.rect.contains(local))
        .min_by(|a, b| a.rect.area().total_cmp(&b.rect.area()));
    block.line + innermost.map_or(0, |r| r.line_offset)
}

/// Rect (page-content coordinates) of the block starting at or before `line`.
fn rect_for_line(page: &PageLayout, line: usize) -> Option<Rect> {
    let index = page
        .blocks
        .partition_point(|b| b.line <= line)
        .checked_sub(1)?;
    let block = &page.blocks[index];
    let offset = line - block.line;
    let region = block
        .layout
        .regions
        .iter()
        .filter(|r| r.line_offset <= offset)
        .max_by_key(|r| r.line_offset)?;
    Some(region.rect.translate(vec2(0.0, block.top)))
}

fn hotspot_at(page: &PageLayout, pos: Pos2) -> Option<HotspotKind> {
    if let Some(title) = &page.title {
        if let Some(hotspot) = title.drawing.hotspot_at(pos) {
            return Some(hotspot.kind.clone());
        }
    }
    let index = page
        .blocks
        .partition_point(|b| b.top <= pos.y)
        .checked_sub(1)?;
    let block = &page.blocks[index];
    block
        .layout
        .drawing
        .hotspot_at(pos - vec2(0.0, block.top))
        .map(|h| h.kind.clone())
}

pub fn show_math(
    ui: &mut Ui,
    math: &[MathNode],
    is_display: bool,
    theme: &Theme,
) -> egui::Response {
    let size = egui::TextStyle::Body.resolve(ui.style()).size * UI_MATH_SCALE;
    let pixels_per_point = ui.ctx().pixels_per_point();
    let laid_out = ui.ctx().fonts_mut(|fonts| {
        let mut measure = EguiMeasure::new(fonts, pixels_per_point);
        let mut layout = MathLayout::new(&mut measure, size);
        if is_display {
            layout.display_expression(math)
        } else {
            layout.inline(math)
        }
    });
    let (rect, response) =
        ui.allocate_exact_size(vec2(laid_out.width, laid_out.height()), Sense::hover());
    let baseline_origin = vec2(rect.left(), rect.top() + laid_out.ascent);
    laid_out.drawing.paint(ui.painter(), baseline_origin, theme);
    let tooltip = response
        .hover_pos()
        .and_then(|pos| error_message_at(&laid_out.drawing, pos - baseline_origin));
    match tooltip {
        Some(message) => response.on_hover_text_at_pointer(message),
        None => response,
    }
}

fn error_message_at(drawing: &Drawing, pos: Pos2) -> Option<String> {
    match &drawing.hotspot_at(pos)?.kind {
        HotspotKind::Tooltip(message) => Some(message.clone()),
        HotspotKind::WikiLink(_) => None,
    }
}

#[cfg(test)]
mod tests;
