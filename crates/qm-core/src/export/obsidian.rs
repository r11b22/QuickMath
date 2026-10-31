//! Obsidian-flavoured Markdown export.
//!
//! Math is written for Obsidian's MathJax (`$…$`, `$$…$$`), wikilinks stay
//! `[[…]]` (minus the `.qm` suffix), front matter becomes YAML properties,
//! and callouts become Obsidian callouts. Callout types are mapped onto
//! Obsidian's built-in types so they get distinct colours and icons:
//!
//! | QuickMath  | Obsidian callout |
//! |------------|------------------|
//! | definition | `[!abstract]`    |
//! | theorem    | `[!important]`   |
//! | lemma      | `[!info]`        |
//! | corollary  | `[!info]`        |
//! | proof      | `[!quote]`       |
//! | example    | `[!example]`     |
//! | note       | `[!note]`        |
//! | warning    | `[!warning]`     |
//!
//! Every callout gets an explicit title that starts with the kind name
//! ("Definition", or "Definition (Group)" when the note gives a title), so a
//! definition never shows up as "Abstract".

use qm_format::{Block, BlockKind, CalloutKind, DisplayMath, Document, FrontMatter, Inline};

use super::{is_aligned, latex_rows, strip_note_extension};

/// Renders `document` as an Obsidian Markdown note.
pub fn to_markdown(document: &Document) -> String {
    let mut markdown = front_matter(&document.front_matter);
    markdown.push_str(&blocks(&document.blocks));
    if !markdown.ends_with('\n') {
        markdown.push('\n');
    }
    markdown
}

fn front_matter(front_matter: &FrontMatter) -> String {
    let fields = [
        ("title", &front_matter.title),
        ("author", &front_matter.author),
        ("date", &front_matter.date),
        ("course", &front_matter.course),
    ];
    let mut yaml = String::new();
    for (key, value) in fields {
        if let Some(value) = value {
            yaml.push_str(&format!("{key}: {}\n", yaml_string(value)));
        }
    }
    if !front_matter.tags.is_empty() {
        yaml.push_str("tags:\n");
        for tag in &front_matter.tags {
            yaml.push_str(&format!(
                "  - {}\n",
                yaml_string(tag.trim_start_matches('#'))
            ));
        }
    }
    if yaml.is_empty() {
        return yaml;
    }
    format!("---\n{yaml}---\n\n")
}

/// A double-quoted YAML scalar, which can hold any text once escaped.
fn yaml_string(value: &str) -> String {
    let mut quoted = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' => quoted.push_str("\\n"),
            '\t' => quoted.push_str("\\t"),
            control if control.is_control() => {
                quoted.push_str(&format!("\\u{:04x}", u32::from(control)))
            }
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}

fn blocks(blocks: &[Block]) -> String {
    blocks
        .iter()
        .map(|block| self::block(&block.kind))
        .filter(|markdown| !markdown.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn block(kind: &BlockKind) -> String {
    match kind {
        BlockKind::Heading { level, content } => {
            let hashes = "#".repeat(usize::from((*level).clamp(1, 6)));
            format!("{hashes} {}", inlines(content).trim())
        }
        BlockKind::Paragraph(content) => paragraph(content),
        BlockKind::DisplayMath(math) => display_math(math),
        BlockKind::List { ordered, items } => list(*ordered, items),
        BlockKind::Callout { kind, title, body } => callout(*kind, title, body),
        BlockKind::Code { language, code } => code_block(language.as_deref(), code),
        BlockKind::Quote(body) => prefix_lines(&blocks(body), "> ", ">"),
        BlockKind::Rule => "---".to_string(),
    }
}

/// Renders a paragraph, escaping a leading character that Markdown would
/// otherwise read as a list, quote or code-block marker.
fn paragraph(content: &[Inline]) -> String {
    let text = inlines(content);
    let text = text.trim_start();
    if text.starts_with(['>', '-', '+']) {
        return format!("\\{text}");
    }
    let digits = text.len() - text.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    if digits > 0 && text[digits..].starts_with(['.', ')']) {
        return format!("{}\\{}", &text[..digits], &text[digits..]);
    }
    text.to_string()
}

fn display_math(math: &DisplayMath) -> String {
    display_block(&latex_rows(math), is_aligned(math))
}

/// Wraps rendered rows in `$$`, inside `aligned` when cells are aligned or
/// `gathered` for several unaligned rows (both supported by MathJax).
fn display_block(rows: &[String], is_aligned: bool) -> String {
    match rows {
        [] => String::new(),
        [single] if !is_aligned => format!("$$\n{single}\n$$"),
        _ => {
            let environment = if is_aligned { "aligned" } else { "gathered" };
            format!(
                "$$\n\\begin{{{environment}}}\n{}\n\\end{{{environment}}}\n$$",
                rows.join(" \\\\\n")
            )
        }
    }
}

fn list(ordered: bool, items: &[Vec<Block>]) -> String {
    let is_tight = items.iter().all(|item| item.len() <= 1);
    let separator = if is_tight { "\n" } else { "\n\n" };
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let marker = if ordered {
                format!("{}. ", index + 1)
            } else {
                "- ".to_string()
            };
            let indent = " ".repeat(marker.len());
            let body = prefix_lines(&blocks(item), &indent, "");
            format!("{marker}{}", body.trim_start())
        })
        .collect::<Vec<_>>()
        .join(separator)
}

