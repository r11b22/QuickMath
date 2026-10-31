//! Conversion of a QuickMath [`Document`] to Typst markup for PDF export.
//!
//! Text is backslash-escaped, and all structure (headings, lists, callouts,
//! code) is emitted as explicit `#function(...)[...]` calls terminated with
//! `;`, so no user text can be mistaken for markup and no line-start rules
//! (`=`, `-`, `+`) apply.

use qm_format::{Block, BlockKind, CalloutKind, DisplayMath, Document, FrontMatter, Inline};

use super::obsidian::callout_name;
use super::typst_math::{math_list, math_rows};
use super::wikilink_text;

const PREAMBLE: &str = r##"#set page(paper: "a4", margin: (x: 2.5cm, y: 2.5cm), numbering: "1")
#set text(font: "New Computer Modern", size: 11pt)
#show math.equation: set text(font: "New Computer Modern Math")
#show math.equation.where(block: true): set block(breakable: true)
#set par(justify: true)
#show raw: set text(font: "DejaVu Sans Mono", size: 9pt)
#show raw.where(block: true): block.with(width: 100%, fill: luma(246), inset: 8pt, radius: 3pt)
#show heading: set block(above: 1.4em, below: 0.8em)
#show quote.where(block: true): block.with(stroke: (left: 1.5pt + luma(180)), inset: (left: 10pt, y: 4pt))

#let qm-link = rgb("#4c51bf")

