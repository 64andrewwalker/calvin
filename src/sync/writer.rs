//! Atomic file writer (legacy location)
//!
//! **Migration Note**: All functions in this module are now available through
//! the new architecture:
//!
//! - `atomic_write` → Use `infrastructure::fs::LocalFs::write()` or `write_atomic()`
//! - `hash_content` → Use `domain::value_objects::ContentHash::from_bytes()`
//! - `hash_file` → Use `LocalFs::hash()` or `FileSystem::hash_file()`
//!
//! This module provides backward-compatible wrappers for legacy code.

use std::path::Path;

use crate::error::CalvinResult;
use crate::infrastructure::fs::LocalFs;

/// Write content to a file atomically
///
/// Uses tempfile + rename pattern to ensure atomic writes.
///
/// **Note**: For new code, prefer using `LocalFs::write()` which has the same behavior.
pub fn atomic_write(path: &Path, content: &[u8]) -> CalvinResult<()> {
    use crate::domain::ports::FileSystem;
    let fs = LocalFs::new();
    fs.write(path, std::str::from_utf8(content).unwrap_or(""))
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    Ok(())
}

/// Compute SHA-256 hash of file content
///
/// **Note**: For new code, prefer using `ContentHash::from_bytes()`.
pub fn hash_content(content: &[u8]) -> String {
    crate::domain::value_objects::ContentHash::from_bytes(content).to_string()
}

/// Compute SHA-256 hash of a file
///
/// **Note**: For new code, prefer using `LocalFs::hash()` or `FileSystem::hash_file()`.
pub fn hash_file(path: &Path) -> CalvinResult<String> {
    let fs = LocalFs::new();
    fs.hash_file(path)
        .map(|h| h.to_string())
        .map_err(|e| std::io::Error::other(e.to_string()).into())
}

/// Check if file has the expected hash
///
/// **Note**: For new code, prefer using `LocalFs::hash()` and comparing directly.
pub fn verify_hash(path: &Path, expected: &str) -> CalvinResult<bool> {
    let actual = hash_file(path)?;
    Ok(actual == expected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn atomic_write_new_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");

        atomic_write(&path, b"Hello, World!").unwrap();

        assert!(path.exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), "Hello, World!");
    }

    #[test]
    fn atomic_write_overwrite() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");

        fs::write(&path, "Original").unwrap();
        atomic_write(&path, b"Replaced").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "Replaced");
    }

    #[test]
    fn hash_content_works() {
        let hash = hash_content(b"Hello, World!");
        assert!(hash.starts_with("sha256:"));
        // SHA-256 is 64 hex chars + "sha256:" prefix
        assert_eq!(hash.len(), 71);
    }

    #[test]
    fn hash_file_works() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "Content").unwrap();

        let hash = hash_file(&path).unwrap();
        let expected = hash_content(b"Content");
        assert_eq!(hash, expected);
    }

    #[test]
    fn verify_hash_works() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "Content").unwrap();

        let hash = hash_content(b"Content");
        assert!(verify_hash(&path, &hash).unwrap());

        let wrong_hash = hash_content(b"Different");
        assert!(!verify_hash(&path, &wrong_hash).unwrap());
    }
}
