//! Standalone LaTeX export: a complete `article` that compiles with
//! pdfLaTeX, XeLaTeX or LuaLaTeX.
//!
//! Callouts map onto `amsthm` environments (`definition`, `theorem`, `lemma`,
//! `corollary`, `example`, `proof`, plus unnumbered `note` and `warning`),
//! which share one counter. Headings are unnumbered sections, code uses
//! `listings`, and wikilinks become their (emphasised) label text.

use qm_format::{
    math_to_latex, Block, BlockKind, CalloutKind, DisplayMath, Document, FrontMatter, Inline,
};

use super::{is_aligned, latex_rows, wikilink_text};

const PREAMBLE: &str = r"\documentclass[11pt,a4paper]{article}
\usepackage{iftex}
\ifPDFTeX
  \usepackage[utf8]{inputenc}
  \usepackage[T1]{fontenc}
  \usepackage{lmodern}
\else
  \usepackage{fontspec}
\fi
\usepackage[margin=2.5cm]{geometry}
\usepackage{amsmath}
\usepackage{amssymb}
\usepackage{amsthm}
\usepackage{listings}
\usepackage{hyperref}

\setcounter{secnumdepth}{0}
\lstset{basicstyle=\ttfamily\small, breaklines=true, columns=fullflexible, frame=single, upquote=true}

\theoremstyle{plain}
\newtheorem{theorem}{Theorem}
\newtheorem{lemma}[theorem]{Lemma}
\newtheorem{corollary}[theorem]{Corollary}
\theoremstyle{definition}
\newtheorem{definition}[theorem]{Definition}
\newtheorem{example}[theorem]{Example}
\theoremstyle{remark}
\newtheorem*{note}{Note}
\newtheorem*{warning}{Warning}
";

/// Renders `document` as a standalone `.tex` file.
pub fn to_latex(document: &Document) -> String {
    let front_matter = &document.front_matter;
    let mut latex = String::from(PREAMBLE);
    latex.push_str(&hyperref_setup(front_matter));
    latex.push_str(&title_commands(front_matter));
    latex.push_str("\n\\begin{document}\n");
    if has_title(front_matter) {
        latex.push_str("\\maketitle\n");
    }
    let body = blocks(&document.blocks);
    if !body.is_empty() {
        latex.push('\n');
        latex.push_str(&body);
        latex.push('\n');
    }
    latex.push_str("\n\\end{document}\n");
    latex
}

fn has_title(front_matter: &FrontMatter) -> bool {
    front_matter.title.is_some() || front_matter.course.is_some()
}

fn hyperref_setup(front_matter: &FrontMatter) -> String {
    let mut options = vec!["colorlinks=true".to_string(), "linkcolor=blue".to_string()];
    if let Some(title) = &front_matter.title {
        options.push(format!("pdftitle={{{}}}", escape_text(title)));
    }
    if let Some(author) = &front_matter.author {
        options.push(format!("pdfauthor={{{}}}", escape_text(author)));
    }
    if !front_matter.tags.is_empty() {
        let tags: Vec<_> = front_matter
            .tags
            .iter()
            .map(|tag| escape_text(tag))
            .collect();
        options.push(format!("pdfkeywords={{{}}}", tags.join(", ")));
    }
    format!("\\hypersetup{{{}}}\n", options.join(", "))
}

fn title_commands(front_matter: &FrontMatter) -> String {
    if !has_title(front_matter) {
        return String::new();
    }
    let course = front_matter.course.as_deref().map(escape_text);
    let title = match (front_matter.title.as_deref().map(escape_text), course) {
        (Some(title), Some(course)) => format!("{title}\\\\[0.4em]\\large {course}"),
        (Some(title), None) => title,
        (None, Some(course)) => course,
        (None, None) => String::new(),
    };
    let optional = |value: &Option<String>| value.as_deref().map(escape_text).unwrap_or_default();
    format!(
        "\n\\title{{{title}}}\n\\author{{{}}}\n\\date{{{}}}\n",
        optional(&front_matter.author),
        optional(&front_matter.date)
    )
}

