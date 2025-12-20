//! Local File System Implementation
//!
//! Implements the FileSystem port for local disk operations.

use crate::domain::ports::file_system::{FileSystem, FsError, FsResult};
use crate::error::CalvinResult;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use tempfile::NamedTempFile;

/// Local file system implementation
///
/// Provides atomic writes, home directory expansion, and standard file operations.
#[derive(Debug, Clone, Copy, Default)]
pub struct LocalFs;

impl LocalFs {
    /// Create a new LocalFs instance
    pub fn new() -> Self {
        Self
    }
}

impl FileSystem for LocalFs {
    fn read(&self, path: &Path) -> FsResult<String> {
        let expanded = self.expand_home(path);
        std::fs::read_to_string(&expanded).map_err(Into::into)
    }

    fn write(&self, path: &Path, content: &str) -> FsResult<()> {
        let expanded = self.expand_home(path);
        Self::atomic_write_internal(&expanded, content.as_bytes())
            .map_err(|e| FsError::Other(e.to_string()))
    }

    fn exists(&self, path: &Path) -> bool {
        let expanded = self.expand_home(path);
        expanded.exists()
    }

    fn remove(&self, path: &Path) -> FsResult<()> {
        let expanded = self.expand_home(path);
        std::fs::remove_file(&expanded).map_err(Into::into)
    }

    fn create_dir_all(&self, path: &Path) -> FsResult<()> {
        let expanded = self.expand_home(path);
        std::fs::create_dir_all(&expanded).map_err(Into::into)
    }

    fn hash(&self, path: &Path) -> FsResult<String> {
        use sha2::{Digest, Sha256};
        let expanded = self.expand_home(path);
        let content = std::fs::read(&expanded)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        Ok(format!("sha256:{:x}", hasher.finalize()))
    }

    fn expand_home(&self, path: &Path) -> PathBuf {
        let path_str = path.to_string_lossy();
        if path_str.starts_with("~/") || path_str == "~" {
            if let Some(home) = dirs::home_dir() {
                return home.join(path_str.strip_prefix("~/").unwrap_or(""));
            }
        }
        path.to_path_buf()
    }
}

/// Maximum retries for atomic write (Windows file locking)
const MAX_RETRIES: u32 = 3;
/// Retry delays in milliseconds
const RETRY_DELAYS_MS: [u64; 3] = [100, 500, 1000];

/// Additional convenience methods for LocalFs
impl LocalFs {
    /// Atomic write with CalvinResult error type (for legacy compatibility)
    pub fn write_atomic(&self, path: &Path, content: &str) -> CalvinResult<()> {
        Self::atomic_write_internal(path, content.as_bytes())
    }

    /// Hash file with CalvinResult error type (for legacy compatibility)
    pub fn hash_file(&self, path: &Path) -> CalvinResult<String> {
        let content = std::fs::read(path)?;
        Ok(format!("sha256:{:x}", Sha256::digest(&content)))
    }

    /// Internal atomic write implementation
    ///
    /// Uses tempfile + rename pattern to ensure atomic writes.
    /// On Windows, retries with backoff if file is locked.
    fn atomic_write_internal(path: &Path, content: &[u8]) -> CalvinResult<()> {
        let dir = path.parent().unwrap_or(Path::new("."));

        // Ensure directory exists
        std::fs::create_dir_all(dir)?;

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn local_fs_write_and_read() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.txt");
        let fs = LocalFs::new();

        fs.write(&file, "hello world").unwrap();
        let content = fs.read(&file).unwrap();

        assert_eq!(content, "hello world");
    }

    #[test]
    fn local_fs_write_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("nested").join("dir").join("test.txt");
        let fs = LocalFs::new();

        fs.write(&file, "content").unwrap();

        assert!(file.exists());
    }

    #[test]
    fn local_fs_exists() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("exists.txt");
        let fs = LocalFs::new();

        assert!(!fs.exists(&file));

        fs.write(&file, "content").unwrap();

        assert!(fs.exists(&file));
    }

    #[test]
    fn local_fs_remove() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("remove.txt");
        let fs = LocalFs::new();

        fs.write(&file, "content").unwrap();
        assert!(file.exists());

        fs.remove(&file).unwrap();
        assert!(!file.exists());
    }

    #[test]
    fn local_fs_create_dir_all() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("a").join("b").join("c");
        let fs = LocalFs::new();

        fs.create_dir_all(&nested).unwrap();

        assert!(nested.exists());
    }

    #[test]
    fn local_fs_hash() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("hash.txt");
        let fs = LocalFs::new();

        fs.write(&file, "hello").unwrap();
        let hash = fs.hash(&file).unwrap();

        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 7 + 64); // "sha256:" + 64 hex chars
    }

    #[test]
    fn local_fs_expand_home() {
        let fs = LocalFs::new();

        // Non-home path unchanged
        let path = PathBuf::from("/tmp/test");
        assert_eq!(fs.expand_home(&path), path);

        // Home path expanded
        let home_path = PathBuf::from("~/.claude");
        let expanded = fs.expand_home(&home_path);
        assert!(!expanded.to_string_lossy().contains('~'));
    }
}
