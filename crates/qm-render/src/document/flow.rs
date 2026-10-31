//! Paragraph flow: words and inline math boxes broken into lines.
//!
//! egui's `LayoutJob` cannot embed custom boxes, so text is measured word by
//! word and placed together with math boxes on shared baselines here.

use egui::{pos2, vec2, FontId, Pos2, Rect};
use qm_format::{Inline, MathNode};

use crate::fonts;
use crate::math::{MathBox, MathLayout};
use crate::measure::TextMeasure;
use crate::paint::{Drawing, Hotspot, HotspotKind, Primitive, Role};

/// New Computer Modern's ascender and descender, in em.
pub const TEXT_ASCENT: f32 = 0.806;
pub const TEXT_DESCENT: f32 = 0.194;
/// Baseline-to-baseline distance for body text, in em.
pub const LINE_SPREAD: f32 = 1.45;
/// Minimum gap between lines pushed apart by tall inline math, in em.
const LINE_SKIP_MIN: f32 = 0.15;
/// Lines shorter than this fraction of the width are not stretched when justifying.
const JUSTIFY_MIN_FILL: f32 = 0.6;
const CODE_SCALE: f32 = 0.88;
const CODE_PADDING: f32 = 0.2;
/// Gap kept between the last word and a flush-right end mark (∎), in em.
const END_MARK_GAP: f32 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Center,
    Justify,
}

/// Styling inherited by nested inlines.
#[derive(Debug, Clone, PartialEq)]
pub struct InlineStyle {
    pub is_bold: bool,
    pub is_italic: bool,
    pub role: Role,
    pub link: Option<String>,
}

impl InlineStyle {
    pub fn plain(role: Role) -> InlineStyle {
        InlineStyle {
            is_bold: false,
            is_italic: false,
            role,
            link: None,
        }
    }

