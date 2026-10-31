//! Block layout: headings, paragraphs, lists, callouts, code, quotes, rules,
//! display math and the title block.

use egui::{pos2, vec2, FontId, Rect, Vec2};
use qm_format::{Block, BlockKind, CalloutKind, FrontMatter, Inline};

use super::flow::{
    layout_flow, Alignment, FlowBuilder, FlowItem, FlowOptions, InlineStyle, LINE_SPREAD,
    TEXT_ASCENT,
};
use crate::fonts;
use crate::math::MathLayout;
use crate::measure::TextMeasure;
use crate::paint::{Drawing, Primitive, Role};

const HEADING_SCALES: [f32; 6] = [1.7, 1.4, 1.2, 1.05, 1.0, 1.0];
/// Space above and below headings, in body em.
const HEADING_SPACE_BEFORE: [f32; 6] = [1.4, 1.2, 1.0, 0.9, 0.8, 0.8];
const HEADING_SPACE_AFTER: f32 = 0.45;
const PARAGRAPH_GAP: f32 = 0.7;
const DISPLAY_MATH_GAP: f32 = 0.6;
const LIST_INDENT: f32 = 1.6;
const LIST_MARKER_GAP: f32 = 0.5;
const LIST_ITEM_GAP: f32 = 0.25;
const BULLETS: [&str; 4] = ["•", "–", "∗", "·"];
const CALLOUT_GAP: f32 = 0.9;
const CALLOUT_BAR_WIDTH: f32 = 3.0;
const CALLOUT_PADDING: Vec2 = Vec2::new(0.9, 0.55);
const CALLOUT_RADIUS: f32 = 6.0;
const CODE_SCALE: f32 = 0.85;
const CODE_PADDING: f32 = 0.7;
const CODE_LABEL_SCALE: f32 = 0.65;
const QUOTE_INDENT: f32 = 1.1;
const QUOTE_BAR_WIDTH: f32 = 3.0;
const RULE_HEIGHT: f32 = 1.2;
const TITLE_SCALE: f32 = 2.0;
const TAG_SCALE: f32 = 0.8;
const QED: &str = "∎";

/// Callout numbers so far. All numbered callouts share one counter, matching
/// the LaTeX and PDF exports.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Counters {
    pub numbered: u32,
}

impl Counters {
    /// Advances the counter and returns the callout's number, if numbered.
    fn next(&mut self, kind: CalloutKind) -> Option<u32> {
        if matches!(
            kind,
            CalloutKind::Proof | CalloutKind::Note | CalloutKind::Warning
        ) {
            return None;
        }
        self.numbered += 1;
        Some(self.numbered)
    }
}

/// A rectangle that maps back to a source line.
#[derive(Debug, Clone, PartialEq)]
pub struct LineRegion {
    pub rect: Rect,
    /// Line relative to the top-level block's line.
    pub line_offset: usize,
}

/// A laid-out block, drawn from its top-left corner.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BlockLayout {
    pub size: Vec2,
    pub drawing: Drawing,
    /// Baseline of the first line, for aligning list markers.
    pub first_baseline: Option<f32>,
    /// The block's own region first, then nested blocks'.
    pub regions: Vec<LineRegion>,
    pub space_before: f32,
    pub space_after: f32,
    /// Callout counters after this block (set for blocks laid out by [`BlockLayouter::block`]).
    pub counters_after: Counters,
}

impl BlockLayout {
    /// Marks the whole layout as coming from the line at `offset`.
    fn with_region(mut self, offset: usize) -> BlockLayout {
        self.regions = vec![LineRegion {
            rect: Rect::from_min_size(pos2(0.0, 0.0), self.size),
            line_offset: offset,
        }];
        self
    }
}

/// Inherited block settings.
#[derive(Debug, Clone)]
pub struct BlockContext {
    pub width: f32,
    pub text_role: Role,
    pub list_depth: usize,
    pub counters: Counters,
}

pub struct BlockLayouter<'m> {
    measure: &'m mut dyn TextMeasure,
    /// Body font size.
    size: f32,
}

impl<'m> BlockLayouter<'m> {
    pub fn new(measure: &'m mut dyn TextMeasure, size: f32) -> Self {
        BlockLayouter { measure, size }
    }

