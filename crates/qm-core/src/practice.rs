//! Progress in the notation practice game, stored as
//! `quickmath/practice.json` next to the app settings.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::fs_util::{create_dir_all, write_atomically};
use crate::settings::SETTINGS_DIR_NAME;
use crate::{CoreError, Result};

const PRACTICE_FILE_NAME: &str = "practice.json";

/// The best result reached on one practice level.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LevelRecord {
    /// 0 means never finished; at least 1 once the level was completed.
    pub stars: u8,
    pub best_wpm: f64,
}

/// Best results per practice level (keyed by a stable level id) and for the sprint.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PracticeProgress {
    pub levels: BTreeMap<String, LevelRecord>,
    pub best_sprint_wpm: f64,
}

impl PracticeProgress {
    /// Loads progress, falling back to none when the file is missing or unreadable.
    pub fn load() -> PracticeProgress {
        Self::default_path()
            .map(|path| Self::load_from(&path))
            .unwrap_or_default()
    }

    /// Saves to [`PracticeProgress::default_path`], creating the folder if needed.
    pub fn save(&self) -> Result<()> {
        let path = Self::default_path().ok_or_else(|| {
            let error = std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "the OS does not provide a config directory",
            );
            CoreError::io(Path::new(PRACTICE_FILE_NAME), error)
        })?;
        self.save_to(&path)
    }

    /// Where progress is kept, or `None` if the OS has no config directory.
    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join(SETTINGS_DIR_NAME).join(PRACTICE_FILE_NAME))
    }

    /// Like [`PracticeProgress::load`], but from an explicit file. Never fails.
    pub fn load_from(path: &Path) -> PracticeProgress {
        fs::read_to_string(path)
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default()
    }

    /// Like [`PracticeProgress::save`], but to an explicit file (written atomically).
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(folder) = path.parent() {
            create_dir_all(folder)?;
        }
        let json = serde_json::to_string_pretty(self).expect("progress always serializes");
        write_atomically(path, json.as_bytes())
    }

    /// The best result on `level_id`, or an unfinished record.
    pub fn level(&self, level_id: &str) -> LevelRecord {
        self.levels.get(level_id).copied().unwrap_or_default()
    }

    /// Keeps the higher star count and the higher speed separately. Returns
    /// whether `wpm` beat the previous best speed.
    pub fn record_level(&mut self, level_id: &str, stars: u8, wpm: f64) -> bool {
        let record = self.levels.entry(level_id.to_string()).or_default();
        record.stars = record.stars.max(stars);
        let is_new_best = wpm > record.best_wpm;
        if is_new_best {
            record.best_wpm = wpm;
        }
        is_new_best
    }

    /// Returns whether `wpm` beat the previous best sprint.
    pub fn record_sprint(&mut self, wpm: f64) -> bool {
        let is_new_best = wpm > self.best_sprint_wpm;
        if is_new_best {
            self.best_sprint_wpm = wpm;
        }
        is_new_best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_level_keeps_best_stars_and_speed_independently() {
        let mut progress = PracticeProgress::default();

        assert!(progress.record_level("fractions", 3, 20.0));
        assert!(progress.record_level("fractions", 1, 30.0));
        assert!(!progress.record_level("fractions", 2, 25.0));

        let record = progress.level("fractions");
        assert_eq!(record.stars, 3);
        assert_eq!(record.best_wpm, 30.0);
    }

    #[test]
    fn unknown_level_is_unfinished() {
        assert_eq!(PracticeProgress::default().level("missing").stars, 0);
    }

    #[test]
    fn record_sprint_reports_only_improvements() {
        let mut progress = PracticeProgress::default();

        assert!(progress.record_sprint(18.5));
        assert!(!progress.record_sprint(18.5));
        assert_eq!(progress.best_sprint_wpm, 18.5);
    }

    #[test]
    fn progress_round_trips_through_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join(PRACTICE_FILE_NAME);
        let mut progress = PracticeProgress::default();
        progress.record_level("basics", 2, 27.0);
        progress.record_sprint(31.0);

        progress.save_to(&path).unwrap();

        assert_eq!(PracticeProgress::load_from(&path), progress);
    }

    #[test]
    fn load_from_falls_back_to_no_progress() {
        let dir = tempfile::tempdir().unwrap();
        let corrupt = dir.path().join("corrupt.json");
        fs::write(&corrupt, "[1, 2").unwrap();

        assert_eq!(
            PracticeProgress::load_from(&corrupt),
            PracticeProgress::default()
        );
        assert_eq!(
            PracticeProgress::load_from(&dir.path().join("missing.json")),
            PracticeProgress::default()
        );
    }
}