fn blocks(blocks: &[Block]) -> String {
    blocks
        .iter()
        .map(|block| self::block(&block.kind))
        .filter(|latex| !latex.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn block(kind: &BlockKind) -> String {
    match kind {
        BlockKind::Heading { level, content } => {
            // `\\` inside a sectioning command breaks the table of contents.
            let content: Vec<Inline> = content
                .iter()
                .filter(|inline| !matches!(inline, Inline::LineBreak))
                .cloned()
                .collect();
            format!(
                "\\{}{{{}}}",
                heading_command(*level),
                inlines(&content).trim()
            )
        }
        BlockKind::Paragraph(content) => paragraph(content),
        BlockKind::DisplayMath(math) => display_math(math),
        BlockKind::List { ordered, items } => list(*ordered, items),
        BlockKind::Callout { kind, title, body } => callout(*kind, title, body),
        BlockKind::Code { code, .. } => {
            // `listings` aborts on languages it does not know, so the
            // language is deliberately not passed on.
            let code = code.strip_suffix('\n').unwrap_or(code);
            format!("\\begin{{lstlisting}}\n{code}\n\\end{{lstlisting}}")
        }
        BlockKind::Quote(body) => environment("quote", "", &blocks(body)),
        BlockKind::Rule => "\\begin{center}\\rule{0.5\\linewidth}{0.4pt}\\end{center}".to_string(),
    }
}

fn heading_command(level: u8) -> &'static str {
    match level {
        0 | 1 => "section",
        2 => "subsection",
        3 => "subsubsection",
        4 => "paragraph",
        _ => "subparagraph",
    }
}

/// Leading and trailing line breaks are dropped: `\\` with no line to end
/// is a LaTeX error.
fn paragraph(content: &[Inline]) -> String {
    let is_break = |inline: &Inline| matches!(inline, Inline::LineBreak);
    let start = content.iter().position(|inline| !is_break(inline));
    let end = content.iter().rposition(|inline| !is_break(inline));
    match (start, end) {
        (Some(start), Some(end)) => inlines(&content[start..=end]).trim().to_string(),
        _ => String::new(),
    }
}

fn display_math(math: &DisplayMath) -> String {
    display_environment(&latex_rows(math), is_aligned(math))
}

/// Wraps rendered rows: `\[…\]` for one plain row, `align*` when cells are
/// aligned, `gather*` for several unaligned rows.
fn display_environment(rows: &[String], is_aligned: bool) -> String {
    match rows {
        [] => String::new(),
        [single] if !is_aligned => format!("\\[\n{single}\n\\]"),
        _ => {
            let name = if is_aligned { "align*" } else { "gather*" };
            environment(name, "", &rows.join(" \\\\\n"))
        }
    }
}

fn list(ordered: bool, items: &[Vec<Block>]) -> String {
    if items.is_empty() {
        // An environment without `\item` does not compile.
        return String::new();
    }
    let items: Vec<String> = items
        .iter()
        .map(|item| match blocks(item) {
            body if body.is_empty() => "\\item".to_string(),
            body => format!("\\item {}", protect_bracket(&body)),
        })
        .collect();
    let name = if ordered { "enumerate" } else { "itemize" };
    environment(name, "", &items.join("\n"))
}

fn callout(kind: CalloutKind, title: &[Inline], body: &[Block]) -> String {
    let title = inlines(title);
    let argument = match title.trim() {
        "" => String::new(),
        // Braces keep a `]` inside the title from ending the optional argument.
        title => format!("[{{{title}}}]"),
    };
    environment(callout_environment(kind), &argument, &blocks(body))
}

fn callout_environment(kind: CalloutKind) -> &'static str {
    match kind {
        CalloutKind::Definition => "definition",
        CalloutKind::Theorem => "theorem",
        CalloutKind::Lemma => "lemma",
        CalloutKind::Corollary => "corollary",
        CalloutKind::Proof => "proof",
        CalloutKind::Example => "example",
        CalloutKind::Note => "note",
        CalloutKind::Warning => "warning",
    }
}

