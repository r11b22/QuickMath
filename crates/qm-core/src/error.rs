use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum CoreError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    InvalidName(String),
    AlreadyExists(PathBuf),
    OutsideProject(PathBuf),
    Export(String),
    /// A spelling dictionary could not be loaded.
    Dictionary {
        language: String,
        message: String,
    },
}

impl CoreError {
    /// Wraps an I/O error together with the path it happened on.
    pub(crate) fn io(path: &Path, source: std::io::Error) -> CoreError {
        CoreError::Io {
            path: path.to_path_buf(),
            source,
        }
    }
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::Io { path, source } => write!(f, "{}: {source}", path.display()),
            CoreError::InvalidName(name) => write!(f, "invalid name: {name:?}"),
            CoreError::AlreadyExists(path) => write!(f, "{} already exists", path.display()),
            CoreError::OutsideProject(path) => {
                write!(f, "{} is outside the project", path.display())
            }
            CoreError::Export(message) => write!(f, "export failed: {message}"),
            CoreError::Dictionary { language, message } => {
                write!(f, "{language} dictionary: {message}")
            }
        }
    }
}

impl std::error::Error for CoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CoreError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, CoreError>;
