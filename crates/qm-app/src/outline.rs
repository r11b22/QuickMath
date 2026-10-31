//! The sidebar outline: the current note's headings, click to jump.

use qm_format::{AtomClass, BlockKind, Document, Inline, MathNode};
use qm_render::Theme;

/// One heading in the outline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineEntry {
    /// 1..=6
    pub level: u8,
    pub title: String,
    /// Zero-based source line of the heading.
    pub line: usize,
}

/// Lists the document's top-level headings in source order.
pub fn extract_outline(document: &Document) -> Vec<OutlineEntry> {
    document
        .blocks
        .iter()
        .filter_map(|block| match &block.kind {
            BlockKind::Heading { level, content } => Some(OutlineEntry {
                level: *level,
                title: plain_text(content),
                line: block.line,
            }),
            _ => None,
        })
        .collect()
}

/// Flattens inline content to a single line of readable text.
fn plain_text(inlines: &[Inline]) -> String {
    let mut text = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(content) | Inline::Code(content) => text.push_str(content),
            Inline::Strong(children) | Inline::Emphasis(children) => {
                text.push_str(&plain_text(children))
            }
            Inline::Math(math) => push_math_text(&mut text, math),
            Inline::WikiLink { target, label } => text.push_str(label.as_deref().unwrap_or(target)),
            Inline::LineBreak => text.push(' '),
        }
    }
    text.trim().to_string()
}

/// Appends a one-line Unicode approximation of `math`, e.g. `a/b` for a fraction.
fn push_math_text(text: &mut String, math: &[MathNode]) {
    for node in math {
        match node {
            MathNode::Atom(atom) if atom.class == AtomClass::Relation => {
                text.push(' ');
                text.push_str(&atom.text);
                text.push(' ');
            }
            MathNode::Atom(atom) => text.push_str(&atom.text),
            MathNode::Text(content)
            | MathNode::Error {
                source: content, ..
            } => text.push_str(content),
            MathNode::Fraction {
                numerator,
                denominator,
            } => {
                push_math_text(text, numerator);
                text.push('/');
                push_math_text(text, denominator);
            }
            MathNode::Scripts { base, sub, sup } => {
                push_math_text(text, std::slice::from_ref(base.as_ref()));
                for (marker, script) in [('_', sub), ('^', sup)] {
                    if let Some(script) = script {
                        text.push(marker);
                        push_math_text(text, script);
                    }
                }
            }
            MathNode::Root { radicand, .. } => {
                text.push('√');
                push_math_text(text, radicand);
            }
            MathNode::Fenced { open, close, body } => {
                text.push_str(open);
                push_math_text(text, body);
                text.push_str(close);
            }
            MathNode::Accent { body, .. } | MathNode::Group(body) => push_math_text(text, body),
            MathNode::Matrix { .. } => text.push_str("[…]"),
            MathNode::Space(_) => text.push(' '),
        }
    }
}

const INDENT_PER_LEVEL: f32 = 12.0;

/// Draws the outline and returns the line of a heading the user clicked.
pub fn show_outline(ui: &mut egui::Ui, entries: &[OutlineEntry], theme: &Theme) -> Option<usize> {
    if entries.is_empty() {
        ui.label(
            egui::RichText::new("No headings yet. Start a line with # to add one.")
                .color(theme.text_faint)
                .small(),
        );
        return None;
    }
    let mut clicked_line = None;
    let min_level = entries.iter().map(|entry| entry.level).min().unwrap_or(1);
    egui::ScrollArea::vertical()
        .id_salt("outline_scroll")
        // Filling the panel matters: a shrinking scroll area and the
        // resizable panel around it collapse each other to a few rows.
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for entry in entries {
                let indent = f32::from(entry.level - min_level) * INDENT_PER_LEVEL;
                ui.horizontal(|ui| {
                    ui.add_space(indent);
                    let color = if entry.level == min_level {
                        theme.text
                    } else {
                        theme.text_muted
                    };
                    let title = if entry.title.is_empty() {
                        "(untitled)"
                    } else {
                        entry.title.as_str()
                    };
                    let button = egui::Button::new(egui::RichText::new(title).color(color))
                        .frame(false)
                        .truncate();
                    if ui
                        .add(button)
                        .on_hover_text(format!("Line {}", entry.line + 1))
                        .clicked()
                    {
                        clicked_line = Some(entry.line);
                    }
                });
            }
        });
    clicked_line
}

#[cfg(test)]
mod tests {
    use super::*;
    use qm_format::Block;

    fn heading(level: u8, content: Vec<Inline>, line: usize) -> Block {
        Block {
            kind: BlockKind::Heading { level, content },
            line,
        }
    }

    #[test]
    fn extracts_headings_in_order_with_lines() {
        let document = Document {
            blocks: vec![
                heading(1, vec![Inline::Text("Limits".into())], 0),
                Block {
                    kind: BlockKind::Paragraph(vec![Inline::Text("body".into())]),
                    line: 2,
                },
                heading(
                    2,
                    vec![
                        Inline::Text("Epsilon ".into()),
                        Inline::Strong(vec![Inline::Text("delta".into())]),
                    ],
                    4,
                ),
            ],
            ..Document::default()
        };
        let outline = extract_outline(&document);
        assert_eq!(
            outline,
            vec![
                OutlineEntry {
                    level: 1,
                    title: "Limits".into(),
                    line: 0
                },
                OutlineEntry {
                    level: 2,
                    title: "Epsilon delta".into(),
                    line: 4
                },
            ]
        );
    }

    #[test]
    fn empty_document_has_empty_outline() {
        assert!(extract_outline(&Document::default()).is_empty());
    }

    #[test]
    fn wikilinks_use_label_and_line_breaks_become_spaces() {
        let content = vec![
            Inline::WikiLink {
                target: "notes/a".into(),
                label: Some("Intro".into()),
            },
            Inline::LineBreak,
            Inline::WikiLink {
                target: "B".into(),
                label: None,
            },
        ];
        assert_eq!(plain_text(&content), "Intro B");
    }

    #[test]
    fn math_in_headings_reads_as_unicode_not_latex() {
        let document = qm_format::parse_document("# Roots of ;;x sq = 1/2;;");

        let outline = extract_outline(&document);

        assert_eq!(outline[0].title, "Roots of x^2 = 1/2");
    }
}
