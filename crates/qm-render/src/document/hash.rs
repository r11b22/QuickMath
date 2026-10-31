//! Structural hashing of the syntax tree for the layout cache.
//!
//! The AST does not implement `Hash`, and its absolute line numbers would make
//! every block below an inserted line look changed. Nested blocks therefore
//! hash their line *relative to the top-level block*, which is also how
//! layouts store them, so a cached layout stays valid when its block moves.

use std::hash::{Hash, Hasher};
use std::mem::discriminant;

use qm_format::{Atom, AtomClass, Block, BlockKind, DisplayMath, FrontMatter, Inline, MathNode};

pub fn hash_block(block: &Block, root_line: usize, state: &mut impl Hasher) {
    block.line.saturating_sub(root_line).hash(state);
    hash_block_kind(&block.kind, root_line, state);
}

fn hash_blocks(blocks: &[Block], root_line: usize, state: &mut impl Hasher) {
    blocks.len().hash(state);
    blocks.iter().for_each(|b| hash_block(b, root_line, state));
}

fn hash_block_kind(kind: &BlockKind, root_line: usize, state: &mut impl Hasher) {
    discriminant(kind).hash(state);
    match kind {
        BlockKind::Heading { level, content } => {
            level.hash(state);
            hash_inlines(content, state);
        }
        BlockKind::Paragraph(content) => hash_inlines(content, state),
        BlockKind::DisplayMath(math) => hash_display_math(math, state),
        BlockKind::List { ordered, items } => {
            ordered.hash(state);
            items.len().hash(state);
            items
                .iter()
                .for_each(|item| hash_blocks(item, root_line, state));
        }
        BlockKind::Callout { kind, title, body } => {
            kind.hash(state);
            hash_inlines(title, state);
            hash_blocks(body, root_line, state);
        }
        BlockKind::Code { language, code } => {
            language.hash(state);
            code.hash(state);
        }
        BlockKind::Quote(body) => hash_blocks(body, root_line, state),
        BlockKind::Rule => {}
    }
}

pub fn hash_front_matter(front_matter: &FrontMatter, state: &mut impl Hasher) {
    let FrontMatter {
        title,
        author,
        date,
        course,
        tags,
    } = front_matter;
    (title, author, date, course, tags).hash(state);
}

fn hash_inlines(inlines: &[Inline], state: &mut impl Hasher) {
    inlines.len().hash(state);
    for inline in inlines {
        discriminant(inline).hash(state);
        match inline {
            Inline::Text(text) | Inline::Code(text) => text.hash(state),
            Inline::Strong(children) | Inline::Emphasis(children) => hash_inlines(children, state),
            Inline::Math(math) => hash_math(math, state),
            Inline::WikiLink { target, label } => (target, label).hash(state),
            Inline::LineBreak => {}
        }
    }
}

fn hash_display_math(math: &DisplayMath, state: &mut impl Hasher) {
    math.rows.len().hash(state);
    for row in &math.rows {
        row.cells.len().hash(state);
        row.cells.iter().for_each(|cell| hash_math(cell, state));
    }
}

pub fn hash_math(math: &[MathNode], state: &mut impl Hasher) {
    math.len().hash(state);
    math.iter().for_each(|node| hash_math_node(node, state));
}

fn hash_optional_math(math: &Option<Vec<MathNode>>, state: &mut impl Hasher) {
    math.is_some().hash(state);
    if let Some(math) = math {
        hash_math(math, state);
    }
}

fn hash_math_node(node: &MathNode, state: &mut impl Hasher) {
    discriminant(node).hash(state);
    match node {
        MathNode::Atom(atom) => hash_atom(atom, state),
        MathNode::Text(text) => text.hash(state),
        MathNode::Fraction {
            numerator,
            denominator,
        } => {
            hash_math(numerator, state);
            hash_math(denominator, state);
        }
        MathNode::Scripts { base, sub, sup } => {
            hash_math_node(base, state);
            hash_optional_math(sub, state);
            hash_optional_math(sup, state);
        }
        MathNode::Root { index, radicand } => {
            hash_optional_math(index, state);
            hash_math(radicand, state);
        }
        MathNode::Fenced { open, close, body } => {
            (open, close).hash(state);
            hash_math(body, state);
        }
        MathNode::Matrix { open, close, rows } => {
            (open, close).hash(state);
            rows.len().hash(state);
            for row in rows {
                row.len().hash(state);
                row.iter().for_each(|cell| hash_math(cell, state));
            }
        }
        MathNode::Accent { accent, body } => {
            discriminant(accent).hash(state);
            hash_math(body, state);
        }
        MathNode::Group(body) => hash_math(body, state),
        MathNode::Space(width) => discriminant(width).hash(state),
        MathNode::Error { source, message } => (source, message).hash(state),
    }
}

fn hash_atom(atom: &Atom, state: &mut impl Hasher) {
    // `latex` does not affect rendering.
    atom.text.hash(state);
    discriminant(&atom.class).hash(state);
    if let AtomClass::LargeOperator { limits } = atom.class {
        limits.hash(state);
    }
    discriminant(&atom.style).hash(state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use qm_format::{AtomStyle, MathRow};
    use std::hash::DefaultHasher;

    fn hash_of(block: &Block) -> u64 {
        let mut state = DefaultHasher::new();
        hash_block(block, block.line, &mut state);
        state.finish()
    }

    fn atom(text: &str) -> MathNode {
        MathNode::Atom(Atom {
            text: text.into(),
            latex: text.into(),
            class: AtomClass::Ordinary,
            style: AtomStyle::Italic,
        })
    }

    fn list_block(line: usize, item_line: usize, text: &str) -> Block {
        let item = Block {
            kind: BlockKind::Paragraph(vec![Inline::Text(text.into())]),
            line: item_line,
        };
        Block {
            kind: BlockKind::List {
                ordered: false,
                items: vec![vec![item]],
            },
            line,
        }
    }

    #[test]
    fn moving_a_block_keeps_its_hash() {
        assert_eq!(
            hash_of(&list_block(3, 4, "a")),
            hash_of(&list_block(10, 11, "a"))
        );
    }

    #[test]
    fn content_and_relative_lines_change_the_hash() {
        assert_ne!(
            hash_of(&list_block(3, 4, "a")),
            hash_of(&list_block(3, 4, "b"))
        );
        assert_ne!(
            hash_of(&list_block(3, 4, "a")),
            hash_of(&list_block(3, 5, "a"))
        );
    }

    #[test]
    fn math_structure_changes_the_hash() {
        let display = |cells: Vec<Vec<MathNode>>| Block {
            kind: BlockKind::DisplayMath(DisplayMath {
                rows: vec![MathRow { cells }],
            }),
            line: 0,
        };
        assert_ne!(
            hash_of(&display(vec![vec![atom("x")]])),
            hash_of(&display(vec![vec![atom("y")]]))
        );
        assert_ne!(
            hash_of(&display(vec![vec![atom("x")], vec![]])),
            hash_of(&display(vec![vec![atom("x")]]))
        );
    }
}
