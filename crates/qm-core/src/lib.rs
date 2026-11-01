//! QuickMath backend: projects (Obsidian-style vault folders), note storage,
//! app settings, practice progress, spell checking, slide decks and exporters.
//!
//! The GUI talks to the file system and exporters only through this crate.

mod clock;
mod error;
pub mod export;
mod fs_util;
mod practice;
mod project;
mod settings;
mod slides;
mod spelling;

pub use error::{CoreError, Result};
pub use export::{export_note, export_project, ExportFormat};
pub use practice::{LevelRecord, PracticeProgress};
pub use project::{FileKind, FileNode, NoteSlides, Project, WorkspaceState};
pub use settings::{AppSettings, MAX_RECENT_PROJECTS};
pub use slides::{is_slide_deck, SlideDeck, SlideImage};
pub use spelling::{Dictionary, Language, SpellChecker, WordList};

/// Name of the hidden folder that marks a directory as a QuickMath project
/// (like Obsidian's `.obsidian/`).
pub const PROJECT_DIR_NAME: &str = ".quickmath";