    /// Lays out `block`; nested lines are stored relative to `root_line`.
    pub fn block(
        &mut self,
        block: &Block,
        root_line: usize,
        context: &mut BlockContext,
    ) -> BlockLayout {
        let mut layout = match &block.kind {
            BlockKind::Heading { level, content } => self.heading(*level, content, context),
            BlockKind::Paragraph(content) => self.paragraph(content, context, None, false),
            BlockKind::DisplayMath(math) => self.display_math(math, context),
            BlockKind::List { ordered, items } => self.list(*ordered, items, root_line, context),
            BlockKind::Callout { kind, title, body } => {
                self.callout(*kind, title, body, root_line, context)
            }
            BlockKind::Code { language, code } => self.code(language.as_deref(), code, context),
            BlockKind::Quote(body) => self.quote(body, root_line, context),
            BlockKind::Rule => self.rule(context),
        };
        let own_region = LineRegion {
            rect: Rect::from_min_size(pos2(0.0, 0.0), layout.size),
            line_offset: block.line.saturating_sub(root_line),
        };
        layout.regions.insert(0, own_region);
        layout.counters_after = context.counters;
        layout
    }

    fn flow(&mut self, size: f32) -> FlowBuilder<'_> {
        FlowBuilder::new(self.measure, size)
    }

    fn heading(&mut self, level: u8, content: &[Inline], context: &BlockContext) -> BlockLayout {
        let index = usize::from(level.clamp(1, 6)) - 1;
        let size = self.size * HEADING_SCALES[index];
        let mut flow = self.flow(size);
        flow.push_inlines(
            content,
            &InlineStyle {
                is_bold: true,
                ..InlineStyle::plain(context.text_role)
            },
        );
        let items = flow.finish();
        let laid_out = layout_flow(
            &items,
            FlowOptions {
                width: context.width,
                size,
                alignment: Alignment::Left,
            },
        );
        BlockLayout {
            size: vec2(context.width, laid_out.height),
            drawing: laid_out.drawing,
            first_baseline: Some(laid_out.first_baseline),
            space_before: HEADING_SPACE_BEFORE[index] * self.size,
            space_after: HEADING_SPACE_AFTER * self.size,
            ..BlockLayout::default()
        }
    }

    /// A justified paragraph, optionally starting with a run-in `header`
    /// (callout label) and ending with a flush-right ∎.
    fn paragraph(
        &mut self,
        content: &[Inline],
        context: &BlockContext,
        header: Option<&[Inline]>,
        has_end_mark: bool,
    ) -> BlockLayout {
        let style = InlineStyle::plain(context.text_role);
        let mut flow = self.flow(self.size);
        if let Some(header) = header {
            flow.push_inlines(header, &style);
            if !content.is_empty() {
                flow.push_text(" ", &style);
            }
        }
        flow.push_inlines(content, &style);
        if has_end_mark {
            flow.push_end_mark(QED);
        }
        let items = flow.finish();
        let laid_out = layout_flow(
            &items,
            FlowOptions {
                width: context.width,
                size: self.size,
                alignment: Alignment::Justify,
            },
        );
        BlockLayout {
            size: vec2(context.width, laid_out.height),
            drawing: laid_out.drawing,
            first_baseline: Some(laid_out.first_baseline),
            space_before: PARAGRAPH_GAP * self.size,
            space_after: PARAGRAPH_GAP * self.size,
            ..BlockLayout::default()
        }
    }

    fn display_math(
        &mut self,
        math: &qm_format::DisplayMath,
        context: &BlockContext,
    ) -> BlockLayout {
        let laid_out = MathLayout::new(self.measure, self.size).display(math);
        let x = ((context.width - laid_out.width) / 2.0).max(0.0);
        let mut drawing = Drawing::default();
        drawing.append(&laid_out.drawing, vec2(x, laid_out.ascent));
        BlockLayout {
            size: vec2(context.width, laid_out.height()),
            drawing,
            first_baseline: Some(laid_out.ascent),
            space_before: DISPLAY_MATH_GAP * self.size,
            space_after: DISPLAY_MATH_GAP * self.size,
            ..BlockLayout::default()
        }
    }

    fn list(
        &mut self,
        is_ordered: bool,
        items: &[Vec<Block>],
        root_line: usize,
        context: &mut BlockContext,
    ) -> BlockLayout {
        let indent = LIST_INDENT * self.size;
        let mut item_context = BlockContext {
            width: (context.width - indent).max(1.0),
            list_depth: context.list_depth + 1,
            ..context.clone()
        };
        let marker_font = FontId::new(self.size, fonts::family(fonts::SERIF));
        let mut parts = Vec::with_capacity(items.len());
        for (number, item) in items.iter().enumerate() {
            let mut body = self.stack(item, root_line, &mut item_context, LIST_ITEM_GAP);
            let marker = if is_ordered {
                format!("{}.", number + 1)
            } else {
                BULLETS[context.list_depth % BULLETS.len()].to_owned()
            };
            let measured = self.measure.measure(&marker, &marker_font);
            let baseline = body
                .first_baseline
                .unwrap_or(TEXT_ASCENT * self.size + (LINE_SPREAD - 1.0) * self.size / 2.0);
            let mut drawing = Drawing::default();
            let marker_x = indent - LIST_MARKER_GAP * self.size - measured.width;
            drawing.push(Primitive::Text {
                pos: pos2(marker_x, baseline + measured.galley_offset.y),
                text: marker,
                font: marker_font.clone(),
                role: context.text_role,
                wrap_width: f32::INFINITY,
            });
            drawing.append(&body.drawing, vec2(indent, 0.0));
            let min_height = LINE_SPREAD * self.size;
            body.size = vec2(context.width, body.size.y.max(min_height));
            body.regions
                .iter_mut()
                .for_each(|r| r.rect = r.rect.translate(vec2(indent, 0.0)));
            body.drawing = drawing;
            body.space_before = LIST_ITEM_GAP * self.size;
            body.space_after = LIST_ITEM_GAP * self.size;
            body.first_baseline = Some(baseline);
            parts.push(body);
        }
        context.counters = item_context.counters;
        let mut list = stack_layouts(parts, context.width, LIST_ITEM_GAP * self.size);
        // Nested lists hug their parent item.
        let gap = if context.list_depth > 0 {
            LIST_ITEM_GAP
        } else {
            PARAGRAPH_GAP
        };
        list.space_before = gap * self.size;
        list.space_after = gap * self.size;
        list
    }

    /// Lays out child blocks top to bottom with collapsed spacing.
    fn stack(
        &mut self,
        blocks: &[Block],
        root_line: usize,
        context: &mut BlockContext,
        min_gap: f32,
    ) -> BlockLayout {
        let parts: Vec<BlockLayout> = blocks
            .iter()
            .map(|b| self.block(b, root_line, context))
            .collect();
        stack_layouts(parts, context.width, min_gap)
    }

    fn callout(
        &mut self,
        kind: CalloutKind,
        title: &[Inline],
        body: &[Block],
        root_line: usize,
        context: &mut BlockContext,
    ) -> BlockLayout {
        let number = context.counters.next(kind);
        let header = callout_header(kind, number, title);
        let padding = CALLOUT_PADDING * self.size;
        let inner_x = CALLOUT_BAR_WIDTH + padding.x;
        let mut inner = BlockContext {
            width: (context.width - inner_x - padding.x).max(1.0),
            ..context.clone()
        };
        let is_proof = kind == CalloutKind::Proof;

        let mut parts = Vec::with_capacity(body.len() + 1);
        let starts_with_paragraph = matches!(
            body.first(),
            Some(Block {
                kind: BlockKind::Paragraph(_),
                ..
            })
        );
        if !starts_with_paragraph {
            parts.push(self.paragraph(&[], &inner, Some(&header), is_proof && body.is_empty()));
        }
        for (index, block) in body.iter().enumerate() {
            let is_last = index + 1 == body.len();
            let part = match &block.kind {
                BlockKind::Paragraph(content) => {
                    let run_in = (index == 0).then_some(header.as_slice());
                    let has_end_mark = is_proof && is_last;
                    self.paragraph(content, &inner, run_in, has_end_mark)
                        .with_region(block.line.saturating_sub(root_line))
                }
                _ => self.block(block, root_line, &mut inner),
            };
            parts.push(part);
        }
        let ends_with_paragraph = matches!(
            body.last(),
            Some(Block {
                kind: BlockKind::Paragraph(_),
                ..
            })
        );
        if is_proof && !body.is_empty() && !ends_with_paragraph {
            parts.push(self.paragraph(&[], &inner, None, true));
        }
        context.counters = inner.counters;

        let content = stack_layouts(parts, inner.width, 0.0);
        let size = vec2(context.width, content.size.y + 2.0 * padding.y);
        let mut drawing = Drawing::default();
        let frame = Rect::from_min_size(pos2(0.0, 0.0), size);
        drawing.push(Primitive::Rect {
            rect: frame,
            role: Role::CalloutFill(kind),
            radius: CALLOUT_RADIUS,
        });
        let bar = Rect::from_min_size(pos2(0.0, 0.0), vec2(CALLOUT_BAR_WIDTH, size.y));
        drawing.push(Primitive::Rect {
            rect: bar,
            role: Role::CalloutBar(kind),
            radius: CALLOUT_BAR_WIDTH / 2.0,
        });
        let offset = vec2(inner_x, padding.y);
        drawing.append(&content.drawing, offset);
        BlockLayout {
            size,
            drawing,
            first_baseline: content.first_baseline.map(|b| b + padding.y),
            regions: content
                .regions
                .into_iter()
                .map(|r| LineRegion {
                    rect: r.rect.translate(offset),
                    ..r
                })
                .collect(),
            space_before: CALLOUT_GAP * self.size,
            space_after: CALLOUT_GAP * self.size,
            counters_after: context.counters,
        }
    }

    fn code(&mut self, language: Option<&str>, code: &str, context: &BlockContext) -> BlockLayout {
        let padding = CODE_PADDING * self.size;
        let font = FontId::new(self.size * CODE_SCALE, fonts::family(fonts::MONO));
        let text = code.trim_end_matches('\n');
        let wrap_width = (context.width - 2.0 * padding).max(1.0);
        let text_size = self.measure.measure_wrapped(text, &font, wrap_width);
        let label_font = FontId::new(self.size * CODE_LABEL_SCALE, fonts::family(fonts::MONO));
        let label = language
            .filter(|l| !l.is_empty())
            .map(|l| (l, self.measure.measure(l, &label_font)));
        let top = padding + label.map_or(0.0, |_| 0.6 * self.size);

        let size = vec2(context.width, top + text_size.y + padding);
        let mut drawing = Drawing::default();
        drawing.push(Primitive::Rect {
            rect: Rect::from_min_size(pos2(0.0, 0.0), size),
            role: Role::SurfaceRaised,
            radius: CALLOUT_RADIUS,
        });
        if let Some((label, measured)) = label {
            let pos = pos2(
                context.width - padding * 0.6 - measured.width,
                padding * 0.35,
            );
            drawing.push(Primitive::Text {
                pos,
                text: label.to_owned(),
                font: label_font,
                role: Role::Faint,
                wrap_width: f32::INFINITY,
            });
        }
        drawing.push(Primitive::Text {
            pos: pos2(padding, top),
            text: text.to_owned(),
            font,
            role: Role::Text,
            wrap_width,
        });
        BlockLayout {
            size,
            drawing,
            first_baseline: None,
            space_before: PARAGRAPH_GAP * self.size,
            space_after: PARAGRAPH_GAP * self.size,
            ..BlockLayout::default()
        }
    }

    fn quote(
        &mut self,
        body: &[Block],
        root_line: usize,
        context: &mut BlockContext,
    ) -> BlockLayout {
        let indent = QUOTE_INDENT * self.size;
        let mut inner = BlockContext {
            width: (context.width - indent).max(1.0),
            text_role: Role::Muted,
            ..context.clone()
        };
        let content = self.stack(body, root_line, &mut inner, 0.0);
        context.counters = inner.counters;
        let size = vec2(context.width, content.size.y.max(LINE_SPREAD * self.size));
        let mut drawing = Drawing::default();
        let bar = Rect::from_min_size(pos2(0.0, 0.0), vec2(QUOTE_BAR_WIDTH, size.y));
        drawing.push(Primitive::Rect {
            rect: bar,
            role: Role::Border,
            radius: QUOTE_BAR_WIDTH / 2.0,
        });
        drawing.append(&content.drawing, vec2(indent, 0.0));
        BlockLayout {
            size,
            drawing,
            first_baseline: content.first_baseline,
            regions: content
                .regions
                .into_iter()
                .map(|r| LineRegion {
                    rect: r.rect.translate(vec2(indent, 0.0)),
                    ..r
                })
                .collect(),
            space_before: PARAGRAPH_GAP * self.size,
            space_after: PARAGRAPH_GAP * self.size,
            ..BlockLayout::default()
        }
    }

    fn rule(&mut self, context: &BlockContext) -> BlockLayout {
        let height = RULE_HEIGHT * self.size;
        let thickness = 1.0 / self.measure.pixels_per_point();
        let mut drawing = Drawing::default();
        let line = Rect::from_min_size(pos2(0.0, height / 2.0), vec2(context.width, thickness));
        drawing.push(Primitive::Rect {
            rect: line,
            role: Role::Border,
            radius: 0.0,
        });
        BlockLayout {
            size: vec2(context.width, height),
            drawing,
            space_before: PARAGRAPH_GAP * self.size,
            space_after: PARAGRAPH_GAP * self.size,
            ..BlockLayout::default()
        }
    }

    /// Centred title, meta line and tag chips; `None` without front matter.
    pub fn title(&mut self, front_matter: &FrontMatter, width: f32) -> Option<BlockLayout> {
        let meta: Vec<&str> = [
            &front_matter.author,
            &front_matter.course,
            &front_matter.date,
        ]
        .into_iter()
        .flatten()
        .map(String::as_str)
        .collect();
        if front_matter.title.is_none() && meta.is_empty() && front_matter.tags.is_empty() {
            return None;
        }
        let mut parts = Vec::new();
        if let Some(title) = &front_matter.title {
            let size = self.size * TITLE_SCALE;
            let mut flow = self.flow(size);
            flow.push_text(title, &InlineStyle::plain(Role::Text));
            parts.push(centred(&flow.finish(), width, size));
        }
        if !meta.is_empty() {
            let mut flow = self.flow(self.size);
            flow.push_text(
                &meta.join("  ·  "),
                &InlineStyle {
                    is_italic: true,
                    ..InlineStyle::plain(Role::Muted)
                },
            );
            parts.push(centred(&flow.finish(), width, self.size));
        }
        if !front_matter.tags.is_empty() {
            let size = self.size * TAG_SCALE;
            let font = FontId::new(size, fonts::family(fonts::SERIF));
            let mut flow = self.flow(size);
            for tag in &front_matter.tags {
                flow.push_word(
                    tag,
                    font.clone(),
                    Role::Link,
                    None,
                    Some((Role::AccentSoft, 0.5 * size)),
                );
                flow.push_glue(0.4 * size);
            }
            let mut tags = centred(&flow.finish(), width, size);
            tags.space_before = 0.6 * self.size;
            parts.push(tags);
        }
        let mut title = stack_layouts(parts, width, 0.25 * self.size);
        title.space_after = 1.6 * self.size;
        Some(title)
    }
}

