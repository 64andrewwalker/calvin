//! LockfileRepository port - abstraction for lockfile persistence
//!
//! This trait allows the domain layer to load/save lockfiles
//! without knowing about TOML serialization details.

use crate::domain::entities::Lockfile;
use anyhow::Result;
use std::path::Path;

/// Error type for lockfile repository operations
#[derive(Debug)]
pub enum LockfileError {
    /// Lockfile not found
    NotFound(std::path::PathBuf),
    /// Parse error
    ParseError(String),
    /// I/O error
    IoError(String),

    /// Lockfile format incompatible
    VersionMismatch { found: u32, expected: u32 },
}

impl std::fmt::Display for LockfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockfileError::NotFound(path) => write!(f, "Lockfile not found: {}", path.display()),
            LockfileError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            LockfileError::IoError(msg) => write!(f, "I/O error: {}", msg),
            LockfileError::VersionMismatch { found, expected } => write!(
                f,
                "lockfile format incompatible (version {}, expected {})\n  → Fix: Run calvin migrate to update\n  → Run: calvin migrate",
                found, expected
            ),
        }
    }
}

impl std::error::Error for LockfileError {}

/// Abstract repository for lockfile persistence
///
/// The lockfile tracks deployed file hashes for change detection.
/// This trait is implemented by infrastructure layer.
pub trait LockfileRepository {
    /// Load lockfile from path, or create a new empty one if not found
    fn load_or_new(&self, path: &Path) -> Lockfile;

    /// Load lockfile from path
    fn load(&self, path: &Path) -> Result<Lockfile, LockfileError>;

    /// Save lockfile to path
    fn save(&self, lockfile: &Lockfile, path: &Path) -> Result<(), LockfileError>;

    /// Delete lockfile at path
    ///
    /// Used when all tracked files have been cleaned, leaving the lockfile empty.
    /// An empty lockfile contains no meaningful information and should be removed.
    fn delete(&self, path: &Path) -> Result<(), LockfileError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockfile_repository_trait_is_object_safe() {
        fn _assert_object_safe(_: &dyn LockfileRepository) {}
    }
}
