//! QuickMath backend: projects (Obsidian-style vault folders), note storage,
//! app settings, practice progress and exporters.
//!
//! The GUI talks to the file system and exporters only through this crate.

mod error;
pub mod export;
mod fs_util;
mod practice;
mod project;
mod settings;

pub use error::{CoreError, Result};
pub use export::{export_note, export_project, ExportFormat};
pub use practice::{LevelRecord, PracticeProgress};
pub use project::{FileKind, FileNode, Project, WorkspaceState};
pub use settings::{AppSettings, MAX_RECENT_PROJECTS};

/// Name of the hidden folder that marks a directory as a QuickMath project
/// (like Obsidian's `.obsidian/`).
pub const PROJECT_DIR_NAME: &str = ".quickmath";