fn centred(items: &[FlowItem], width: f32, size: f32) -> BlockLayout {
    let laid_out = layout_flow(
        items,
        FlowOptions {
            width,
            size,
            alignment: Alignment::Center,
        },
    );
    BlockLayout {
        size: vec2(width, laid_out.height),
        drawing: laid_out.drawing,
        first_baseline: Some(laid_out.first_baseline),
        ..BlockLayout::default()
    }
}

/// The run-in label of a callout, e.g. **Theorem 2** (Title).
fn callout_header(kind: CalloutKind, number: Option<u32>, title: &[Inline]) -> Vec<Inline> {
    let name = match kind {
        CalloutKind::Definition => "Definition",
        CalloutKind::Theorem => "Theorem",
        CalloutKind::Lemma => "Lemma",
        CalloutKind::Corollary => "Corollary",
        CalloutKind::Proof => "Proof",
        CalloutKind::Example => "Example",
        CalloutKind::Note => "Note",
        CalloutKind::Warning => "Warning",
    };
    let label = match number {
        Some(n) => format!("{name} {n}"),
        None => name.to_owned(),
    };
    let wrap = |inlines: Vec<Inline>| {
        if kind == CalloutKind::Proof {
            Inline::Emphasis(inlines)
        } else {
            Inline::Strong(inlines)
        }
    };
    let mut header = vec![wrap(vec![Inline::Text(label)])];
    if !title.is_empty() {
        header.push(Inline::Text(" (".into()));
        header.extend(title.iter().cloned());
        header.push(Inline::Text(")".into()));
    }
    header.push(wrap(vec![Inline::Text(".".into())]));
    header
}

/// Stacks `parts` vertically, collapsing adjacent spacing to the larger of
/// the two (and at least `min_gap`).
pub fn stack_layouts(parts: Vec<BlockLayout>, width: f32, min_gap: f32) -> BlockLayout {
    let mut result = BlockLayout::default();
    let mut y = 0.0;
    let mut previous_space: Option<f32> = None;
    for (index, part) in parts.into_iter().enumerate() {
        if let Some(previous) = previous_space {
            y += previous.max(part.space_before).max(min_gap);
        }
        if index == 0 {
            result.space_before = part.space_before;
            result.first_baseline = part.first_baseline;
        }
        let offset = vec2(0.0, y);
        result.drawing.append(&part.drawing, offset);
        result
            .regions
            .extend(part.regions.into_iter().map(|r| LineRegion {
                rect: r.rect.translate(offset),
                ..r
            }));
        y += part.size.y;
        previous_space = Some(part.space_after);
        result.space_after = part.space_after;
    }
    result.size = vec2(width, y);
    result
}
