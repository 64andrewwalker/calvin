//! FileSystem port - abstraction over file I/O operations
//!
//! This trait allows the domain layer to perform file operations
//! without depending on concrete implementations (local, remote, mock).

use std::path::{Path, PathBuf};

/// Result type for file system operations
pub type FsResult<T> = Result<T, FsError>;

/// File system operation errors
#[derive(Debug)]
pub enum FsError {
    /// File not found
    NotFound(PathBuf),
    /// Permission denied
    PermissionDenied(PathBuf),
    /// I/O error
    Io(std::io::Error),
    /// Other error
    Other(String),
}

impl From<std::io::Error> for FsError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => FsError::NotFound(PathBuf::new()),
            std::io::ErrorKind::PermissionDenied => FsError::PermissionDenied(PathBuf::new()),
            _ => FsError::Io(err),
        }
    }
}

impl std::fmt::Display for FsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FsError::NotFound(path) => write!(f, "File not found: {}", path.display()),
            FsError::PermissionDenied(path) => {
                write!(f, "Permission denied: {}", path.display())
            }
            FsError::Io(err) => write!(f, "I/O error: {}", err),
            FsError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for FsError {}

/// Abstract file system interface
///
/// Implementations:
/// - `LocalFileSystem` - standard file I/O
/// - `RemoteFileSystem` - SSH-based remote operations
/// - `MockFileSystem` - in-memory for testing
pub trait FileSystem {
    /// Read file content as string
    fn read(&self, path: &Path) -> FsResult<String>;

    /// Write content to file atomically
    fn write(&self, path: &Path, content: &str) -> FsResult<()>;

    /// Check if file exists
    fn exists(&self, path: &Path) -> bool;

    /// Remove a file (or an empty directory)
    fn remove(&self, path: &Path) -> FsResult<()>;

    /// Create directory and parents
    fn create_dir_all(&self, path: &Path) -> FsResult<()>;

    /// Compute content hash (SHA256)
    fn hash(&self, path: &Path) -> FsResult<String>;

    /// Expand ~ to home directory
    fn expand_home(&self, path: &Path) -> PathBuf;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fs_error_display() {
        let err = FsError::NotFound(PathBuf::from("test.txt"));
        assert!(err.to_string().contains("test.txt"));
    }

    #[test]
    fn fs_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let fs_err: FsError = io_err.into();
        matches!(fs_err, FsError::NotFound(_));
    }
}
