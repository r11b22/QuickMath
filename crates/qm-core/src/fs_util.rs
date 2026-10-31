//! Small file-system helpers shared by projects, settings and exporters.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{CoreError, Result};

/// Distinguishes temp files created concurrently by the same process.
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Writes `contents` to `path` atomically: the data goes to a hidden temp file
/// in the same directory (so the rename stays on one file system), is flushed
/// to disk, and then replaces `path` in a single rename. A crash therefore
/// leaves either the old or the new file, never a truncated one.
pub(crate) fn write_atomically(path: &Path, contents: &[u8]) -> Result<()> {
    let temp_path = temp_path_for(path)?;
    let result = write_and_sync(&temp_path, contents)
        .and_then(|()| fs::rename(&temp_path, path))
        .map_err(|error| CoreError::io(path, error));
    if result.is_err() {
        // Best effort: the original error is more useful than a cleanup failure.
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn temp_path_for(path: &Path) -> Result<PathBuf> {
    let file_name = path
        .file_name()
        .ok_or_else(|| CoreError::InvalidName(format!("{} has no file name", path.display())))?;
    let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_name = format!(
        ".{}.{}-{counter}.tmp",
        file_name.to_string_lossy(),
        std::process::id()
    );
    Ok(path.with_file_name(temp_name))
}

fn write_and_sync(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(contents)?;
    file.sync_all()
}

/// Creates `path` and all missing parents.
pub(crate) fn create_dir_all(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|error| CoreError::io(path, error))
}
