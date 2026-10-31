//! Converts math syntax trees to LaTeX (amsmath + amssymb).

use crate::ast::{AccentKind, DisplayMath, MathList, MathNode, SpaceWidth};

/// Converts math to LaTeX math-mode source (without surrounding delimiters).
pub(crate) fn math_to_latex(math: &[MathNode]) -> String {
    let mut writer = LatexWriter::default();
    writer.write_list(math);
    writer.output
}

/// Converts display math to LaTeX math-mode source. Several rows (or any
/// alignment) become an `aligned` environment; wrap the result in `\[ \]`.
pub(crate) fn display_math_to_latex(display: &DisplayMath) -> String {
    let mut writer = LatexWriter::default();
    match display.rows.as_slice() {
        [] => {}
        [row] => row.cells.iter().for_each(|cell| writer.write_list(cell)),
        rows => {
            writer.push("\\begin{aligned}");
            for (index, row) in rows.iter().enumerate() {
                if index > 0 {
                    writer.push(" \\\\ ");
                }
                writer.write_cells(&row.cells);
            }
            writer.push("\\end{aligned}");
        }
    }
    writer.output
}

/// Escapes prose for use inside `\text{…}`.
pub(crate) fn escape_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => escaped.push_str("\\textbackslash{}"),
            '{' | '}' | '$' | '&' | '#' | '_' | '%' => {
                escaped.push('\\');
                escaped.push(c);
            }
            '^' => escaped.push_str("\\textasciicircum{}"),
            '~' => escaped.push_str("\\textasciitilde{}"),
            _ => escaped.push(c),
        }
    }
    escaped
}

#[derive(Default)]
struct LatexWriter {
    output: String,
}

impl LatexWriter {
    /// Appends `piece`, separating it from a preceding control word such as
    /// `\alpha` so that `\alpha x` does not become `\alphax`.
    fn push(&mut self, piece: &str) {
        if piece.starts_with(|c: char| c.is_ascii_alphabetic())
            && ends_with_control_word(&self.output)
        {
            self.output.push(' ');
        }
        self.output.push_str(piece);
    }

    fn write_list(&mut self, list: &[MathNode]) {
        for node in list {
            self.write_node(node);
        }
    }

    fn write_braced(&mut self, list: &[MathNode]) {
        self.output.push('{');
        self.write_list(list);
        self.output.push('}');
    }

    fn write_cells(&mut self, cells: &[MathList]) {
        for (index, cell) in cells.iter().enumerate() {
            if index > 0 {
                self.push(" & ");
            }
            self.write_list(cell);
        }
    }

    fn write_node(&mut self, node: &MathNode) {
        match node {
            MathNode::Atom(atom) => self.push(&atom.latex),
            MathNode::Text(text) => self.write_text(text),
            MathNode::Fraction {
                numerator,
                denominator,
            } => {
                self.push("\\frac");
                self.write_braced(numerator);
                self.write_braced(denominator);
            }
            MathNode::Scripts { base, sub, sup } => {
                self.write_scripts(base, sub.as_deref(), sup.as_deref())
            }
            MathNode::Root { index, radicand } => self.write_root(index.as_deref(), radicand),
            MathNode::Fenced { open, close, body } => self.write_fenced(open, close, body),
            MathNode::Matrix { open, close, rows } => self.write_matrix(open, close, rows),
            MathNode::Accent { accent, body } => self.write_accent(*accent, body),
            MathNode::Group(list) => self.write_braced(list),
            MathNode::Space(width) => self.push(space_latex(*width)),
            MathNode::Error { source, .. } => self.write_text(source),
        }
    }

    fn write_text(&mut self, text: &str) {
        self.push("\\text{");
        self.output.push_str(&escape_text(text));
        self.output.push('}');
    }

    fn write_scripts(
        &mut self,
        base: &MathNode,
        sub: Option<&[MathNode]>,
        sup: Option<&[MathNode]>,
    ) {
        let needs_braces = matches!(
            base,
            MathNode::Scripts { .. }
                | MathNode::Fraction { .. }
                | MathNode::Space(_)
                | MathNode::Matrix { .. }
        );
        if needs_braces {
            self.write_braced(std::slice::from_ref(base));
        } else {
            self.write_node(base);
        }
        if let Some(sub) = sub {
            self.output.push('_');
            self.write_braced(sub);
        }
        if let Some(sup) = sup {
            self.output.push('^');
            self.write_braced(sup);
        }
    }

