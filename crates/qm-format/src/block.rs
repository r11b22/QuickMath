//! Block structure: headings, paragraphs, lists, quotes, callouts, code and
//! display math. Containers re-parse their dedented lines recursively, which
//! keeps every block's original line number.

use crate::ast::{Block, BlockKind, CalloutKind, DisplayMath, Inline, MathNode};
use crate::inline::parse_inlines;
use crate::line_syntax::{classify, is_code_fence_close, list_marker, CalloutHeader, LineStart};
use crate::lines::{indent_width, is_blank, strip_indent, Line};
use crate::math::parse_display_row;

/// Deeper containers are read as plain paragraphs, so pathological input
/// such as thousands of `>` cannot overflow the stack.
const MAX_NESTING: usize = 16;
/// Extra columns that make a line belong to the list item or callout above.
const CONTINUATION_INDENT: usize = 2;

/// Parses `lines` into blocks; `depth` is the container nesting level.
pub(crate) fn parse_blocks(lines: &[Line], depth: usize) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if is_blank(lines[index].text) {
            index += 1;
            continue;
        }
        let (kind, consumed) = parse_block(&lines[index..], depth);
        blocks.push(Block {
            kind,
            line: lines[index].number,
        });
        index += consumed.max(1);
    }
    blocks
}

/// Parses the block starting at `lines[0]`; returns it and the lines used.
fn parse_block(lines: &[Line], depth: usize) -> (BlockKind, usize) {
    let can_nest = depth < MAX_NESTING;
    match classify(lines[0].text.trim_start()) {
        LineStart::CodeFence { language } => parse_code(lines, language),
        LineStart::DisplayMath { .. } => parse_display_math(lines),
        LineStart::Heading { level, content } => (
            BlockKind::Heading {
                level,
                content: parse_inlines(content),
            },
            1,
        ),
        LineStart::Rule => (BlockKind::Rule, 1),
        LineStart::Callout(header) if can_nest => parse_callout(lines, header, depth),
        LineStart::Quote if can_nest => parse_quote(lines, depth),
        LineStart::ListItem(_) if can_nest => parse_list(lines, depth),
        _ => parse_paragraph(lines),
    }
}

/// Consecutive text lines; each source line break is kept as a line break.
fn parse_paragraph(lines: &[Line]) -> (BlockKind, usize) {
    let length = 1 + lines[1..]
        .iter()
        .take_while(|line| {
            !is_blank(line.text) && classify(line.text.trim_start()) == LineStart::Text
        })
        .count();
    let mut content = Vec::new();
    for (index, line) in lines[..length].iter().enumerate() {
        if index > 0 {
            content.push(Inline::LineBreak);
        }
        content.extend(parse_inlines(line.text.trim()));
    }
    (BlockKind::Paragraph(content), length)
}

fn parse_code(lines: &[Line], language: &str) -> (BlockKind, usize) {
    let fence_indent = indent_width(lines[0].text);
    let close = lines[1..]
        .iter()
        .position(|line| is_code_fence_close(line.text))
        .map(|offset| offset + 1);
    let content_end = close.unwrap_or(lines.len());
    let code = lines[1..content_end]
        .iter()
        .map(|line| strip_indent(line.text, fence_indent))
        .collect::<Vec<_>>()
        .join("\n");
    let language = Some(language.to_string()).filter(|language| !language.is_empty());
    let consumed = close.map_or(lines.len(), |close| close + 1);
    (BlockKind::Code { language, code }, consumed)
}

/// Consecutive `;;` lines form one display block, one row per line.
fn parse_display_math(lines: &[Line]) -> (BlockKind, usize) {
    let mut rows = Vec::new();
    let mut consumed = 0;
    for line in lines {
        let LineStart::DisplayMath { content } = classify(line.text.trim_start()) else {
            break;
        };
        consumed += 1;
        if !content.trim().is_empty() {
            rows.push(parse_display_row(content.trim()));
        }
    }
    (BlockKind::DisplayMath(DisplayMath { rows }), consumed)
}

/// A callout body is every line directly below the header (until a blank
/// line, or a heading, rule or callout at the header's indentation), plus
/// any later lines indented by at least two more columns than the header.
fn parse_callout(lines: &[Line], header: CalloutHeader, depth: usize) -> (BlockKind, usize) {
    let header_indent = indent_width(lines[0].text);
    let mut body = Vec::new();
    let mut index = 1;
    while index < lines.len() && !ends_callout_directly(lines[index], header_indent) {
        body.push(Line {
            text: strip_indent(lines[index].text, header_indent),
            number: lines[index].number,
        });
        index += 1;
    }
    let (continuation, end) =
        indented_continuation(lines, index, header_indent + CONTINUATION_INDENT);
    body.extend(continuation);
    let mut body = parse_blocks(&body, depth + 1);
    if header.kind == CalloutKind::Proof {
        drop_trailing_qed(&mut body);
    }
    let kind = BlockKind::Callout {
        kind: header.kind,
        title: parse_inlines(header.title),
        body,
    };
    (kind, end)
}

