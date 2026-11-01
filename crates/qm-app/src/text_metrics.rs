//! Pure text measurements used by the editor and status bar.
//!
//! All offsets here are *character* offsets (not bytes), because that is what
//! egui's text cursor uses.

/// A zero-based line/column position in a text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextPosition {
    pub line: usize,
    pub column: usize,
}

/// Returns the line and column of the character offset `char_index`.
/// Offsets past the end are clamped to the end of the text.
pub fn position_of_char(text: &str, char_index: usize) -> TextPosition {
    let mut position = TextPosition::default();
    for ch in text.chars().take(char_index) {
        if ch == '\n' {
            position.line += 1;
            position.column = 0;
        } else {
            position.column += 1;
        }
    }
    position
}

/// Returns the character offset at which zero-based `line` starts.
/// Lines past the end map to the end of the text.
pub fn char_offset_of_line(text: &str, line: usize) -> usize {
    if line == 0 {
        return 0;
    }
    let mut lines_seen = 0;
    for (char_index, ch) in text.chars().enumerate() {
        if ch == '\n' {
            lines_seen += 1;
            if lines_seen == line {
                return char_index + 1;
            }
        }
    }
    text.chars().count()
}

/// Counts whitespace-separated words.
pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Replaces the characters in `char_range` with `insertion` and returns the
/// character offset just after the inserted text. The range is clamped to
/// the text, so a stale cursor can never cause a panic.
pub fn replace_char_range(
    text: &mut String,
    char_range: std::ops::Range<usize>,
    insertion: &str,
) -> usize {
    let start_byte = byte_offset_of_char(text, char_range.start);
    let end_byte = byte_offset_of_char(text, char_range.end.max(char_range.start));
    text.replace_range(start_byte..end_byte, insertion);
    text[..start_byte].chars().count() + insertion.chars().count()
}

/// Removes up to `width` leading spaces from the line containing the
/// character offset `cursor`, returning the adjusted cursor offset.
pub fn outdent_line(text: &mut String, cursor: usize, width: usize) -> usize {
    let line_start = char_offset_of_line(text, position_of_char(text, cursor).line);
    let removable = text
        .chars()
        .skip(line_start)
        .take(width)
        .take_while(|ch| *ch == ' ')
        .count();
    replace_char_range(text, line_start..line_start + removable, "");
    cursor.saturating_sub(removable).max(line_start)
}

/// Returns the byte offset of the character offset `char_index`, clamped to
/// the end of the text.
pub fn byte_offset_of_char(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map_or(text.len(), |(byte, _)| byte)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_of_char_counts_lines_and_columns() {
        let text = "ab\ncde\n";
        assert_eq!(
            position_of_char(text, 0),
            TextPosition { line: 0, column: 0 }
        );
        assert_eq!(
            position_of_char(text, 2),
            TextPosition { line: 0, column: 2 }
        );
        assert_eq!(
            position_of_char(text, 3),
            TextPosition { line: 1, column: 0 }
        );
        assert_eq!(
            position_of_char(text, 7),
            TextPosition { line: 2, column: 0 }
        );
    }

    #[test]
    fn position_of_char_clamps_past_end_and_handles_unicode() {
        assert_eq!(
            position_of_char("αβ", 99),
            TextPosition { line: 0, column: 2 }
        );
        assert_eq!(position_of_char("", 5), TextPosition::default());
    }

    #[test]
    fn char_offset_of_line_finds_line_starts() {
        let text = "αβ\nγ\n\nlast";
        assert_eq!(char_offset_of_line(text, 0), 0);
        assert_eq!(char_offset_of_line(text, 1), 3);
        assert_eq!(char_offset_of_line(text, 2), 5);
        assert_eq!(char_offset_of_line(text, 3), 6);
    }

    #[test]
    fn char_offset_of_line_clamps_past_end() {
        assert_eq!(char_offset_of_line("one\ntwo", 10), 7);
        assert_eq!(char_offset_of_line("", 3), 0);
    }

    #[test]
    fn char_offset_round_trips_with_position() {
        let text = "x = 1\n$sum_(i=1)^n i$\n## Heading";
        for line in 0..3 {
            let offset = char_offset_of_line(text, line);
            assert_eq!(
                position_of_char(text, offset),
                TextPosition { line, column: 0 }
            );
        }
    }

    #[test]
    fn word_count_splits_on_any_whitespace() {
        assert_eq!(word_count(""), 0);
        assert_eq!(word_count("  \n\t "), 0);
        assert_eq!(word_count("Let $x in RR$\nbe real."), 6);
    }

    #[test]
    fn replace_char_range_inserts_at_cursor() {
        let mut text = String::from("aβc");
        let cursor = replace_char_range(&mut text, 1..1, "  ");
        assert_eq!(text, "a  βc");
        assert_eq!(cursor, 3);
    }

    #[test]
    fn replace_char_range_replaces_selection() {
        let mut text = String::from("hello world");
        let cursor = replace_char_range(&mut text, 6..11, "αβ");
        assert_eq!(text, "hello αβ");
        assert_eq!(cursor, 8);
    }

    #[test]
    fn outdent_line_removes_up_to_width_spaces() {
        let mut text = String::from("a\n   b");
        let cursor = outdent_line(&mut text, 6, 2);
        assert_eq!(text, "a\n b");
        assert_eq!(cursor, 4);
    }

    #[test]
    fn outdent_line_keeps_cursor_inside_the_line() {
        let mut text = String::from("  x");
        assert_eq!(outdent_line(&mut text, 1, 2), 0);
        assert_eq!(text, "x");
        let mut unindented = String::from("x");
        assert_eq!(outdent_line(&mut unindented, 1, 2), 1);
        assert_eq!(unindented, "x");
    }

    #[test]
    fn replace_char_range_clamps_stale_ranges() {
        let mut text = String::from("ab");
        let cursor = replace_char_range(&mut text, 5..9, "!");
        assert_eq!(text, "ab!");
        assert_eq!(cursor, 3);
    }
}
