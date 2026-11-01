//! Spell checking with Hunspell dictionaries, in several languages at once.

mod languages;
mod word_list;

pub use languages::Language;
pub use word_list::WordList;

use crate::CoreError;

/// More suggestions than this rarely contain the intended word, and each
/// extra one makes the menu longer to scan.
const MAX_SUGGESTIONS: usize = 6;

/// One language's parsed dictionary, from [`Language::load`].
pub struct Dictionary {
    code: String,
    words: spellbook::Dictionary,
}

impl Dictionary {
    /// The code of the [`Language`] this dictionary was loaded from.
    pub fn code(&self) -> &str {
        &self.code
    }
}

/// Checks words against several dictionaries at once, so notes that mix
/// languages (Dutch prose with English terms) need no switching: a word is
/// correct when any dictionary accepts it.
pub struct SpellChecker {
    dictionaries: Vec<Dictionary>,
}

impl SpellChecker {
    /// Checks with `dictionaries` and also accepts every word in
    /// `word_list`. Entries of the list that the dictionaries cannot take
    /// (only possible in a hand-edited list) are left out and returned as errors.
    pub fn new(
        dictionaries: Vec<Dictionary>,
        word_list: &WordList,
    ) -> (SpellChecker, Vec<CoreError>) {
        let mut checker = SpellChecker { dictionaries };
        let errors = word_list
            .words()
            .iter()
            .filter_map(|word| checker.accept(word).err())
            .collect();
        (checker, errors)
    }

    /// Codes of the languages checked, in the order the dictionaries were given.
    pub fn languages(&self) -> impl Iterator<Item = &str> {
        self.dictionaries.iter().map(Dictionary::code)
    }

    /// Whether any dictionary accepts `word`. Capitalised and all-capital
    /// forms of a dictionary word are accepted (`Hello`, `HELLO`), as in
    /// Hunspell. Without dictionaries every word is correct.
    pub fn is_correct(&self, word: &str) -> bool {
        self.dictionaries.is_empty()
            || self
                .dictionaries
                .iter()
                .any(|dictionary| dictionary.words.check(word))
    }

    /// Corrections for `word`, best first, taking turns between the
    /// dictionaries so every language gets its best guesses in. Slow (tens
    /// of milliseconds per dictionary): call it for one word the user asks
    /// about, not for every word on screen.
    pub fn suggestions(&self, word: &str) -> Vec<String> {
        let per_dictionary: Vec<Vec<String>> = self
            .dictionaries
            .iter()
            .map(|dictionary| {
                let mut suggestions = Vec::new();
                dictionary.words.suggest(word, &mut suggestions);
                suggestions
            })
            .collect();
        let longest = per_dictionary.iter().map(Vec::len).max().unwrap_or(0);
        let mut merged: Vec<String> = Vec::new();
        for rank in 0..longest {
            for suggestion in per_dictionary.iter().filter_map(|list| list.get(rank)) {
                if !merged.contains(suggestion) {
                    merged.push(suggestion.clone());
                }
            }
        }
        merged.truncate(MAX_SUGGESTIONS);
        merged
    }

    /// Accepts `word` from now on, in every language. Only changes this
    /// checker: use [`WordList::add`] to keep the word for next time.
    pub fn accept(&mut self, word: &str) -> crate::Result<()> {
        for dictionary in &mut self.dictionaries {
            dictionary
                .words
                .add(word)
                .map_err(|error| CoreError::Dictionary {
                    language: dictionary.code.clone(),
                    message: format!("cannot add {word:?} from the word list: {error}"),
                })?;
        }
        Ok(())
    }
}