    fn family(&self) -> &'static str {
        match (self.is_bold, self.is_italic) {
            (false, false) => fonts::SERIF,
            (false, true) => fonts::SERIF_ITALIC,
            (true, false) => fonts::SERIF_BOLD,
            (true, true) => fonts::SERIF_BOLD_ITALIC,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Word {
    pub text: String,
    pub font: FontId,
    pub role: Role,
    /// Width including `padding` on both sides.
    pub width: f32,
    pub galley_offset_y: f32,
    pub link: Option<String>,
    /// A rounded background (inline code, tag chips) with this horizontal padding.
    pub background: Option<(Role, f32)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FlowItem {
    Word(Word),
    /// A breakable space. Only `stretchable` glue grows when justifying
    /// (spaces inside formulas keep their width).
    Glue {
        width: f32,
        is_stretchable: bool,
    },
    Math(MathBox),
    /// A forced line break.
    Break,
    /// Placed at the right margin of the last line (the proof ∎).
    EndMark(Word),
}

impl FlowItem {
    fn width(&self) -> f32 {
        match self {
            FlowItem::Word(word) | FlowItem::EndMark(word) => word.width,
            FlowItem::Glue { width, .. } => *width,
            FlowItem::Math(math) => math.width,
            FlowItem::Break => 0.0,
        }
    }

    fn is_unbreakable(&self) -> bool {
        matches!(self, FlowItem::Word(_) | FlowItem::Math(_))
    }
}

/// Turns inlines into measured flow items.
pub struct FlowBuilder<'m> {
    measure: &'m mut dyn TextMeasure,
    size: f32,
    items: Vec<FlowItem>,
    space_widths: Vec<(FontId, f32)>,
}

impl<'m> FlowBuilder<'m> {
    pub fn new(measure: &'m mut dyn TextMeasure, size: f32) -> Self {
        FlowBuilder {
            measure,
            size,
            items: Vec::new(),
            space_widths: Vec::new(),
        }
    }

    pub fn finish(self) -> Vec<FlowItem> {
        self.items
    }

    pub fn push_inlines(&mut self, inlines: &[Inline], style: &InlineStyle) {
        for inline in inlines {
            match inline {
                Inline::Text(text) => self.push_text(text, style),
                Inline::Strong(children) => self.push_inlines(
                    children,
                    &InlineStyle {
                        is_bold: true,
                        ..style.clone()
                    },
                ),
                Inline::Emphasis(children) => self.push_inlines(
                    children,
                    &InlineStyle {
                        is_italic: !style.is_italic,
                        ..style.clone()
                    },
                ),
                Inline::Code(code) => self.push_code(code),
                Inline::Math(math) => self.push_math(math),
                Inline::WikiLink { target, label } => {
                    let link_style = InlineStyle {
                        role: Role::Link,
                        link: Some(target.clone()),
                        ..style.clone()
                    };
                    self.push_text(label.as_deref().unwrap_or(target), &link_style);
                }
                Inline::LineBreak => self.items.push(FlowItem::Break),
            }
        }
    }

    /// Splits `text` at whitespace into words and stretchable glue.
    pub fn push_text(&mut self, text: &str, style: &InlineStyle) {
        let font = FontId::new(self.size, fonts::family(style.family()));
        let mut rest = text;
        while !rest.is_empty() {
            let word_end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            if word_end > 0 {
                self.push_word(
                    &rest[..word_end],
                    font.clone(),
                    style.role,
                    style.link.clone(),
                    None,
                );
            }
            rest = &rest[word_end..];
            let space_end = rest
                .find(|c: char| !c.is_whitespace())
                .unwrap_or(rest.len());
            if space_end > 0 {
                let width = self.space_width(&font);
                self.items.push(FlowItem::Glue {
                    width,
                    is_stretchable: true,
                });
            }
            rest = &rest[space_end..];
        }
    }

    pub fn push_word(
        &mut self,
        text: &str,
        font: FontId,
        role: Role,
        link: Option<String>,
        background: Option<(Role, f32)>,
    ) {
        let word = self.word(text, font, role, link, background);
        self.items.push(FlowItem::Word(word));
    }

    pub fn push_glue(&mut self, width: f32) {
        self.items.push(FlowItem::Glue {
            width,
            is_stretchable: true,
        });
    }

    fn push_code(&mut self, code: &str) {
        let font = FontId::new(self.size * CODE_SCALE, fonts::family(fonts::MONO));
        let padding = CODE_PADDING * self.size;
        self.push_word(
            code,
            font,
            Role::Text,
            None,
            Some((Role::SurfaceRaised, padding)),
        );
    }

    /// Inline math, split at its permitted break points.
    pub fn push_math(&mut self, math: &[MathNode]) {
        let laid_out = MathLayout::new(self.measure, self.size).inline(math);
        let pieces = laid_out.split_at_breaks();
        let count = pieces.len();
        for (index, (piece, glue)) in pieces.into_iter().enumerate() {
            self.items.push(FlowItem::Math(piece));
            if index + 1 < count {
                self.items.push(FlowItem::Glue {
                    width: glue,
                    is_stretchable: false,
                });
            }
        }
    }

    /// The proof end mark, flush right on the last line.
    pub fn push_end_mark(&mut self, mark: &str) {
        let font = FontId::new(self.size, fonts::family(fonts::MATH));
        let word = self.word(mark, font, Role::Text, None, None);
        self.items.push(FlowItem::EndMark(word));
    }

    fn word(
        &mut self,
        text: &str,
        font: FontId,
        role: Role,
        link: Option<String>,
        background: Option<(Role, f32)>,
    ) -> Word {
        let measured = self.measure.measure(text, &font);
        let padding = background.map_or(0.0, |(_, p)| p);
        Word {
            text: text.to_owned(),
            font,
            role,
            width: measured.width + 2.0 * padding,
            galley_offset_y: measured.galley_offset.y,
            link,
            background,
        }
    }

    fn space_width(&mut self, font: &FontId) -> f32 {
        if let Some((_, width)) = self.space_widths.iter().find(|(f, _)| f == font) {
            return *width;
        }
        let width = self.measure.measure(" ", font).width;
        self.space_widths.push((font.clone(), width));
        width
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FlowOptions {
    pub width: f32,
    /// Body font size, which sets the line spacing and strut.
    pub size: f32,
    pub alignment: Alignment,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FlowLayout {
    pub drawing: Drawing,
    pub height: f32,
    pub first_baseline: f32,
    pub line_count: usize,
}

#[derive(Debug, Clone, Copy)]
struct Placed {
    item: usize,
    x: f32,
    stretchable_glues_before: usize,
}

#[derive(Debug, Default)]
struct Line {
    placed: Vec<Placed>,
    width: f32,
    stretchable_glues: usize,
    /// Ends with a forced break or the paragraph end: never justified.
    is_final: bool,
    end_mark: Option<usize>,
}

/// Greedy line breaking at glue; overlong unbreakable chunks overflow.
fn break_lines(items: &[FlowItem], width: f32) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut line = Line::default();
    let mut pending_glue: Option<(f32, bool)> = None;
    let mut index = 0;
    while index < items.len() {
        match &items[index] {
            FlowItem::Break => {
                line.is_final = true;
                lines.push(std::mem::take(&mut line));
                pending_glue = None;
            }
            FlowItem::Glue {
                width,
                is_stretchable,
            } => {
                if !line.placed.is_empty() {
                    let (w, s) = pending_glue.unwrap_or((0.0, false));
                    pending_glue = Some((w + width, s || *is_stretchable));
                }
            }
            FlowItem::EndMark(_) => line.end_mark = Some(index),
            _ => {
                let chunk_end = (index..items.len())
                    .find(|&i| !items[i].is_unbreakable())
                    .unwrap_or(items.len());
                let chunk_width: f32 = items[index..chunk_end].iter().map(FlowItem::width).sum();
                let glue = pending_glue.take();
                let glue_width = glue.map_or(0.0, |(w, _)| w);
                let mut x = line.width + glue_width;
                if !line.placed.is_empty() && x + chunk_width > width + 0.01 {
                    lines.push(std::mem::take(&mut line));
                    x = 0.0;
                } else if glue.is_some_and(|(_, s)| s) {
                    line.stretchable_glues += 1;
                }
                for (offset, chunk_item) in items[index..chunk_end].iter().enumerate() {
                    line.placed.push(Placed {
                        item: index + offset,
                        x,
                        stretchable_glues_before: line.stretchable_glues,
                    });
                    x += chunk_item.width();
                }
                line.width = x;
                index = chunk_end;
                continue;
            }
        }
        index += 1;
    }
    if !line.placed.is_empty() || line.end_mark.is_some() || lines.is_empty() {
        lines.push(line);
    }
    if let Some(last) = lines.last_mut() {
        last.is_final = true;
    }
    lines
}

/// Breaks `items` into lines and draws them.
pub fn layout_flow(items: &[FlowItem], options: FlowOptions) -> FlowLayout {
    let mut lines = break_lines(items, options.width);
    move_end_mark_to_own_line_if_needed(&mut lines, items, options);

    let size = options.size;
    let skip = LINE_SPREAD * size;
    let (strut_ascent, strut_descent) = (TEXT_ASCENT * size, TEXT_DESCENT * size);
    let leading = (skip - strut_ascent - strut_descent) / 2.0;

    let mut layout = FlowLayout {
        line_count: lines.len(),
        ..FlowLayout::default()
    };
    let mut baseline: Option<f32> = None;
    let mut previous_descent = strut_descent;
    for line in &lines {
        let (ascent, descent) = line_extent(line, items, strut_ascent, strut_descent);
        let y = match baseline {
            None => leading + ascent,
            Some(previous) => previous + skip.max(previous_descent + ascent + LINE_SKIP_MIN * size),
        };
        if baseline.is_none() {
            layout.first_baseline = y;
        }
        draw_line(&mut layout.drawing, line, items, y, options);
        baseline = Some(y);
        previous_descent = descent;
    }
    layout.height = baseline.unwrap_or(0.0) + previous_descent + leading;
    layout
}

fn move_end_mark_to_own_line_if_needed(
    lines: &mut Vec<Line>,
    items: &[FlowItem],
    options: FlowOptions,
) {
    let Some(last) = lines.last_mut() else { return };
    let Some(mark) = last.end_mark else { return };
    let needed = last.width + END_MARK_GAP * options.size + items[mark].width();
    if !last.placed.is_empty() && needed > options.width {
        last.end_mark = None;
        lines.push(Line {
            end_mark: Some(mark),
            is_final: true,
            ..Line::default()
        });
    }
}

fn line_extent(
    line: &Line,
    items: &[FlowItem],
    strut_ascent: f32,
    strut_descent: f32,
) -> (f32, f32) {
    line.placed
        .iter()
        .fold(
            (strut_ascent, strut_descent),
            |(a, d), placed| match &items[placed.item] {
                FlowItem::Word(word) => (
                    a.max(TEXT_ASCENT * word.font.size),
                    d.max(TEXT_DESCENT * word.font.size),
                ),
                FlowItem::Math(math) => (a.max(math.ascent), d.max(math.descent)),
                _ => (a, d),
            },
        )
}

fn draw_line(
    drawing: &mut Drawing,
    line: &Line,
    items: &[FlowItem],
    baseline: f32,
    options: FlowOptions,
) {
    let slack = (options.width - line.width).max(0.0);
    let (start, stretch) = match options.alignment {
        Alignment::Left => (0.0, 0.0),
        Alignment::Center => (slack / 2.0, 0.0),
        Alignment::Justify => {
            let should_stretch = !line.is_final
                && line.stretchable_glues > 0
                && line.width >= JUSTIFY_MIN_FILL * options.width;
            (
                0.0,
                if should_stretch {
                    slack / line.stretchable_glues as f32
                } else {
                    0.0
                },
            )
        }
    };
    for placed in &line.placed {
        let x = start + placed.x + stretch * placed.stretchable_glues_before as f32;
        match &items[placed.item] {
            FlowItem::Word(word) => draw_word(drawing, word, pos2(x, baseline)),
            FlowItem::Math(math) => drawing.append(&math.drawing, vec2(x, baseline)),
            _ => {}
        }
    }
    if let Some(FlowItem::EndMark(mark)) = line.end_mark.map(|i| &items[i]) {
        draw_word(drawing, mark, pos2(options.width - mark.width, baseline));
    }
}

fn draw_word(drawing: &mut Drawing, word: &Word, origin: Pos2) {
    let size = word.font.size;
    let rect = Rect::from_min_max(
        pos2(origin.x, origin.y - TEXT_ASCENT * size),
        pos2(origin.x + word.width, origin.y + TEXT_DESCENT * size),
    );
    let padding = word.background.map_or(0.0, |(role, padding)| {
        drawing.push(Primitive::Rect {
            rect: rect.expand2(vec2(0.0, 0.05 * size)),
            role,
            radius: 0.3 * size,
        });
        padding
    });
    drawing.push(Primitive::Text {
        pos: pos2(origin.x + padding, origin.y + word.galley_offset_y),
        text: word.text.clone(),
        font: word.font.clone(),
        role: word.role,
        wrap_width: f32::INFINITY,
    });
    if let Some(target) = &word.link {
        drawing.hotspots.push(Hotspot {
            rect,
            kind: HotspotKind::WikiLink(target.clone()),
        });
    }
}
