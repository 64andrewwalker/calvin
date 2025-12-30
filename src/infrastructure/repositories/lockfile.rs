//! TOML Lockfile Repository
//!
//! Implements the LockfileRepository port using TOML format.

use crate::domain::entities::{
    normalize_lockfile_path, parse_lockfile_path, Lockfile, LockfileEntry,
};
use crate::domain::ports::file_system::FileSystem;
use crate::domain::ports::lockfile_repository::{LockfileError, LockfileRepository};
use crate::infrastructure::fs::LocalFs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// TOML-based lockfile repository
///
/// Stores lockfile as `calvin.lock` in TOML format.
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
    #[serde(default)]
    source_layer: Option<String>,
    #[serde(default)]
    source_layer_path: Option<String>,
    #[serde(default)]
    source_asset: Option<String>,
    #[serde(default)]
    source_file: Option<String>,
    #[serde(default)]
    overrides: Option<String>,
    /// Whether this is a binary file (defaults to false for backwards compatibility)
    #[serde(default, skip_serializing_if = "is_false")]
    is_binary: bool,
}

/// Helper for serde skip_serializing_if
fn is_false(b: &bool) -> bool {
    !*b
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

        let expected_version = Lockfile::new().version();
        if toml_lockfile.version != expected_version {
            return Err(LockfileError::VersionMismatch {
                found: toml_lockfile.version,
                expected: expected_version,
            });
        }

        let mut lockfile = Lockfile::new();
        for (key, entry) in toml_lockfile.files {
            lockfile.set_entry(
                key,
                LockfileEntry::with_parts(
                    entry.hash,
                    entry.source_layer,
                    entry.source_layer_path.map(|p| parse_lockfile_path(&p)),
                    entry.source_asset,
                    entry.source_file.map(|p| parse_lockfile_path(&p)),
                    entry.overrides,
                )
                .with_binary(entry.is_binary),
            );
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
                    source_layer: entry.source_layer().map(|s| s.to_string()),
                    source_layer_path: entry.source_layer_path().map(normalize_lockfile_path),
                    source_asset: entry.source_asset().map(|s| s.to_string()),
                    source_file: entry.source_file().map(normalize_lockfile_path),
                    overrides: entry.overrides().map(|s| s.to_string()),
                    is_binary: entry.is_binary(),
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

    fn delete(&self, path: &Path) -> Result<(), LockfileError> {
        if self.fs.exists(path) {
            self.fs
                .remove(path)
                .map_err(|e| LockfileError::IoError(e.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn load_nonexistent_returns_empty_lockfile() {
        let repo = TomlLockfileRepository::new();
        let lockfile = repo.load(Path::new("/nonexistent/calvin.lock")).unwrap();

        assert!(lockfile.is_empty());
        assert_eq!(lockfile.version(), 1);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let lockfile_path = dir.path().join("calvin.lock");

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
        let lockfile_path = dir.path().join("nested").join("dir").join("calvin.lock");

        let repo = TomlLockfileRepository::new();
        let lockfile = Lockfile::new();

        repo.save(&lockfile, &lockfile_path).unwrap();

        assert!(lockfile_path.exists());
    }

    #[test]
    fn toml_format_is_human_readable() {
        let dir = tempdir().unwrap();
        let lockfile_path = dir.path().join("calvin.lock");

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

    #[test]
    fn deserialize_old_format() {
        let dir = tempdir().unwrap();
        let lockfile_path = dir.path().join("calvin.lock");

        let content = r#"
version = 1

[files."project:test.md"]
hash = "sha256:abc"
"#;
        std::fs::write(&lockfile_path, content).unwrap();

        let repo = TomlLockfileRepository::new();
        let lockfile = repo.load(&lockfile_path).unwrap();

        let entry = lockfile.get("project:test.md").unwrap();
        assert_eq!(entry.hash(), "sha256:abc");
        assert_eq!(entry.source_layer(), None);
        assert_eq!(entry.source_asset(), None);
    }

    #[test]
    fn deserialize_new_format_preserves_provenance() {
        let dir = tempdir().unwrap();
        let lockfile_path = dir.path().join("calvin.lock");

        let content = r#"
version = 1

[files."project:test.md"]
hash = "sha256:abc"
source_layer = "user"
source_layer_path = "~/.calvin/.promptpack"
source_asset = "review"
source_file = "~/.calvin/.promptpack/actions/review.md"
"#;
        std::fs::write(&lockfile_path, content).unwrap();

        let repo = TomlLockfileRepository::new();
        let lockfile = repo.load(&lockfile_path).unwrap();

        let entry = lockfile.get("project:test.md").unwrap();
        assert_eq!(entry.hash(), "sha256:abc");
        assert_eq!(entry.source_layer(), Some("user"));
        assert_eq!(entry.source_asset(), Some("review"));

        let saved_path = dir.path().join("saved.lock");
        repo.save(&lockfile, &saved_path).unwrap();
        let saved = std::fs::read_to_string(&saved_path).unwrap();
        assert!(saved.contains("source_layer = \"user\""));
        assert!(saved.contains("source_asset = \"review\""));
    }

    #[test]
    fn old_reader_ignores_new_fields() {
        #[derive(Deserialize)]
        struct OldTomlFileEntry {
            hash: String,
        }

        #[derive(Deserialize)]
        struct OldTomlLockfile {
            version: u32,
            #[serde(default)]
            files: BTreeMap<String, OldTomlFileEntry>,
        }

        let content = r#"
version = 1

[files."project:test.md"]
hash = "sha256:abc"
source_layer = "user"
source_asset = "review"
"#;

        let parsed: OldTomlLockfile = toml::from_str(content).unwrap();
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.files["project:test.md"].hash, "sha256:abc");
    }

    #[test]
    fn save_normalizes_provenance_paths() {
        let dir = tempdir().unwrap();
        let lockfile_path = dir.path().join("calvin.lock");

        let repo = TomlLockfileRepository::new();
        let mut lockfile = Lockfile::new();
        lockfile.set_entry(
            "project:test.md",
            LockfileEntry::with_parts(
                "sha256:abc",
                Some("user".to_string()),
                Some(PathBuf::from("C:\\Users\\me\\project")),
                Some("review".to_string()),
                Some(PathBuf::from("C:\\Users\\me\\project\\review.md")),
                None,
            ),
        );

        repo.save(&lockfile, &lockfile_path).unwrap();

        let content = std::fs::read_to_string(&lockfile_path).unwrap();
        assert!(content.contains(r#"source_layer_path = "C:/Users/me/project""#));
        assert!(content.contains(r#"source_file = "C:/Users/me/project/review.md""#));
    }

    #[test]
    fn load_errors_on_version_mismatch() {
        let dir = tempdir().unwrap();
        let lockfile_path = dir.path().join("calvin.lock");

        let content = r#"
version = 999

[files."project:test.md"]
hash = "sha256:abc"
"#;
        std::fs::write(&lockfile_path, content).unwrap();

        let repo = TomlLockfileRepository::new();
        let err = repo.load(&lockfile_path).unwrap_err();
        assert!(matches!(err, LockfileError::VersionMismatch { .. }));

        let msg = err.to_string();
        assert!(msg.contains("lockfile format incompatible"));
        assert!(msg.contains("calvin migrate"));
    }
}