    fn write_root(&mut self, index: Option<&[MathNode]>, radicand: &[MathNode]) {
        self.push("\\sqrt");
        if let Some(index) = index.filter(|index| !index.is_empty()) {
            self.output.push('[');
            self.write_list(index);
            self.output.push(']');
        }
        self.write_braced(radicand);
    }

    fn write_fenced(&mut self, open: &str, close: &str, body: &[MathNode]) {
        self.push("\\left");
        self.push(delimiter_latex(open));
        self.write_list(body);
        self.push("\\right");
        self.push(delimiter_latex(close));
    }

    fn write_matrix(&mut self, open: &str, close: &str, rows: &[Vec<MathList>]) {
        let environment = matrix_environment(open, close);
        if environment.is_none() {
            self.push("\\left");
            self.push(delimiter_latex(open));
        }
        let name = environment.unwrap_or("matrix");
        self.push(&format!("\\begin{{{name}}}"));
        for (index, row) in rows.iter().enumerate() {
            if index > 0 {
                self.push(" \\\\ ");
            }
            self.write_cells(row);
        }
        self.push(&format!("\\end{{{name}}}"));
        if environment.is_none() {
            self.push("\\right");
            self.push(delimiter_latex(close));
        }
    }

    fn write_accent(&mut self, accent: AccentKind, body: &[MathNode]) {
        let is_narrow = matches!(body, [MathNode::Atom(_)]);
        let command = match (accent, is_narrow) {
            (AccentKind::Hat, true) => "\\hat",
            (AccentKind::Hat, false) => "\\widehat",
            (AccentKind::Bar, true) => "\\bar",
            (AccentKind::Bar, false) => "\\overline",
            (AccentKind::Vector, true) => "\\vec",
            (AccentKind::Vector, false) => "\\overrightarrow",
            (AccentKind::Tilde, true) => "\\tilde",
            (AccentKind::Tilde, false) => "\\widetilde",
            (AccentKind::Dot, _) => "\\dot",
            (AccentKind::DoubleDot, _) => "\\ddot",
        };
        self.push(command);
        self.write_braced(body);
    }
}

fn ends_with_control_word(output: &str) -> bool {
    let letters = output
        .bytes()
        .rev()
        .take_while(u8::is_ascii_alphabetic)
        .count();
    let Some(backslash_index) = output.len().checked_sub(letters + 1) else {
        return false;
    };
    letters > 0 && output.as_bytes()[backslash_index] == b'\\'
}

fn space_latex(width: SpaceWidth) -> &'static str {
    match width {
        SpaceWidth::Thin => "\\,",
        SpaceWidth::Medium => "\\:",
        SpaceWidth::Thick => "\\;",
        SpaceWidth::Quad => "\\quad ",
    }
}

/// LaTeX for a stretchy delimiter given as Unicode; empty means none.
fn delimiter_latex(delimiter: &str) -> &'static str {
    match delimiter {
        "(" => "(",
        ")" => ")",
        "[" => "[",
        "]" => "]",
        "{" => "\\{",
        "}" => "\\}",
        "|" => "|",
        "‖" => "\\|",
        "⌊" => "\\lfloor",
        "⌋" => "\\rfloor",
        "⌈" => "\\lceil",
        "⌉" => "\\rceil",
        "⟨" => "\\langle",
        "⟩" => "\\rangle",
        _ => ".",
    }
}

fn matrix_environment(open: &str, close: &str) -> Option<&'static str> {
    match (open, close) {
        ("(", ")") => Some("pmatrix"),
        ("[", "]") => Some("bmatrix"),
        ("{", "}") => Some("Bmatrix"),
        ("|", "|") => Some("vmatrix"),
        ("‖", "‖") => Some("Vmatrix"),
        ("", "") => Some("matrix"),
        ("{", "") => Some("cases"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_word_detection() {
        assert!(ends_with_control_word("\\alpha"));
        assert!(!ends_with_control_word("x"));
        assert!(!ends_with_control_word("\\{"));
        assert!(!ends_with_control_word(""));
    }

    #[test]
    fn escapes_text_specials() {
        assert_eq!(escape_text("50% & $5_x"), "50\\% \\& \\$5\\_x");
        assert_eq!(escape_text("a\\b"), "a\\textbackslash{}b");
    }
}
