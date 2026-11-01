//! Finding the Hunspell dictionaries spell checking can use: the built-in
//! ones, plus `.aff`/`.dic` file pairs in the user's dictionary folder and
//! the system's Hunspell folders.

use std::fs;
use std::path::{Path, PathBuf};

use crate::settings::SETTINGS_DIR_NAME;
use crate::{CoreError, Result};

use super::Dictionary;

/// Dictionaries compiled into the app, so spell checking works without
/// anything installed. Sources and licences: `dictionaries/README.md`.
const BUILT_IN: &[(&str, &str, &str)] = &[
    (
        "en_US",
        include_str!("../../dictionaries/en_US.aff"),
        include_str!("../../dictionaries/en_US.dic"),
    ),
    (
        "nl_NL",
        include_str!("../../dictionaries/nl_NL.aff"),
        include_str!("../../dictionaries/nl_NL.dic"),
    ),
];

/// Where Linux distributions install Hunspell dictionaries.
const SYSTEM_FOLDERS: &[&str] = &[
    "/usr/share/hunspell",
    "/usr/share/myspell",
    "/usr/share/myspell/dicts",
];

/// Folder in the app's config directory where users add dictionaries.
const USER_FOLDER_NAME: &str = "dictionaries";

/// A language spell checking can use, backed by one Hunspell dictionary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Language {
    code: String,
    source: DictionarySource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DictionarySource {
    BuiltIn {
        aff: &'static str,
        dic: &'static str,
    },
    Files {
        aff: PathBuf,
        dic: PathBuf,
    },
}

impl Language {
    /// Every language available: the built-in ones and those found in
    /// [`Language::user_folder`] and the system's Hunspell folders.
    /// See [`Language::available_in`] for the rules.
    pub fn available() -> Vec<Language> {
        let mut folders: Vec<PathBuf> = Self::user_folder().into_iter().collect();
        folders.extend(SYSTEM_FOLDERS.iter().map(PathBuf::from));
        Self::available_in(&folders)
    }

    /// The built-in languages plus every `<code>.aff`/`<code>.dic` pair in
    /// `folders`, sorted by code. When a code appears more than once, the
    /// first folder wins, and files win over a built-in dictionary.
    ///
    /// Folders that are missing or cannot be read add no languages: most
    /// systems have only some of the folders searched.
    pub fn available_in(folders: &[PathBuf]) -> Vec<Language> {
        let mut languages: Vec<Language> = Vec::new();
        let found = folders
            .iter()
            .flat_map(|folder| dictionary_files_in(folder));
        let built_in = BUILT_IN.iter().map(|&(code, aff, dic)| Language {
            code: code.to_string(),
            source: DictionarySource::BuiltIn { aff, dic },
        });
        for language in found.chain(built_in) {
            if !languages.iter().any(|known| known.code == language.code) {
                languages.push(language);
            }
        }
        languages.sort_by(|a, b| a.code.cmp(&b.code));
        languages
    }

    /// Where users can add Hunspell dictionaries (`quickmath/dictionaries`
    /// in the OS config directory), or `None` if the OS has no config directory.
    pub fn user_folder() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join(SETTINGS_DIR_NAME).join(USER_FOLDER_NAME))
    }

    /// The dictionary's code, such as `en_US` or `nl_NL`: its file name
    /// without extension. Settings store languages by this code.
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Reads and parses the dictionary. Takes up to a few hundred
    /// milliseconds for large dictionaries, so callers should not do this
    /// on the UI thread.
    pub fn load(&self) -> Result<Dictionary> {
        let parsed = match &self.source {
            DictionarySource::BuiltIn { aff, dic } => spellbook::Dictionary::new(aff, dic),
            DictionarySource::Files { aff, dic } => {
                let aff = read_utf8(aff)?;
                let dic = read_utf8(dic)?;
                spellbook::Dictionary::new(&aff, &dic)
            }
        };
        match parsed {
            Ok(words) => Ok(Dictionary {
                code: self.code.clone(),
                words,
            }),
            Err(error) => Err(CoreError::Dictionary {
                language: self.code.clone(),
                message: error.to_string(),
            }),
        }
    }
}

/// The `.dic` files in `folder` that have an `.aff` file next to them.
/// (A `.dic` file alone is a hyphenation pattern or word list, not a dictionary.)
fn dictionary_files_in(folder: &Path) -> Vec<Language> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "dic"))
        .filter_map(|dic| {
            let aff = dic.with_extension("aff");
            let code = dic.file_stem()?.to_str()?.to_string();
            aff.is_file().then_some(Language {
                code,
                source: DictionarySource::Files { aff, dic },
            })
        })
        .collect()
}

/// Dictionary files must be UTF-8 (`SET UTF-8`), like the built-in ones.
fn read_utf8(path: &Path) -> Result<String> {
    let bytes = fs::read(path).map_err(|error| CoreError::io(path, error))?;
    String::from_utf8(bytes).map_err(|_| CoreError::Dictionary {
        language: path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default(),
        message: format!(
            "{} is not UTF-8; convert it to UTF-8 to use it",
            path.display()
        ),
    })
}