fn environment(name: &str, argument: &str, body: &str) -> String {
    if body.is_empty() {
        return format!("\\begin{{{name}}}{argument}\n\\end{{{name}}}");
    }
    let body = if argument.is_empty() {
        protect_bracket(body)
    } else {
        body.to_string()
    };
    format!("\\begin{{{name}}}{argument}\n{body}\n\\end{{{name}}}")
}

/// `\item`, `\\` and theorem environments look ahead (across whitespace)
/// for an optional `[…]` argument; an empty group stops text starting with
/// `[` from being taken as one.
fn protect_bracket(text: &str) -> String {
    if text.starts_with('[') {
        format!("{{}}{text}")
    } else {
        text.to_string()
    }
}

fn inlines(content: &[Inline]) -> String {
    content.iter().map(inline).collect()
}

fn inline(inline: &Inline) -> String {
    match inline {
        Inline::Text(text) => escape_text(text),
        Inline::Strong(content) => format!("\\textbf{{{}}}", inlines(content)),
        Inline::Emphasis(content) => format!("\\emph{{{}}}", inlines(content)),
        Inline::Code(code) => format!("\\texttt{{{}}}", escape_text(code)),
        // `\(…\)` rather than `$…$`: empty math would otherwise become `$$`.
        Inline::Math(math) => format!("\\({}\\)", math_to_latex(math).trim()),
        Inline::WikiLink { target, label } => {
            format!(
                "\\emph{{{}}}",
                escape_text(&wikilink_text(target, label.as_deref()))
            )
        }
        // Unlike `\\`, `\newline` never takes a following `[…]` as its argument.
        Inline::LineBreak => "\\newline\n".to_string(),
    }
}

