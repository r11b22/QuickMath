//! The optional `---` metadata header at the top of a note.

use crate::ast::FrontMatter;
use crate::lines::{is_blank, Line};

const FENCE: &str = "---";

/// Number of lines the front matter occupies (including both fences), or
/// `None` when the note has none. A first-line `---` only starts front
/// matter if it is closed and every line in between looks like metadata;
/// otherwise it is an ordinary horizontal rule.
pub(crate) fn front_matter_len(lines: &[Line]) -> Option<usize> {
    if lines.first()?.text.trim_end() != FENCE {
        return None;
    }
    let close = lines
        .iter()
        .skip(1)
        .position(|line| line.text.trim_end() == FENCE)?
        + 1;
    lines[1..close]
        .iter()
        .all(|line| is_metadata_line(line.text))
        .then_some(close + 1)
}

fn is_metadata_line(text: &str) -> bool {
    let trimmed = text.trim();
    is_blank(text)
        || trimmed.starts_with("- ")
        || trimmed.starts_with('#')
        || key_value(text).is_some()
}

fn key_value(text: &str) -> Option<(&str, &str)> {
    let (key, value) = text.split_once(':')?;
    let key = key.trim();
    let is_key = !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-');
    is_key.then(|| (key, value.trim()))
}

/// Whether line `index` of the front matter is a key still waiting for a
/// value, such as `author:`. A `tags:` followed by `- item` lines is filled.
pub(crate) fn is_empty_field(lines: &[Line], index: usize) -> bool {
    let Some((_, value)) = lines.get(index).and_then(|line| key_value(line.text)) else {
        return false;
    };
    let has_list_items = lines
        .get(index + 1)
        .is_some_and(|next| next.text.trim().starts_with("- "));
    value.is_empty() && !has_list_items
}

/// The text a new note starts with: front matter with every recognised key,
/// `title` and `date` filled in and the others left empty to fill in later.
pub(crate) fn new_note_template(title: &str, date: &str) -> String {
    // A line break would end the field and break the header.
    let title = title.replace(['\r', '\n'], " ");
    format!("{FENCE}\ntitle: {title}\nauthor:\ndate: {date}\ncourse:\ntags:\n{FENCE}\n")
}

/// Parses front matter; returns it with the number of lines it used.
pub(crate) fn parse_front_matter(lines: &[Line]) -> (FrontMatter, usize) {
    let Some(length) = front_matter_len(lines) else {
        return (FrontMatter::default(), 0);
    };
    let mut front_matter = FrontMatter::default();
    let mut is_in_tag_list = false;
    for line in &lines[1..length - 1] {
        let trimmed = line.text.trim();
        if let Some(item) = trimmed.strip_prefix("- ").filter(|_| is_in_tag_list) {
            push_tags(&mut front_matter.tags, item);
            continue;
        }
        let Some((key, value)) = key_value(line.text) else {
            continue;
        };
        is_in_tag_list =
            matches!(key.to_ascii_lowercase().as_str(), "tags" | "tag") && value.is_empty();
        apply_field(&mut front_matter, key, value);
    }
    (front_matter, length)
}

fn apply_field(front_matter: &mut FrontMatter, key: &str, value: &str) {
    let text = Some(unquote(value).to_string()).filter(|v| !v.is_empty());
    match key.to_ascii_lowercase().as_str() {
        "title" => front_matter.title = text,
        "author" => front_matter.author = text,
        "date" => front_matter.date = text,
        "course" => front_matter.course = text,
        "tags" | "tag" => push_tags(&mut front_matter.tags, value),
        _ => {}
    }
}

/// Accepts `[a, b]`, `a, b` and `#a #b`.
fn push_tags(tags: &mut Vec<String>, value: &str) {
    let list = value.trim().trim_start_matches('[').trim_end_matches(']');
    let separator = if list.contains(',') { ',' } else { ' ' };
    let new_tags = list
        .split(separator)
        .map(|tag| unquote(tag.trim()).trim_start_matches('#').trim())
        .filter(|tag| !tag.is_empty())
        .map(str::to_string);
    tags.extend(new_tags);
}

fn unquote(value: &str) -> &str {
    let value = value.trim();
    for quote in ['"', '\''] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|v| v.strip_suffix(quote))
        {
            return inner;
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lines::split_lines;

    fn parse(source: &str) -> (FrontMatter, usize) {
        parse_front_matter(&split_lines(source))
    }

    #[test]
    fn parses_all_fields() {
        let (front_matter, length) =
            parse("---\ntitle: \"Groups\"\nauthor: Ada\ndate: 2026-10-31\ncourse: Algebra\ntags: [algebra, \"groups\"]\n---\nbody");
        assert_eq!(length, 7);
        assert_eq!(front_matter.title.as_deref(), Some("Groups"));
        assert_eq!(front_matter.author.as_deref(), Some("Ada"));
        assert_eq!(front_matter.date.as_deref(), Some("2026-10-31"));
        assert_eq!(front_matter.course.as_deref(), Some("Algebra"));
        assert_eq!(front_matter.tags, vec!["algebra", "groups"]);
    }

    #[test]
    fn accepts_comma_space_and_yaml_list_tags() {
        assert_eq!(parse("---\ntags: a, b\n---").0.tags, vec!["a", "b"]);
        assert_eq!(parse("---\ntags: #a #b\n---").0.tags, vec!["a", "b"]);
        assert_eq!(
            parse("---\ntags:\n  - a\n  - b\n---").0.tags,
            vec!["a", "b"]
        );
    }

    #[test]
    fn new_note_template_is_front_matter_with_title_and_date() {
        let template = new_note_template("Lecture 7: Groups", "2026-11-01");
        let (front_matter, length) = parse(&template);
        assert_eq!(length, 7);
        assert_eq!(front_matter.title.as_deref(), Some("Lecture 7: Groups"));
        assert_eq!(front_matter.date.as_deref(), Some("2026-11-01"));
        assert_eq!(front_matter.author, None);
        assert!(front_matter.tags.is_empty());
    }

    #[test]
    fn new_note_template_keeps_title_on_one_line() {
        let template = new_note_template("a\nb", "2026-11-01");
        assert_eq!(parse(&template).0.title.as_deref(), Some("a b"));
    }

    #[test]
    fn empty_fields_are_keys_without_value_or_list() {
        let lines = split_lines("---\ntitle: T\nauthor:\ntags:\n  - a\ncourse: \n---");
        let empty: Vec<usize> = (0..lines.len())
            .filter(|&index| is_empty_field(&lines, index))
            .collect();
        assert_eq!(empty, vec![2, 5]);
    }

    #[test]
    fn unclosed_fence_is_not_front_matter() {
        assert_eq!(parse("---\ntitle: x").1, 0);
    }

    #[test]
    fn rules_around_prose_are_not_front_matter() {
        assert_eq!(parse("---\nSome prose here.\n---").1, 0);
    }
}
