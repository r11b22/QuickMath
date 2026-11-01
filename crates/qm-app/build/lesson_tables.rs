//! Turns `src/practice/lessons.toml` into the `LEVELS` table that
//! `src/practice/curriculum.rs` includes. See the header of that file for its
//! format.
//!
//! Shared by `build.rs` and `tests/lesson_file.rs`, so it only uses `toml`
//! and the standard library.

use std::collections::HashSet;

use toml::{Table, Value};

const LEVEL_KEYS: [&str; 5] = ["id", "title", "lesson", "tips", "answers"];

/// One `[[level]]` entry, already checked.
struct Level<'a> {
    id: &'a str,
    title: &'a str,
    lesson: &'a str,
    tips: Vec<&'a str>,
    answers: Vec<&'a str>,
}

/// Generates the Rust table, or describes the first problem in `source`.
pub fn generate(source: &str) -> Result<String, String> {
    let file: Table = source.parse().map_err(|error| format!("{error}"))?;
    if let Some(key) = file.keys().find(|key| *key != "level") {
        return Err(format!(
            "unknown top-level key `{key}`; levels go in [[level]] entries"
        ));
    }
    let entries = match file.get("level") {
        Some(Value::Array(entries)) if !entries.is_empty() => entries,
        _ => return Err("expected at least one [[level]] entry".to_string()),
    };
    let mut ids = HashSet::new();
    let mut levels = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let level =
            parse_level(entry).map_err(|problem| format!("level {}: {problem}", index + 1))?;
        if !ids.insert(level.id) {
            return Err(format!("level {}: duplicate id `{}`", index + 1, level.id));
        }
        levels.push(level);
    }
    Ok(to_rust(&levels))
}

fn parse_level(entry: &Value) -> Result<Level<'_>, String> {
    let table = entry
        .as_table()
        .ok_or("must be a [[level]] table of keys")?;
    let id = text(table, "id")?;
    check_id(id)?;
    let in_level = |problem: String| format!("`{id}`: {problem}");
    if let Some(key) = table.keys().find(|key| !LEVEL_KEYS.contains(&key.as_str())) {
        return Err(in_level(format!(
            "unknown key `{key}`; expected one of {}",
            LEVEL_KEYS.join(", ")
        )));
    }
    let answers = text_list(table, "answers").map_err(in_level)?;
    if answers.is_empty() {
        return Err(in_level("`answers` must not be empty".to_string()));
    }
    Ok(Level {
        id,
        title: text(table, "title").map_err(in_level)?,
        lesson: text(table, "lesson").map_err(in_level)?,
        tips: match table.get("tips") {
            Some(_) => text_list(table, "tips").map_err(in_level)?,
            None => Vec::new(),
        },
        answers,
    })
}

fn text<'a>(table: &'a Table, key: &str) -> Result<&'a str, String> {
    match table.get(key) {
        Some(Value::String(text)) if !text.trim().is_empty() => Ok(text),
        Some(_) => Err(format!("`{key}` must be a non-empty string")),
        None => Err(format!("missing `{key}`")),
    }
}

fn text_list<'a>(table: &'a Table, key: &str) -> Result<Vec<&'a str>, String> {
    let expected = || format!("`{key}` must be a list of non-empty strings");
    let items = table
        .get(key)
        .ok_or_else(|| format!("missing `{key}`"))?
        .as_array()
        .ok_or_else(expected)?;
    items
        .iter()
        .map(|item| match item.as_str() {
            Some(text) if !text.trim().is_empty() => Ok(text),
            _ => Err(expected()),
        })
        .collect()
}

/// The id names the level's generator function, so it must be a Rust name.
fn check_id(id: &str) -> Result<(), String> {
    let mut chars = id.chars();
    let starts_with_letter = chars.next().is_some_and(|first| first.is_ascii_lowercase());
    let is_name = chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if starts_with_letter && is_name {
        return Ok(());
    }
    Err(format!(
        "id `{id}` must be lowercase letters, digits and `_`, starting with a letter"
    ))
}

fn to_rust(levels: &[Level]) -> String {
    let entries: String = levels
        .iter()
        .map(|level| {
            format!(
                "    Level {{\n        id: {:?},\n        title: {:?},\n        lesson: {:?},\n        \
                 tips: &{:?},\n        answers: &{:?},\n        generate: generators::{},\n    }},\n",
                level.id, level.title, level.lesson, level.tips, level.answers, level.id
            )
        })
        .collect();
    format!("pub const LEVELS: &[Level] = &[\n{entries}];\n")
}
