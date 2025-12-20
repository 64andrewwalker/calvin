//! Legacy File System Trait
//!
//! This module provides the legacy FileSystem trait used by the sync module.
//!
//! For new code, prefer using:
//! - `infrastructure::fs::LocalFs` for local operations
//! - `infrastructure::fs::RemoteFs` for remote SSH operations
//! - `domain::ports::file_system::FileSystem` for the new trait

use crate::error::CalvinResult;
use std::path::{Path, PathBuf};

/// Abstract file system interface (legacy)
///
/// This trait is used by the sync module. For new code, prefer the
/// `domain::ports::file_system::FileSystem` trait.
pub trait FileSystem {
    /// Read file content
    fn read_to_string(&self, path: &Path) -> CalvinResult<String>;

    /// Write file content atomically
    fn write_atomic(&self, path: &Path, content: &str) -> CalvinResult<()>;

    /// Check if file exists
    fn exists(&self, path: &Path) -> bool;

    /// Compute SHA256 hash of file content
    fn hash_file(&self, path: &Path) -> CalvinResult<String>;

    /// Create directory and parents
    fn create_dir_all(&self, path: &Path) -> CalvinResult<()>;

    /// Expand ~ to home directory
    fn expand_home(&self, path: &Path) -> PathBuf;

    /// Remove a file
    fn remove_file(&self, path: &Path) -> CalvinResult<()>;
}

// Re-export implementations from infrastructure layer
pub use crate::infrastructure::fs::LocalFs;
pub use crate::infrastructure::fs::RemoteFs;

/// Mock file system for testing
///
/// Uses `Arc<Mutex<>>` internally so it can be cloned and shared.
#[cfg(test)]
#[derive(Clone)]
pub struct MockFileSystem {
    pub files: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<PathBuf, String>>>,
}

#[cfg(test)]
impl MockFileSystem {
    pub fn new() -> Self {
        Self {
            files: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }
}

#[cfg(test)]
impl Default for MockFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl FileSystem for MockFileSystem {
    fn read_to_string(&self, path: &Path) -> CalvinResult<String> {
        let files = self.files.lock().unwrap();
        files.get(path).cloned().ok_or_else(|| {
            crate::error::CalvinError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File not found",
            ))
        })
    }

    fn write_atomic(&self, path: &Path, content: &str) -> CalvinResult<()> {
        let mut files = self.files.lock().unwrap();
        files.insert(path.to_path_buf(), content.to_string());
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        let files = self.files.lock().unwrap();
        files.contains_key(path)
    }

    fn hash_file(&self, path: &Path) -> CalvinResult<String> {
        use sha2::{Digest, Sha256};
        let content = self.read_to_string(path)?;
        let mut hasher = Sha256::new();
        hasher.update(content);
        Ok(format!("sha256:{:x}", hasher.finalize()))
    }

    fn create_dir_all(&self, _path: &Path) -> CalvinResult<()> {
        Ok(())
    }

    fn expand_home(&self, path: &Path) -> PathBuf {
        let p = path.to_string_lossy();
        if p.starts_with("~/") {
            PathBuf::from("/mock/home").join(p.strip_prefix("~/").unwrap())
        } else if p == "~" {
            PathBuf::from("/mock/home")
        } else {
            path.to_path_buf()
        }
    }

    fn remove_file(&self, path: &Path) -> CalvinResult<()> {
        let mut files = self.files.lock().unwrap();
        files.remove(path);
        Ok(())
    }
}
