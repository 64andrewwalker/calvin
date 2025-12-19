//! LockfileRepository port - abstraction for lockfile persistence
//!
//! This trait allows the domain layer to load/save lockfiles
//! without knowing about TOML serialization details.

use std::path::Path;

/// Result type for lockfile operations
pub type LockfileResult<T> = Result<T, LockfileError>;

/// Lockfile operation errors
#[derive(Debug)]
pub enum LockfileError {
    /// Lockfile not found (not an error - create new)
    NotFound,
    /// Invalid lockfile format
    InvalidFormat(String),
    /// I/O error
    Io(std::io::Error),
}

impl std::fmt::Display for LockfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockfileError::NotFound => write!(f, "Lockfile not found"),
            LockfileError::InvalidFormat(msg) => write!(f, "Invalid lockfile format: {}", msg),
            LockfileError::Io(err) => write!(f, "I/O error: {}", err),
        }
    }
}

impl std::error::Error for LockfileError {}

/// Abstract repository for lockfile persistence
///
/// The lockfile tracks deployed file hashes for change detection.
/// This trait is implemented by infrastructure layer.
pub trait LockfileRepository {
    /// Lockfile type (to be defined in entities)
    type Lockfile;

    /// Load lockfile from path, or create empty if not exists
    fn load_or_new(&self, path: &Path) -> Self::Lockfile;

    /// Load lockfile from path
    fn load(&self, path: &Path) -> LockfileResult<Self::Lockfile>;

    /// Save lockfile to path
    fn save(&self, lockfile: &Self::Lockfile, path: &Path) -> LockfileResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockfile_error_display() {
        let err = LockfileError::InvalidFormat("bad toml".to_string());
        assert!(err.to_string().contains("bad toml"));
    }
}