fn callout(kind: CalloutKind, title: &[Inline], body: &[Block]) -> String {
    let title = inlines(title);
    let title = match title.trim() {
        "" => callout_name(kind).to_string(),
        title => format!("{} ({title})", callout_name(kind)),
    };
    let header = format!("> [!{}] {title}", callout_type(kind));
    let body = blocks(body);
    if body.is_empty() {
        return header;
    }
    format!("{header}\n{}", prefix_lines(&body, "> ", ">"))
}

fn callout_type(kind: CalloutKind) -> &'static str {
    match kind {
        CalloutKind::Definition => "abstract",
        CalloutKind::Theorem => "important",
        CalloutKind::Lemma | CalloutKind::Corollary => "info",
        CalloutKind::Proof => "quote",
        CalloutKind::Example => "example",
        CalloutKind::Note => "note",
        CalloutKind::Warning => "warning",
    }
}

/// The human-readable name of a callout kind, shared with the other exporters.
pub(crate) fn callout_name(kind: CalloutKind) -> &'static str {
    match kind {
        CalloutKind::Definition => "Definition",
        CalloutKind::Theorem => "Theorem",
        CalloutKind::Lemma => "Lemma",
        CalloutKind::Corollary => "Corollary",
        CalloutKind::Proof => "Proof",
        CalloutKind::Example => "Example",
        CalloutKind::Note => "Note",
        CalloutKind::Warning => "Warning",
    }
}

fn code_block(language: Option<&str>, code: &str) -> String {
    let fence = "`".repeat(longest_backtick_run(code).max(2) + 1);
    let code = code.strip_suffix('\n').unwrap_or(code);
    format!("{fence}{}\n{code}\n{fence}", language.unwrap_or_default())
}

fn longest_backtick_run(text: &str) -> usize {
    text.split(|c| c != '`').map(str::len).max().unwrap_or(0)
}

