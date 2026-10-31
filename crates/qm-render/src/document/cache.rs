//! Per-block layout cache, so typing only re-lays out the edited block.
//!
//! Keys hash the block's content (with relative nested lines), the layout
//! width, the font size, the pixel density and the callout counters at the
//! block's start. Entries not used in a frame are dropped at the end of it.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use qm_format::Document;

use super::blocks::{BlockContext, BlockLayout, BlockLayouter, Counters};
use super::hash::{hash_block, hash_front_matter};
use crate::measure::TextMeasure;
use crate::paint::Role;

/// What every layout in a frame depends on besides the block itself.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutParams {
    pub width: f32,
    pub font_size: f32,
    pub pixels_per_point: f32,
}

impl LayoutParams {
    fn hash(&self, state: &mut impl Hasher) {
        (
            self.width.to_bits(),
            self.font_size.to_bits(),
            self.pixels_per_point.to_bits(),
        )
            .hash(state);
    }
}

#[derive(Debug, Clone)]
pub struct PlacedBlock {
    pub layout: Arc<BlockLayout>,
    /// Top edge relative to the page content's top.
    pub top: f32,
    pub line: usize,
}

#[derive(Debug, Clone, Default)]
pub struct PageLayout {
    pub title: Option<Arc<BlockLayout>>,
    pub blocks: Vec<PlacedBlock>,
    pub height: f32,
}

#[derive(Debug, Default)]
pub struct LayoutCache {
    blocks: HashMap<u64, Arc<BlockLayout>>,
    title: Option<(u64, Option<Arc<BlockLayout>>)>,
    /// Number of block layouts computed (not served from the cache) so far.
    pub misses: usize,
}

impl LayoutCache {
    /// Lays out the whole document, reusing every unchanged block.
    pub fn layout_page(
        &mut self,
        document: &Document,
        params: LayoutParams,
        measure: &mut dyn TextMeasure,
    ) -> PageLayout {
        let mut layouter = BlockLayouter::new(measure, params.font_size);
        let mut page = PageLayout {
            title: self.title_layout(document, params, &mut layouter),
            ..PageLayout::default()
        };
        let mut y = 0.0;
        let mut previous_space = None;
        if let Some(title) = &page.title {
            y = title.size.y;
            previous_space = Some(title.space_after);
        }

        let mut next = HashMap::with_capacity(document.blocks.len());
        let mut counters = Counters::default();
        for block in &document.blocks {
            let mut state = DefaultHasher::new();
            hash_block(block, block.line, &mut state);
            params.hash(&mut state);
            counters.hash(&mut state);
            let key = state.finish();
            let layout = match next.get(&key).cloned().or_else(|| self.blocks.remove(&key)) {
                Some(layout) => layout,
                None => {
                    self.misses += 1;
                    let mut context = BlockContext {
                        width: params.width,
                        text_role: Role::Text,
                        list_depth: 0,
                        counters,
                    };
                    Arc::new(layouter.block(block, block.line, &mut context))
                }
            };
            counters = layout.counters_after;
            next.insert(key, layout.clone());
            if let Some(previous) = previous_space {
                y += f32::max(previous, layout.space_before);
            }
            previous_space = Some(layout.space_after);
            let height = layout.size.y;
            page.blocks.push(PlacedBlock {
                layout,
                top: y,
                line: block.line,
            });
            y += height;
        }
        self.blocks = next;
        page.height = y;
        page
    }

    fn title_layout(
        &mut self,
        document: &Document,
        params: LayoutParams,
        layouter: &mut BlockLayouter<'_>,
    ) -> Option<Arc<BlockLayout>> {
        let mut state = DefaultHasher::new();
        hash_front_matter(&document.front_matter, &mut state);
        params.hash(&mut state);
        let key = state.finish();
        if let Some((cached_key, layout)) = &self.title {
            if *cached_key == key {
                return layout.clone();
            }
        }
        let layout = layouter
            .title(&document.front_matter, params.width)
            .map(Arc::new);
        self.title = Some((key, layout.clone()));
        layout
    }
}