/// Every proof ends with an automatic ∎, so a hand-written `;;qed;;` at the
/// end of its last paragraph would show the mark twice.
fn drop_trailing_qed(body: &mut [Block]) {
    let Some(Block {
        kind: BlockKind::Paragraph(content),
        ..
    }) = body.last_mut()
    else {
        return;
    };
    while matches!(content.last(), Some(Inline::Text(text)) if text.trim().is_empty()) {
        content.pop();
    }
    let is_qed = |inline: &Inline| matches!(inline, Inline::Math(math) if matches!(math.as_slice(), [MathNode::Atom(atom)] if atom.text == "∎"));
    if content.last().is_some_and(is_qed) {
        content.pop();
    }
    if let Some(Inline::Text(text)) = content.last_mut() {
        text.truncate(text.trim_end().len());
    }
}

fn ends_callout_directly(line: Line, header_indent: usize) -> bool {
    if is_blank(line.text) {
        return true;
    }
    let starts_new_section = matches!(
        classify(line.text.trim_start()),
        LineStart::Heading { .. } | LineStart::Rule | LineStart::Callout(_)
    );
    starts_new_section && indent_width(line.text) <= header_indent
}

/// Collects lines from `start` indented by at least `min_indent`, including
/// blank lines between them, dedented by `min_indent`. Returns the lines and
/// the index just past them (trailing blank lines are not consumed).
fn indented_continuation<'a>(
    lines: &[Line<'a>],
    start: usize,
    min_indent: usize,
) -> (Vec<Line<'a>>, usize) {
    let mut collected = Vec::new();
    let mut cursor = start;
    while cursor < lines.len() {
        let next_content = (cursor..lines.len())
            .find(|&i| !is_blank(lines[i].text))
            .unwrap_or(lines.len());
        if next_content >= lines.len() || indent_width(lines[next_content].text) < min_indent {
            break;
        }
        for line in &lines[cursor..=next_content] {
            collected.push(Line {
                text: strip_indent(line.text, min_indent),
                number: line.number,
            });
        }
        cursor = next_content + 1;
    }
    (collected, cursor)
}

fn parse_quote(lines: &[Line], depth: usize) -> (BlockKind, usize) {
    let quoted: Vec<Line> = lines
        .iter()
        .map_while(|line| {
            let rest = line.text.trim_start().strip_prefix('>')?;
            let text = rest.strip_prefix(' ').unwrap_or(rest);
            Some(Line {
                text,
                number: line.number,
            })
        })
        .collect();
    let consumed = quoted.len();
    (BlockKind::Quote(parse_blocks(&quoted, depth + 1)), consumed)
}

/// Items are siblings when their markers sit at (about) the first item's
/// indentation; deeper-indented lines belong to the item above.
fn parse_list(lines: &[Line], depth: usize) -> (BlockKind, usize) {
    let base_indent = indent_width(lines[0].text);
    let is_ordered =
        list_marker(lines[0].text.trim_start()).is_some_and(|marker| marker.is_ordered);
    let mut items = Vec::new();
    let mut index = 0;
    loop {
        let (item_lines, end) = list_item_lines(lines, index, base_indent);
        items.push(parse_blocks(&item_lines, depth + 1));
        index = end;
        let next_content = (index..lines.len()).find(|&i| !is_blank(lines[i].text));
        match next_content {
            Some(next) if is_sibling_item(lines[next], base_indent, is_ordered) => index = next,
            _ => break,
        }
    }
    (
        BlockKind::List {
            ordered: is_ordered,
            items,
        },
        index,
    )
}

fn list_item_lines<'a>(
    lines: &[Line<'a>],
    index: usize,
    base_indent: usize,
) -> (Vec<Line<'a>>, usize) {
    let line = lines[index];
    let trimmed = line.text.trim_start();
    let content_offset = list_marker(trimmed).map_or(0, |marker| marker.content_offset);
    let content_column = indent_width(line.text) + content_offset;
    let mut item_lines = vec![Line {
        text: &trimmed[content_offset..],
        number: line.number,
    }];
    let (continuation, end) =
        indented_continuation(lines, index + 1, base_indent + CONTINUATION_INDENT);
    // Re-indent relative to the item content so nested structure survives.
    let extra = content_column.saturating_sub(base_indent + CONTINUATION_INDENT);
    item_lines.extend(continuation.into_iter().map(|l| Line {
        text: strip_indent(l.text, extra),
        number: l.number,
    }));
    (item_lines, end)
}

fn is_sibling_item(line: Line, base_indent: usize, is_ordered: bool) -> bool {
    let is_same_kind =
        list_marker(line.text.trim_start()).is_some_and(|marker| marker.is_ordered == is_ordered);
    is_same_kind && indent_width(line.text) < base_indent + CONTINUATION_INDENT
}
