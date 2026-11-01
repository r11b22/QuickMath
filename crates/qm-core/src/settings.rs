//! App-wide settings, stored as `quickmath/settings.json` in the OS config
//! directory (e.g. `~/.config/quickmath/settings.json` on Linux).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::fs_util::{create_dir_all, write_atomically};
use crate::{CoreError, Result};

/// How many recently opened projects are remembered.
pub const MAX_RECENT_PROJECTS: usize = 10;

/// Folder in the OS config directory that holds all app-wide files.
pub(crate) const SETTINGS_DIR_NAME: &str = "quickmath";
const SETTINGS_FILE_NAME: &str = "settings.json";
/// Spell checking language for new installs, by [`crate::Language`] code.
const DEFAULT_SPELLING_LANGUAGE: &str = "en_US";

/// App-wide settings stored in the user's config directory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    /// Most recent first, without duplicates.
    pub recent_projects: Vec<PathBuf>,
    pub dark_mode: bool,
    /// Project that was open when the app quit, reopened on the next start.
    /// `None` after the user explicitly closes the project.
    pub open_project: Option<PathBuf>,
    /// [`crate::Language`] codes spell checking uses, all at once. Empty
    /// turns spell checking off.
    pub spelling_languages: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> AppSettings {
        AppSettings {
            recent_projects: Vec::new(),
            dark_mode: false,
            open_project: None,
            spelling_languages: vec![DEFAULT_SPELLING_LANGUAGE.to_string()],
        }
    }
}

impl AppSettings {
    /// Loads settings, falling back to defaults when the file is missing or unreadable.
    pub fn load() -> AppSettings {
        Self::default_path()
            .map(|path| Self::load_from(&path))
            .unwrap_or_default()
    }

    /// Saves to [`AppSettings::default_path`], creating the folder if needed.
    pub fn save(&self) -> Result<()> {
        let path = Self::default_path().ok_or_else(|| {
            let error = std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "the OS does not provide a config directory",
            );
            CoreError::io(Path::new(SETTINGS_FILE_NAME), error)
        })?;
        self.save_to(&path)
    }

    /// Moves `root` to the front of the recent-projects list.
    pub fn remember_project(&mut self, root: &Path) {
        self.recent_projects.retain(|path| path != root);
        self.recent_projects.insert(0, root.to_path_buf());
        self.recent_projects.truncate(MAX_RECENT_PROJECTS);
    }

    /// Turns spell checking in the language with `code` on, or off if it was on.
    pub fn toggle_spelling_language(&mut self, code: &str) {
        if let Some(index) = self
            .spelling_languages
            .iter()
            .position(|known| known == code)
        {
            self.spelling_languages.remove(index);
        } else {
            self.spelling_languages.push(code.to_string());
        }
    }

    /// Where [`AppSettings::load`] and [`AppSettings::save`] keep the
    /// settings, or `None` if the OS has no config directory.
    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join(SETTINGS_DIR_NAME).join(SETTINGS_FILE_NAME))
    }

    /// Like [`AppSettings::load`], but from an explicit file. Never fails.
    /// A hand-edited list is cleaned up (deduplicated and capped).
    pub fn load_from(path: &Path) -> AppSettings {
        let mut settings: AppSettings = fs::read_to_string(path)
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();
        settings.normalize_recent_projects();
        settings
    }

    /// Like [`AppSettings::save`], but to an explicit file (written atomically).
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(folder) = path.parent() {
            create_dir_all(folder)?;
        }
        let json = serde_json::to_string_pretty(self).expect("settings always serialize");
        write_atomically(path, json.as_bytes())
    }

    fn normalize_recent_projects(&mut self) {
        let mut unique = Vec::new();
        for path in self.recent_projects.drain(..) {
            if !unique.contains(&path) {
                unique.push(path);
            }
        }
        unique.truncate(MAX_RECENT_PROJECTS);
        self.recent_projects = unique;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remember_project_moves_existing_entry_to_front() {
        let mut settings = AppSettings {
            recent_projects: vec!["/a".into(), "/b".into(), "/c".into()],
            ..AppSettings::default()
        };
        settings.remember_project(Path::new("/c"));
        assert_eq!(
            settings.recent_projects,
            [PathBuf::from("/c"), "/a".into(), "/b".into()]
        );
    }

    #[test]
    fn remember_project_caps_the_list() {
        let mut settings = AppSettings::default();
        for index in 0..15 {
            settings.remember_project(Path::new(&format!("/project{index}")));
        }
        assert_eq!(settings.recent_projects.len(), MAX_RECENT_PROJECTS);
        assert_eq!(settings.recent_projects[0], PathBuf::from("/project14"));
        assert_eq!(settings.recent_projects[9], PathBuf::from("/project5"));
    }

    #[test]
    fn settings_round_trip_through_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join(SETTINGS_FILE_NAME);
        let mut settings = AppSettings {
            dark_mode: true,
            ..AppSettings::default()
        };
        settings.remember_project(Path::new("/home/me/Lectures"));
        settings.open_project = Some("/home/me/Lectures".into());

        settings.save_to(&path).unwrap();

        assert_eq!(AppSettings::load_from(&path), settings);
    }

    #[test]
    fn load_from_falls_back_to_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.json");
        let corrupt = dir.path().join("corrupt.json");
        fs::write(&corrupt, "{ not json").unwrap();

        assert_eq!(AppSettings::load_from(&missing), AppSettings::default());
        assert_eq!(AppSettings::load_from(&corrupt), AppSettings::default());
    }

    #[test]
    fn load_from_tolerates_missing_fields_and_cleans_the_list() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(SETTINGS_FILE_NAME);
        fs::write(&path, r#"{"recent_projects": ["/a", "/b", "/a"]}"#).unwrap();

        let settings = AppSettings::load_from(&path);

        assert_eq!(settings.recent_projects, [PathBuf::from("/a"), "/b".into()]);
        assert!(!settings.dark_mode);
        assert_eq!(settings.spelling_languages, [DEFAULT_SPELLING_LANGUAGE]);
    }

    #[test]
    fn toggle_spelling_language_adds_and_removes_it() {
        let mut settings = AppSettings::default();
        settings.toggle_spelling_language("nl_NL");
        assert_eq!(settings.spelling_languages, ["en_US", "nl_NL"]);
        settings.toggle_spelling_language("en_US");
        assert_eq!(settings.spelling_languages, ["nl_NL"]);
    }

    #[test]
    fn spelling_can_be_turned_off_for_good() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(SETTINGS_FILE_NAME);
        let settings = AppSettings {
            spelling_languages: Vec::new(),
            ..AppSettings::default()
        };

        settings.save_to(&path).unwrap();

        assert!(AppSettings::load_from(&path).spelling_languages.is_empty());
    }
}