#let qm-callout(name, accent, number: none, title: none, qed: false, body) = {
  let label = if number == none { name } else { name + " " + str(number) }
  let heading = text(weight: "bold", fill: accent.darken(25%))[#label#if title != none [ (#title)];.]
  block(
    width: 100%,
    breakable: true,
    inset: (left: 10pt, right: 10pt, y: 8pt),
    fill: accent.lighten(92%),
    stroke: (left: 2.5pt + accent),
    radius: (right: 3pt),
    above: 1.2em,
    below: 1.2em,
  )[#heading #h(0.4em) #body#if qed [#h(1fr)$square$]]
}
"##;

/// Renders `document` as a complete Typst source file.
pub(crate) fn to_typst(document: &Document) -> String {
    let mut typst = document_metadata(&document.front_matter);
    typst.push_str(PREAMBLE);
    typst.push('\n');
    typst.push_str(&title_block(&document.front_matter));
    let mut renderer = BlockRenderer::default();
    typst.push_str(&renderer.blocks(&document.blocks));
    typst.push('\n');
    typst
}

/// `set document` must come before any content, so it precedes the preamble.
fn document_metadata(front_matter: &FrontMatter) -> String {
    let mut fields = vec!["date: none".to_string()];
    if let Some(title) = &front_matter.title {
        fields.push(format!("title: {}", string_literal(title)));
    }
    if let Some(author) = &front_matter.author {
        fields.push(format!("author: {}", string_literal(author)));
    }
    if !front_matter.tags.is_empty() {
        let tags: Vec<String> = front_matter
            .tags
            .iter()
            .map(|tag| string_literal(tag))
            .collect();
        fields.push(format!("keywords: ({},)", tags.join(", ")));
    }
    format!("#set document({})\n", fields.join(", "))
}

fn title_block(front_matter: &FrontMatter) -> String {
    let mut lines = Vec::new();
    if let Some(title) = &front_matter.title {
        lines.push(format!(
            "#text(size: 18pt, weight: \"bold\")[{}];",
            escape_text(title)
        ));
    }
    if let Some(course) = &front_matter.course {
        lines.push(format!(
            "#text(size: 12pt, style: \"italic\")[{}];",
            escape_text(course)
        ));
    }
    let byline: Vec<String> = [&front_matter.author, &front_matter.date]
        .into_iter()
        .flatten()
        .map(|value| escape_text(value))
        .collect();
    if !byline.is_empty() {
        lines.push(format!(
            "#text(fill: luma(80))[{}];",
            byline.join(" #sym.dot.c; ")
        ));
    }
    if lines.is_empty() {
        return String::new();
    }
    format!(
        "#align(center, block(below: 2em)[\n{}\n]);\n\n",
        lines.join(" \\\n")
    )
}

/// Walks the blocks; it numbers definitions, theorems, lemmas, corollaries
/// and examples with one shared counter, like the LaTeX export.
#[derive(Default)]
struct BlockRenderer {
    callout_counter: usize,
}

impl BlockRenderer {
    fn blocks(&mut self, blocks: &[Block]) -> String {
        blocks
            .iter()
            .map(|block| self.block(&block.kind))
            .filter(|typst| !typst.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn block(&mut self, kind: &BlockKind) -> String {
        match kind {
            BlockKind::Heading { level, content } => format!(
                "#heading(level: {})[{}];",
                (*level).clamp(1, 6),
                inlines(content).trim()
            ),
            BlockKind::Paragraph(content) => inlines(content).trim().to_string(),
            BlockKind::DisplayMath(math) => display_math(math),
            BlockKind::List { ordered, items } => self.list(*ordered, items),
            BlockKind::Callout { kind, title, body } => self.callout(*kind, title, body),
            BlockKind::Code { language, code } => code_block(language.as_deref(), code),
            BlockKind::Quote(body) => match self.blocks(body) {
                body if body.is_empty() => String::new(),
                body => format!("#quote(block: true)[{body}];"),
            },
            BlockKind::Rule => "#line(length: 100%, stroke: 0.5pt + luma(160));".to_string(),
        }
    }

    fn list(&mut self, ordered: bool, items: &[Vec<Block>]) -> String {
        let function = if ordered { "enum" } else { "list" };
        let is_tight = items.iter().all(|item| item.len() <= 1);
        let mut arguments = vec![format!("tight: {is_tight}")];
        arguments.extend(items.iter().map(|item| format!("[{}]", self.blocks(item))));
        format!("#{function}({});", arguments.join(", "))
    }

    fn callout(&mut self, kind: CalloutKind, title: &[Inline], body: &[Block]) -> String {
        let mut arguments = vec![
            string_literal(callout_name(kind)),
            format!("rgb(\"{}\")", callout_color(kind)),
        ];
        if is_numbered(kind) {
            self.callout_counter += 1;
            arguments.push(format!("number: {}", self.callout_counter));
        }
        let title = inlines(title);
        if !title.trim().is_empty() {
            arguments.push(format!("title: [{}]", title.trim()));
        }
        if kind == CalloutKind::Proof {
            arguments.push("qed: true".to_string());
        }
        format!(
            "#qm-callout({})[{}];",
            arguments.join(", "),
            self.blocks(body)
        )
    }
}

fn is_numbered(kind: CalloutKind) -> bool {
    !matches!(
        kind,
        CalloutKind::Proof | CalloutKind::Note | CalloutKind::Warning
    )
}

fn callout_color(kind: CalloutKind) -> &'static str {
    match kind {
        CalloutKind::Definition => "#1f6feb",
        CalloutKind::Theorem => "#8250df",
        CalloutKind::Lemma => "#0e7c86",
        CalloutKind::Corollary => "#6e40c9",
        CalloutKind::Proof => "#57606a",
        CalloutKind::Example => "#1a7f37",
        CalloutKind::Note => "#0969da",
        CalloutKind::Warning => "#bc4c00",
    }
}

fn display_math(math: &DisplayMath) -> String {
    let body = math_rows(&math.rows);
    if body.trim().is_empty() {
        return String::new();
    }
    format!("$ {body} $")
}

fn code_block(language: Option<&str>, code: &str) -> String {
    let code = code.strip_suffix('\n').unwrap_or(code);
    match language {
        Some(language) => format!(
            "#raw(block: true, lang: {}, {});",
            string_literal(language),
            string_literal(code)
        ),
        None => format!("#raw(block: true, {});", string_literal(code)),
    }
}

fn inlines(content: &[Inline]) -> String {
    content.iter().map(inline).collect()
}

fn inline(inline: &Inline) -> String {
    match inline {
        Inline::Text(text) => escape_text(text),
        Inline::Strong(content) => format!("#strong[{}];", inlines(content)),
        Inline::Emphasis(content) => format!("#emph[{}];", inlines(content)),
        Inline::Code(code) => format!("#raw({});", string_literal(code)),
        Inline::Math(math) => match math_list(math) {
            markup if markup.is_empty() => String::new(),
            // No spaces inside the dollars: that keeps the equation inline.
            markup => format!("${markup}$"),
        },
        Inline::WikiLink { target, label } => format!(
            "#text(fill: qm-link)[{}];",
            escape_text(&wikilink_text(target, label.as_deref()))
        ),
        Inline::LineBreak => "#linebreak();".to_string(),
    }
}

/// Escapes running text for Typst markup. ASCII punctuation that has a
/// meaning in markup is backslash-escaped; quotes are left alone so Typst
/// still typesets them as smart quotes. Newlines become spaces because a
/// paragraph is emitted on one line.
fn escape_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    let mut previous = None;
    for character in text.chars() {
        let needs_escape = match character {
            '\\' | '*' | '_' | '`' | '#' | '[' | ']' | '$' | '<' | '>' | '@' | '=' | '-' | '+'
            | '/' | '~' | ':' => true,
            // `1.` at the start of a line would begin a numbered list.
            '.' => previous.is_some_and(|c: char| c.is_ascii_digit()),
            _ => false,
        };
        if needs_escape {
            escaped.push('\\');
        }
        escaped.push(if character == '\n' { ' ' } else { character });
        previous = Some(character);
    }
    escaped
}

/// A Typst string literal (`"…"`) holding `text` verbatim.
pub(crate) fn string_literal(text: &str) -> String {
    let mut literal = String::with_capacity(text.len() + 2);
    literal.push('"');
    for character in text.chars() {
        match character {
            '\\' => literal.push_str("\\\\"),
            '"' => literal.push_str("\\\""),
            '\n' => literal.push_str("\\n"),
            '\r' => literal.push_str("\\r"),
            '\t' => literal.push_str("\\t"),
            control if control.is_control() => {
                literal.push_str(&format!("\\u{{{:x}}}", u32::from(control)))
            }
            other => literal.push(other),
        }
    }
    literal.push('"');
    literal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_text_neutralizes_markup() {
        assert_eq!(
            escape_text("*a* _b_ #c $d$"),
            "\\*a\\* \\_b\\_ \\#c \\$d\\$"
        );
        assert_eq!(escape_text("1. x // y"), "1\\. x \\/\\/ y");
        assert_eq!(escape_text("https://x.org"), "https\\:\\/\\/x.org");
    }

    #[test]
    fn string_literal_escapes_quotes_backslashes_and_controls() {
        assert_eq!(
            string_literal("a\"b\\c\nd\u{1}"),
            "\"a\\\"b\\\\c\\nd\\u{1}\""
        );
    }
}
