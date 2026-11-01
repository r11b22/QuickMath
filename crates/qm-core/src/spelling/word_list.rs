//! The user's own words, accepted by spell checking in every language and
//! stored one per line as `quickmath/words.txt` next to the app settings.

use std::fs;
use std::path::{Path, PathBuf};

use crate::fs_util::{create_dir_all, write_atomically};
use crate::settings::SETTINGS_DIR_NAME;
use crate::{CoreError, Result};

const WORD_LIST_FILE_NAME: &str = "words.txt";

/// Words added with "Add to dictionary", in the order they were added.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WordList {
    /// Where [`WordList::add`] saves; `None` if the OS has no config directory.
    path: Option<PathBuf>,
    words: Vec<String>,
}

impl WordList {
    /// Loads the list from [`WordList::default_path`]; empty if there is none yet.
    pub fn load() -> WordList {
        match Self::default_path() {
            Some(path) => Self::load_from(&path),
            None => WordList::default(),
        }
    }

    /// Where the list is kept, or `None` if the OS has no config directory.
    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join(SETTINGS_DIR_NAME).join(WORD_LIST_FILE_NAME))
    }

    /// Like [`WordList::load`], but from an explicit file, which
    /// [`WordList::add`] then saves to. Never fails: a missing or unreadable
    /// file gives an empty list. Blank lines and surrounding spaces are
    /// ignored, so the file can be edited by hand.
    pub fn load_from(path: &Path) -> WordList {
        // Lossy, so one badly encoded hand-edited line does not hide (and,
        // on the next save, delete) every other word.
        let text = fs::read(path)
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default();
        let mut words: Vec<String> = Vec::new();
        for word in text.lines().map(str::trim).filter(|word| !word.is_empty()) {
            if !words.iter().any(|known| known == word) {
                words.push(word.to_string());
            }
        }
        WordList {
            path: Some(path.to_path_buf()),
            words,
        }
    }

    pub fn words(&self) -> &[String] {
        &self.words
    }

    /// Adds `word` (trimmed) and saves the list. Adding a word that is
    /// already there, or an empty one, changes nothing.
    pub fn add(&mut self, word: &str) -> Result<()> {
        let word = word.trim();
        if word.is_empty() || self.words.iter().any(|known| known == word) {
            return Ok(());
        }
        let path = self.path.clone().ok_or_else(|| {
            let error = std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "the OS does not provide a config directory",
            );
            CoreError::io(Path::new(WORD_LIST_FILE_NAME), error)
        })?;
        self.words.push(word.to_string());
        if let Err(error) = self.save_to(&path) {
            self.words.pop();
            return Err(error);
        }
        Ok(())
    }

    fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(folder) = path.parent() {
            create_dir_all(folder)?;
        }
        let mut text = self.words.join("\n");
        text.push('\n');
        write_atomically(path, text.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn added_words_survive_a_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join(WORD_LIST_FILE_NAME);
        let mut list = WordList::load_from(&path);

        list.add("eigenbasis").unwrap();
        list.add(" Banach ").unwrap();

        assert_eq!(WordList::load_from(&path).words(), ["eigenbasis", "Banach"]);
    }

    #[test]
    fn adding_a_known_or_empty_word_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(WORD_LIST_FILE_NAME);
        let mut list = WordList::load_from(&path);
        list.add("eigenbasis").unwrap();

        list.add("eigenbasis").unwrap();
        list.add("  ").unwrap();

        assert_eq!(list.words(), ["eigenbasis"]);
    }

    #[test]
    fn hand_edited_files_are_cleaned_up() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(WORD_LIST_FILE_NAME);
        fs::write(&path, "  alpha \n\nbeta\r\nalpha\n").unwrap();

        assert_eq!(WordList::load_from(&path).words(), ["alpha", "beta"]);
    }

    #[test]
    fn missing_file_gives_an_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        let list = WordList::load_from(&dir.path().join("missing.txt"));
        assert!(list.words().is_empty());
    }

    #[test]
    fn failed_save_keeps_the_list_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        // A file where the folder should be makes saving fail.
        let blocker = dir.path().join("blocker");
        fs::write(&blocker, "").unwrap();
        let mut list = WordList::load_from(&blocker.join(WORD_LIST_FILE_NAME));

        assert!(list.add("eigenbasis").is_err());
        assert!(list.words().is_empty());
    }
}
