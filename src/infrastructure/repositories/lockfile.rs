//! TOML Lockfile Repository
//!
//! Implements the LockfileRepository port using TOML format.

use crate::domain::entities::Lockfile;
use crate::domain::ports::file_system::FileSystem;
use crate::domain::ports::lockfile_repository::{LockfileError, LockfileRepository};
use crate::infrastructure::fs::LocalFs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// TOML-based lockfile repository
///
/// Stores lockfile as `.calvin.lock` in TOML format.
pub struct TomlLockfileRepository {
    fs: LocalFs,
}

impl TomlLockfileRepository {
    /// Create a new repository with the default file system
    pub fn new() -> Self {
        Self { fs: LocalFs::new() }
    }

    /// Create with a custom file system (for testing)
    pub fn with_fs(fs: LocalFs) -> Self {
        Self { fs }
    }
}

impl Default for TomlLockfileRepository {
    fn default() -> Self {
        Self::new()
    }
}

/// TOML representation of a file entry
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TomlFileEntry {
    hash: String,
}

/// TOML representation of the lockfile
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TomlLockfile {
    version: u32,
    #[serde(default)]
    files: BTreeMap<String, TomlFileEntry>,
}

impl LockfileRepository for TomlLockfileRepository {
    fn load_or_new(&self, path: &Path) -> Lockfile {
        self.load(path).unwrap_or_else(|_| Lockfile::new())
    }

    fn load(&self, path: &Path) -> Result<Lockfile, LockfileError> {
        if !self.fs.exists(path) {
            return Ok(Lockfile::new());
        }

        let content = self
            .fs
            .read(path)
            .map_err(|e| LockfileError::IoError(e.to_string()))?;
        let toml_lockfile: TomlLockfile =
            toml::from_str(&content).map_err(|e| LockfileError::ParseError(e.to_string()))?;

        let mut lockfile = Lockfile::new();
        for (key, entry) in toml_lockfile.files {
            lockfile.set(&key, &entry.hash);
        }

        Ok(lockfile)
    }

    fn save(&self, lockfile: &Lockfile, path: &Path) -> Result<(), LockfileError> {
        let mut files = BTreeMap::new();
        for (key, entry) in lockfile.entries() {
            files.insert(
                key.to_string(),
                TomlFileEntry {
                    hash: entry.hash().to_string(),
                },
            );
        }

        let toml_lockfile = TomlLockfile {
            version: lockfile.version(),
            files,
        };

        let content = toml::to_string_pretty(&toml_lockfile)
            .map_err(|e| LockfileError::ParseError(e.to_string()))?;
        self.fs
            .write(path, &content)
            .map_err(|e| LockfileError::IoError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_nonexistent_returns_empty_lockfile() {
        let repo = TomlLockfileRepository::new();
        let lockfile = repo.load(Path::new("/nonexistent/.calvin.lock")).unwrap();

        assert!(lockfile.is_empty());
        assert_eq!(lockfile.version(), 1);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let lockfile_path = dir.path().join(".calvin.lock");

        let repo = TomlLockfileRepository::new();

        // Create and save lockfile
        let mut lockfile = Lockfile::new();
        lockfile.set("project:.claude/commands/test.md", "sha256:abc123");
        lockfile.set("home:~/.claude/commands/global.md", "sha256:def456");

        repo.save(&lockfile, &lockfile_path).unwrap();

        // Load it back
        let loaded = repo.load(&lockfile_path).unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(
            loaded.get_hash("project:.claude/commands/test.md"),
            Some("sha256:abc123")
        );
        assert_eq!(
            loaded.get_hash("home:~/.claude/commands/global.md"),
            Some("sha256:def456")
        );
    }

    #[test]
    fn save_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let lockfile_path = dir.path().join("nested").join("dir").join(".calvin.lock");

        let repo = TomlLockfileRepository::new();
        let lockfile = Lockfile::new();

        repo.save(&lockfile, &lockfile_path).unwrap();

        assert!(lockfile_path.exists());
    }

    #[test]
    fn toml_format_is_human_readable() {
        let dir = tempdir().unwrap();
        let lockfile_path = dir.path().join(".calvin.lock");

        let repo = TomlLockfileRepository::new();

        let mut lockfile = Lockfile::new();
        lockfile.set("project:test.md", "sha256:abc");

        repo.save(&lockfile, &lockfile_path).unwrap();

        let content = std::fs::read_to_string(&lockfile_path).unwrap();

        // Check TOML structure
        assert!(content.contains("version = 1"));
        assert!(content.contains("[files.\"project:test.md\"]"));
        assert!(content.contains("hash = \"sha256:abc\""));
    }
}