/// Prefixes each line with `prefix`, or with `empty_prefix` for blank lines
/// (so quotes and callouts stay continuous without trailing spaces).
fn prefix_lines(text: &str, prefix: &str, empty_prefix: &str) -> String {
    if text.is_empty() {
        return empty_prefix.to_string();
    }
    text.lines()
        .map(|line| {
            if line.is_empty() {
                empty_prefix.to_string()
            } else {
                format!("{prefix}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn inlines(content: &[Inline]) -> String {
    content.iter().map(inline).collect()
}

fn inline(inline: &Inline) -> String {
    match inline {
        Inline::Text(text) => escape_text(text),
        Inline::Strong(content) => wrap_nonempty("**", &inlines(content)),
        Inline::Emphasis(content) => wrap_nonempty("*", &inlines(content)),
        Inline::Code(code) => inline_code(code),
        Inline::Math(math) => wrap_nonempty("$", qm_format::math_to_latex(math).trim()),
        Inline::WikiLink { target, label } => wikilink(target, label.as_deref()),
        Inline::LineBreak => "<br>".to_string(),
    }
}

fn wrap_nonempty(delimiter: &str, content: &str) -> String {
    if content.is_empty() {
        return String::new();
    }
    format!("{delimiter}{content}{delimiter}")
}

fn inline_code(code: &str) -> String {
    if code.is_empty() {
        return String::new();
    }
    let fence = "`".repeat(longest_backtick_run(code) + 1);
    let padding = if code.starts_with('`') || code.ends_with('`') {
        " "
    } else {
        ""
    };
    format!("{fence}{padding}{code}{padding}{fence}")
}

/// Keeps the link but drops the `.qm` suffix, since the exported vault
/// contains `.md` files with the same stems.
fn wikilink(target: &str, label: Option<&str>) -> String {
    let (path, anchor) = match target.find('#') {
        Some(hash) => target.split_at(hash),
        None => (target, ""),
    };
    let path = strip_note_extension(path);
    match label {
        Some(label) => format!("[[{path}{anchor}|{label}]]"),
        None => format!("[[{path}{anchor}]]"),
    }
}

/// Escapes characters Markdown (or Obsidian) would treat as markup.
fn escape_text(text: &str) -> String {
    let characters: Vec<char> = text.chars().collect();
    let mut escaped = String::with_capacity(text.len());
    for (index, &character) in characters.iter().enumerate() {
        let is_doubled = |c: char| {
            characters.get(index + 1) == Some(&c) || (index > 0 && characters[index - 1] == c)
        };
        let needs_escape = match character {
            '\\' | '`' | '*' | '_' | '$' | '[' | ']' | '<' | '#' => true,
            '~' | '=' => is_doubled(character),
            _ => false,
        };
        if needs_escape {
            escaped.push('\\');
        }
        escaped.push(if character == '\n' { ' ' } else { character });
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::fixtures::{callout, display, kitchen_sink, paragraph as para, text};

    fn markdown_of(blocks: Vec<Block>) -> String {
        to_markdown(&Document {
            blocks,
            ..Document::default()
        })
    }

    #[test]
    fn front_matter_becomes_yaml_properties() {
        let markdown = to_markdown(&kitchen_sink());
        assert!(markdown.starts_with(
            "---\ntitle: \"Linear \\\"Algebra\\\" & #1\"\nauthor: \"Ada Lovelace\"\ndate: \"2026-10-31\"\ncourse: \"MATH_101 {intro}\"\ntags:\n  - \"math\"\n  - \"lecture\"\n---\n\n"
        ));
    }

    #[test]
    fn empty_document_has_no_front_matter() {
        assert_eq!(to_markdown(&Document::default()), "\n");
    }

    #[test]
    fn display_rows_use_double_dollars_and_aligned() {
        let rows = |rows: &[&str]| rows.iter().map(|row| row.to_string()).collect::<Vec<_>>();
        assert_eq!(display_block(&rows(&["x = 1"]), false), "$$\nx = 1\n$$");
        assert_eq!(
            display_block(&rows(&["a & = b", "& = c"]), true),
            "$$\n\\begin{aligned}\na & = b \\\\\n& = c\n\\end{aligned}\n$$"
        );
        assert_eq!(
            display_block(&rows(&["u", "v"]), false),
            "$$\n\\begin{gathered}\nu \\\\\nv\n\\end{gathered}\n$$"
        );
    }

    #[test]
    fn empty_display_math_is_omitted() {
        assert_eq!(
            markdown_of(vec![display(vec![]), display(vec![vec![vec![]]])]),
            "\n"
        );
    }

    #[test]
    fn callouts_become_obsidian_callouts_with_titles() {
        let markdown = markdown_of(vec![
            callout(
                CalloutKind::Definition,
                "Group",
                vec![para(vec![text("A set.")]), para(vec![text("More.")])],
            ),
            callout(CalloutKind::Proof, "", vec![]),
            callout(CalloutKind::Theorem, "", vec![]),
        ]);
        assert_eq!(
            markdown,
            "> [!abstract] Definition (Group)\n> A set.\n>\n> More.\n\n> [!quote] Proof\n\n> [!important] Theorem\n"
        );
    }

    #[test]
    fn nested_lists_are_indented_under_their_marker() {
        let markdown = markdown_of(vec![Block {
            kind: BlockKind::List {
                ordered: true,
                items: vec![
                    vec![para(vec![text("one")])],
                    vec![
                        para(vec![text("two")]),
                        Block {
                            kind: BlockKind::List {
                                ordered: false,
                                items: vec![vec![para(vec![text("inner")])]],
                            },
                            line: 0,
                        },
                    ],
                ],
            },
            line: 0,
        }]);
        assert_eq!(markdown, "1. one\n\n2. two\n\n   - inner\n");
    }

    #[test]
    fn inline_markup_quotes_and_rules() {
        let markdown = markdown_of(vec![
            para(vec![
                Inline::Strong(vec![text("b")]),
                text(" "),
                Inline::Emphasis(vec![text("e")]),
                text(" "),
                Inline::Code("x".into()),
                Inline::LineBreak,
                Inline::WikiLink {
                    target: "a.qm".into(),
                    label: Some("A".into()),
                },
            ]),
            Block {
                kind: BlockKind::Quote(vec![para(vec![text("q")])]),
                line: 0,
            },
            Block {
                kind: BlockKind::Rule,
                line: 0,
            },
        ]);
        assert_eq!(markdown, "**b** *e* `x`<br>[[a|A]]\n\n> q\n\n---\n");
    }

    #[test]
    fn escape_text_escapes_markdown_specials() {
        assert_eq!(
            escape_text("a*b_c [x] $5 #tag"),
            "a\\*b\\_c \\[x\\] \\$5 \\#tag"
        );
        assert_eq!(escape_text("x = y == z ~ ~~"), "x = y \\=\\= z ~ \\~\\~");
    }

    #[test]
    fn paragraph_escapes_block_markers_at_the_start() {
        let text = |s: &str| vec![Inline::Text(s.to_string())];
        assert_eq!(paragraph(&text("- not a list")), "\\- not a list");
        assert_eq!(paragraph(&text("> not a quote")), "\\> not a quote");
        assert_eq!(paragraph(&text("1. not a list")), "1\\. not a list");
        assert_eq!(paragraph(&text("    not code")), "not code");
    }

    #[test]
    fn wikilink_drops_note_extension_but_keeps_anchor() {
        assert_eq!(
            wikilink("week1/intro.qm#Goals", None),
            "[[week1/intro#Goals]]"
        );
        assert_eq!(wikilink("Topology", Some("topo")), "[[Topology|topo]]");
    }

    #[test]
    fn code_fence_is_longer_than_backticks_in_code() {
        assert_eq!(
            code_block(Some("md"), "```\nx\n```\n"),
            "````md\n```\nx\n```\n````"
        );
    }
}
