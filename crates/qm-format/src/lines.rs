//! Source lines with their numbers, and indentation helpers.

/// Columns a tab counts for when measuring indentation.
const TAB_WIDTH: usize = 4;

/// One source line (without its line terminator) and its zero-based number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Line<'a> {
    pub text: &'a str,
    pub number: usize,
}

/// Splits `source` into lines, accepting `\n` and `\r\n` endings.
pub(crate) fn split_lines(source: &str) -> Vec<Line<'_>> {
    source
        .split('\n')
        .enumerate()
        .map(|(number, text)| Line {
            text: text.strip_suffix('\r').unwrap_or(text),
            number,
        })
        .collect()
}

pub(crate) fn is_blank(text: &str) -> bool {
    text.trim().is_empty()
}

/// Width of the leading whitespace in columns (tabs count as four).
pub(crate) fn indent_width(text: &str) -> usize {
    text.chars()
        .take_while(|c| c.is_whitespace())
        .map(|c| if c == '\t' { TAB_WIDTH } else { 1 })
        .sum()
}

/// Removes up to `columns` columns of leading whitespace.
pub(crate) fn strip_indent(text: &str, columns: usize) -> &str {
    let mut removed = 0;
    for (index, c) in text.char_indices() {
        if removed >= columns || !c.is_whitespace() {
            return &text[index..];
        }
        removed += if c == '\t' { TAB_WIDTH } else { 1 };
    }
    ""
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_crlf_and_keeps_numbers() {
        let lines = split_lines("a\r\nb\n");
        assert_eq!(
            lines,
            vec![
                Line {
                    text: "a",
                    number: 0
                },
                Line {
                    text: "b",
                    number: 1
                },
                Line {
                    text: "",
                    number: 2
                }
            ]
        );
    }

    #[test]
    fn measures_tabs_as_four_columns() {
        assert_eq!(indent_width("\t  x"), 6);
    }

    #[test]
    fn strips_at_most_the_requested_columns() {
        assert_eq!(strip_indent("    x", 2), "  x");
        assert_eq!(strip_indent(" x", 4), "x");
        assert_eq!(strip_indent("\tx", 2), "x");
        assert_eq!(strip_indent("   ", 2), " ");
    }
}
