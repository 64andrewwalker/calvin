//! Atomic file writer with retry logic
//!
//! Implements TD-14: Atomic writes via tempfile + rename

use std::fs;
use std::io::Write;
use std::path::Path;
use std::thread;
use std::time::Duration;

use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;

use crate::error::CalvinResult;

/// Maximum retries for atomic write (Windows file locking)
const MAX_RETRIES: u32 = 3;

/// Retry delays in milliseconds
const RETRY_DELAYS_MS: [u64; 3] = [100, 500, 1000];

/// Write content to a file atomically
///
/// Uses tempfile + rename pattern to ensure atomic writes.
/// On Windows, retries with backoff if file is locked.
pub fn atomic_write(path: &Path, content: &[u8]) -> CalvinResult<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    
    // Ensure directory exists
    fs::create_dir_all(dir)?;
    
    // Create temp file in same directory (ensures same filesystem)
    let mut temp = NamedTempFile::new_in(dir)?;
    temp.write_all(content)?;
    temp.flush()?;
    
    // Try atomic rename with retries
    for attempt in 0..=MAX_RETRIES {
        match temp.persist(path) {
            Ok(_) => return Ok(()),
            Err(e) => {
                if attempt < MAX_RETRIES {
                    // Retry with backoff
                    let delay = Duration::from_millis(RETRY_DELAYS_MS[attempt as usize]);
                    thread::sleep(delay);
                    temp = e.file;
                } else {
                    // Exhausted retries
                    return Err(e.error.into());
                }
            }
        }
    }
    
    Ok(())
}

/// Compute SHA-256 hash of file content
pub fn hash_content(content: &[u8]) -> String {
    let hash = Sha256::digest(content);
    format!("sha256:{:x}", hash)
}

/// Compute SHA-256 hash of a file
pub fn hash_file(path: &Path) -> CalvinResult<String> {
    let content = fs::read(path)?;
    Ok(hash_content(&content))
}

/// Check if file has the expected hash
pub fn verify_hash(path: &Path, expected: &str) -> CalvinResult<bool> {
    let actual = hash_file(path)?;
    Ok(actual == expected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_atomic_write_new_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        
        atomic_write(&path, b"Hello, World!").unwrap();
        
        assert!(path.exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), "Hello, World!");
    }

    #[test]
    fn test_atomic_write_overwrite() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        
        fs::write(&path, "Original").unwrap();
        atomic_write(&path, b"Replaced").unwrap();
        
        assert_eq!(fs::read_to_string(&path).unwrap(), "Replaced");
    }

    #[test]
    fn test_atomic_write_nested_directory() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested/deep/test.txt");
        
        atomic_write(&path, b"Content").unwrap();
        
        assert!(path.exists());
    }

    #[test]
    fn test_hash_content() {
        let hash = hash_content(b"Hello, World!");
        assert!(hash.starts_with("sha256:"));
        // SHA-256 is 64 hex chars + "sha256:" prefix
        assert_eq!(hash.len(), 71);
    }

    #[test]
    fn test_hash_content_deterministic() {
        let hash1 = hash_content(b"Test");
        let hash2 = hash_content(b"Test");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_content_different() {
        let hash1 = hash_content(b"Test1");
        let hash2 = hash_content(b"Test2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "Content").unwrap();
        
        let hash = hash_file(&path).unwrap();
        let expected = hash_content(b"Content");
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_verify_hash() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "Content").unwrap();
        
        let hash = hash_content(b"Content");
        assert!(verify_hash(&path, &hash).unwrap());
        
        let wrong_hash = hash_content(b"Different");
        assert!(!verify_hash(&path, &wrong_hash).unwrap());
    }
}