/// Escapes LaTeX's special characters in running text.
pub(crate) fn escape_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '\\' => escaped.push_str("\\textbackslash{}"),
            '{' | '}' | '$' | '&' | '%' | '#' | '_' => {
                escaped.push('\\');
                escaped.push(character);
            }
            '~' => escaped.push_str("\\textasciitilde{}"),
            '^' => escaped.push_str("\\textasciicircum{}"),
            '<' => escaped.push_str("\\textless{}"),
            '>' => escaped.push_str("\\textgreater{}"),
            '|' => escaped.push_str("\\textbar{}"),
            '\n' => escaped.push(' '),
            other => escaped.push(other),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::fixtures::{callout, display, kitchen_sink, paragraph as para, text};

    fn body_of(blocks: Vec<Block>) -> String {
        let latex = to_latex(&Document {
            blocks,
            ..Document::default()
        });
        let start = latex.find("\\begin{document}\n").unwrap() + "\\begin{document}\n".len();
        let end = latex.find("\\end{document}").unwrap();
        latex[start..end].trim().to_string()
    }

    #[test]
    fn kitchen_sink_has_balanced_environments_and_full_preamble() {
        let latex = to_latex(&kitchen_sink());
        for package in ["amsmath", "amssymb", "amsthm", "hyperref", "listings"] {
            assert!(
                latex.contains(&format!("\\usepackage{{{package}}}")),
                "{package}"
            );
        }
        assert_eq!(
            latex.matches("\\begin{").count(),
            latex.matches("\\end{").count()
        );
        assert!(latex.starts_with("\\documentclass"));
        assert!(latex.trim_end().ends_with("\\end{document}"));
        assert!(!latex.contains("\n\n\\]"), "blank line inside display math");
    }

    #[test]
    fn front_matter_becomes_title_block_and_pdf_metadata() {
        let latex = to_latex(&kitchen_sink());
        assert!(latex.contains(
            "\\title{Linear \"Algebra\" \\& \\#1\\\\[0.4em]\\large MATH\\_101 \\{intro\\}}"
        ));
        assert!(latex.contains("\\author{Ada Lovelace}"));
        assert!(latex.contains("\\date{2026-10-31}"));
        assert!(latex.contains("pdfkeywords={math, \\#lecture}"));
        assert!(latex.contains("\\maketitle"));
    }

    #[test]
    fn document_without_title_has_no_maketitle() {
        assert!(!to_latex(&Document::default()).contains("\\maketitle"));
    }

    #[test]
    fn display_rows_pick_the_matching_environment() {
        let rows = |rows: &[&str]| rows.iter().map(|row| row.to_string()).collect::<Vec<_>>();
        assert_eq!(display_environment(&rows(&["x"]), false), "\\[\nx\n\\]");
        assert_eq!(
            display_environment(&rows(&["a & = b", "& = c"]), true),
            "\\begin{align*}\na & = b \\\\\n& = c\n\\end{align*}"
        );
        assert_eq!(
            display_environment(&rows(&["u", "v"]), false),
            "\\begin{gather*}\nu \\\\\nv\n\\end{gather*}"
        );
        assert_eq!(display_environment(&[], true), "");
    }

    #[test]
    fn empty_display_math_is_omitted() {
        assert_eq!(
            body_of(vec![display(vec![]), display(vec![vec![vec![]]])]),
            ""
        );
    }

    #[test]
    fn callouts_map_to_amsthm_environments() {
        let body = body_of(vec![
            callout(
                CalloutKind::Theorem,
                "Pythagoras",
                vec![para(vec![text("[a] holds")])],
            ),
            callout(CalloutKind::Proof, "", vec![para(vec![text("[trivial]")])]),
        ]);
        assert_eq!(
            body,
            "\\begin{theorem}[{Pythagoras}]\n[a] holds\n\\end{theorem}\n\n\\begin{proof}\n{}[trivial]\n\\end{proof}"
        );
    }

    #[test]
    fn lists_code_quotes_and_rules() {
        let body = body_of(vec![
            Block {
                kind: BlockKind::List {
                    ordered: true,
                    items: vec![vec![para(vec![text("[1] one")])], vec![]],
                },
                line: 0,
            },
            Block {
                kind: BlockKind::List {
                    ordered: false,
                    items: vec![],
                },
                line: 0,
            },
            Block {
                kind: BlockKind::Code {
                    language: Some("python".into()),
                    code: "print('%$')\n".into(),
                },
                line: 0,
            },
            Block {
                kind: BlockKind::Quote(vec![para(vec![text("q")])]),
                line: 0,
            },
        ]);
        assert_eq!(
            body,
            "\\begin{enumerate}\n\\item {}[1] one\n\\item\n\\end{enumerate}\n\n\\begin{lstlisting}\nprint('%$')\n\\end{lstlisting}\n\n\\begin{quote}\nq\n\\end{quote}"
        );
    }

    #[test]
    fn inline_formatting_and_wikilinks() {
        let body = body_of(vec![para(vec![
            Inline::Strong(vec![text("b")]),
            Inline::Emphasis(vec![text("e")]),
            Inline::Code("a_b".into()),
            Inline::WikiLink {
                target: "notes/Topology.qm#Open".into(),
                label: None,
            },
            Inline::Math(vec![]),
        ])]);
        assert_eq!(
            body,
            "\\textbf{b}\\emph{e}\\texttt{a\\_b}\\emph{notes/Topology \\textgreater{} Open}\\(\\)"
        );
    }

    #[test]
    fn escape_text_escapes_every_special_character() {
        assert_eq!(
            escape_text(r"\ { } $ & % # _ ~ ^ < > |"),
            r"\textbackslash{} \{ \} \$ \& \% \# \_ \textasciitilde{} \textasciicircum{} \textless{} \textgreater{} \textbar{}"
        );
    }

    #[test]
    fn paragraph_drops_leading_and_trailing_line_breaks() {
        let content = [
            Inline::LineBreak,
            Inline::Text("a".into()),
            Inline::LineBreak,
            Inline::Text("b".into()),
            Inline::LineBreak,
        ];
        assert_eq!(paragraph(&content), "a\\newline\nb");
        assert_eq!(paragraph(&[Inline::LineBreak]), "");
    }
}
