//! The app's spell checker: the languages there are, the personal word
//! list, and loading the chosen dictionaries on a background thread so
//! startup and switching languages never block typing.

use std::thread::JoinHandle;
use std::time::Duration;

use qm_core::{Dictionary, Language, SpellChecker, WordList};

use crate::toasts::Toasts;

const LOAD_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// The spell checker as views see it for one frame, from [`Spelling::current`].
#[derive(Clone, Copy)]
pub struct SpellCheck<'a> {
    pub checker: &'a SpellChecker,
    /// Changes whenever the checker's answers may change, so views know
    /// when their cached results are stale.
    pub revision: u64,
}

pub struct Spelling {
    available: Vec<Language>,
    word_list: WordList,
    checker: Option<SpellChecker>,
    loading: Option<JoinHandle<Vec<qm_core::Result<Dictionary>>>>,
    revision: u64,
}

impl Spelling {
    /// Finds the available languages and starts loading those with `codes`.
    pub fn start(codes: &[String], toasts: &mut Toasts) -> Spelling {
        let mut spelling = Spelling {
            available: Language::available(),
            word_list: WordList::load(),
            checker: None,
            loading: None,
            revision: 0,
        };
        spelling.use_languages(codes, toasts);
        spelling
    }

    /// Every language that can be turned on, sorted by code.
    pub fn available(&self) -> &[Language] {
        &self.available
    }

    /// Switches to checking in the languages with `codes`; none turns spell
    /// checking off. Codes without a dictionary are reported and skipped.
    pub fn use_languages(&mut self, codes: &[String], toasts: &mut Toasts) {
        let mut languages = Vec::new();
        for code in codes {
            match self
                .available
                .iter()
                .find(|language| language.code() == code)
            {
                Some(language) => languages.push(language.clone()),
                None => toasts.error(format!(
                    "No spelling dictionary for {code} was found, so it is not checked."
                )),
            }
        }
        self.checker = None;
        self.revision += 1;
        // Replacing a running job detaches it; its result is simply dropped.
        self.loading = (!languages.is_empty())
            .then(|| std::thread::spawn(move || languages.iter().map(Language::load).collect()));
    }

    /// Takes over the dictionaries once they are loaded. Call every frame;
    /// keeps the UI repainting while loading.
    pub fn poll(&mut self, ctx: &egui::Context, toasts: &mut Toasts) {
        let Some(job) = &self.loading else {
            return;
        };
        if !job.is_finished() {
            ctx.request_repaint_after(LOAD_POLL_INTERVAL);
            return;
        }
        let Some(job) = self.loading.take() else {
            return;
        };
        let Ok(results) = job.join() else {
            toasts.error("Loading the spelling dictionaries crashed unexpectedly.");
            return;
        };
        let mut dictionaries = Vec::new();
        for result in results {
            match result {
                Ok(dictionary) => dictionaries.push(dictionary),
                Err(error) => {
                    toasts.error(format!("Could not load a spelling dictionary: {error}"))
                }
            }
        }
        if dictionaries.is_empty() {
            return;
        }
        let (checker, errors) = SpellChecker::new(dictionaries, &self.word_list);
        for error in errors {
            toasts.error(format!(
                "Your word list has an entry that was skipped: {error}"
            ));
        }
        self.checker = Some(checker);
        self.revision += 1;
    }

    /// The checker, once its dictionaries are loaded and while spell
    /// checking is on.
    pub fn current(&self) -> Option<SpellCheck<'_>> {
        self.checker.as_ref().map(|checker| SpellCheck {
            checker,
            revision: self.revision,
        })
    }

    /// Accepts `word` from now on and keeps it in the personal word list.
    /// If the list cannot be saved, the word is still accepted until the
    /// app quits, and the failure is reported.
    pub fn add_word(&mut self, word: &str, toasts: &mut Toasts) {
        if let Err(error) = self.word_list.add(word) {
            toasts.error(format!(
                "Could not save \"{word}\" to your word list: {error}"
            ));
        }
        let Some(checker) = &mut self.checker else {
            return;
        };
        if let Err(error) = checker.accept(word) {
            toasts.error(format!("Could not add \"{word}\": {error}"));
        }
        self.revision += 1;
    }
}
