//! The note currently open in the editor: its text, save state and parsed
//! document.

use std::path::{Path, PathBuf};

use qm_core::Project;
use qm_format::Document;

use crate::autosave::AutosaveTimer;
use crate::text_metrics::{self, TextPosition};

/// Requests for the editor, applied on its next frame (they need the
/// editor's widget state, which only exists while it is being drawn).
#[derive(Debug, Clone, Default)]
pub struct EditorRequests {
    /// Move the cursor to the start of this zero-based line and scroll there.
    pub jump_to_line: Option<usize>,
    /// Insert this text at the cursor (replacing any selection).
    pub insertion: Option<String>,
}

pub struct OpenNote {
    path: PathBuf,
    /// Edited in place by the editor widget; call [`OpenNote::mark_edited`] after changing it.
    pub text: String,
    document: Document,
    word_count: usize,
    is_dirty: bool,
    autosave: AutosaveTimer,
    /// Where the editor cursor is, updated by the editor every frame.
    pub cursor: TextPosition,
    pub requests: EditorRequests,
}

impl OpenNote {
    /// Reads the note at `path` from `project`.
    pub fn load(project: &Project, path: &Path) -> qm_core::Result<OpenNote> {
        let text = project.read_note(path)?;
        Ok(OpenNote {
            path: path.to_path_buf(),
            document: qm_format::parse_document(&text),
            word_count: text_metrics::word_count(&text),
            text,
            is_dirty: false,
            autosave: AutosaveTimer::default(),
            cursor: TextPosition::default(),
            requests: EditorRequests::default(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The note's file name without the `.qm` extension.
    pub fn title(&self) -> String {
        self.path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// The parsed note; re-parsed only when the text changes.
    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn word_count(&self) -> usize {
        self.word_count
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    /// Records that `text` changed at time `now`: re-parses and schedules an autosave.
    pub fn mark_edited(&mut self, now: f64) {
        self.document = qm_format::parse_document(&self.text);
        self.word_count = text_metrics::word_count(&self.text);
        self.is_dirty = true;
        self.autosave.record_edit(now);
    }

    /// Updates the stored path after the note (or a parent folder) was renamed.
    pub fn set_path(&mut self, path: PathBuf) {
        self.path = path;
    }

    /// Writes the note if it has unsaved changes. On failure the note stays
    /// dirty, so the text is never silently dropped.
    pub fn save(&mut self, project: &Project) -> qm_core::Result<()> {
        if !self.is_dirty {
            return Ok(());
        }
        project.write_note(&self.path, &self.text)?;
        self.is_dirty = false;
        self.autosave.clear();
        Ok(())
    }

    /// Saves if typing has paused long enough. Returns `None` when nothing was due.
    pub fn autosave_if_due(&mut self, project: &Project, now: f64) -> Option<qm_core::Result<()>> {
        if !self.autosave.is_due(now) {
            return None;
        }
        // Clear first so a failing disk is retried on the next edit, not every frame.
        self.autosave.clear();
        Some(self.save(project))
    }

    /// Seconds until the pending autosave, so the app can schedule a repaint.
    pub fn seconds_until_autosave(&self, now: f64) -> Option<f64> {
        self.autosave.seconds_until_due(now)
    }
}
