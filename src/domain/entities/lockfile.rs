//! Lockfile entity - tracks deployed file hashes
//!
//! The lockfile is used to detect external modifications to deployed files.
//! It's a pure data structure - I/O operations are handled by LockfileRepository.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::domain::value_objects::Scope;

/// Normalize a path for lockfile storage (always use forward slashes).
pub(crate) fn normalize_lockfile_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Parse a normalized lockfile path (handle both `/` and `\\` separators).
pub(crate) fn parse_lockfile_path(s: &str) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(s.replace('/', "\\"))
    } else {
        PathBuf::from(s)
    }
}

/// Provenance metadata for a generated output file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputProvenance {
    source_layer: String,
    source_layer_path: PathBuf,
    source_asset: String,
    source_file: PathBuf,
    overrides: Option<String>,
}

impl OutputProvenance {
    pub fn new(
        source_layer: impl Into<String>,
        source_layer_path: impl Into<PathBuf>,
        source_asset: impl Into<String>,
        source_file: impl Into<PathBuf>,
    ) -> Self {
        Self {
            source_layer: source_layer.into(),
            source_layer_path: source_layer_path.into(),
            source_asset: source_asset.into(),
            source_file: source_file.into(),
            overrides: None,
        }
    }

    pub fn with_overrides(mut self, overrides: impl Into<String>) -> Self {
        self.overrides = Some(overrides.into());
        self
    }

    pub fn source_layer(&self) -> &str {
        &self.source_layer
    }

    pub fn source_layer_path(&self) -> &Path {
        &self.source_layer_path
    }

    pub fn source_asset(&self) -> &str {
        &self.source_asset
    }

    pub fn source_file(&self) -> &Path {
        &self.source_file
    }

    pub fn overrides(&self) -> Option<&str> {
        self.overrides.as_deref()
    }
}

/// Lockfile entry for a tracked file
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockfileEntry {
    /// SHA-256 hash of file content
    hash: String,
    /// Source layer name (e.g., "user", "project")
    source_layer: Option<String>,
    /// Source layer root path (normalized for lockfile storage)
    source_layer_path: Option<PathBuf>,
    /// Source asset ID
    source_asset: Option<String>,
    /// Source asset file path (normalized for lockfile storage)
    source_file: Option<PathBuf>,
    /// Overrides applied (if any)
    overrides: Option<String>,
    /// Whether this is a binary file (for skills with binary assets)
    is_binary: bool,
}

impl LockfileEntry {
    /// Create a new entry with the given hash
    pub fn new(hash: impl Into<String>) -> Self {
        Self {
            hash: hash.into(),
            source_layer: None,
            source_layer_path: None,
            source_asset: None,
            source_file: None,
            overrides: None,
            is_binary: false,
        }
    }

    pub fn with_provenance(hash: impl Into<String>, provenance: OutputProvenance) -> Self {
        Self::with_parts(
            hash,
            Some(provenance.source_layer),
            Some(provenance.source_layer_path),
            Some(provenance.source_asset),
            Some(provenance.source_file),
            provenance.overrides,
        )
    }

    pub fn with_parts(
        hash: impl Into<String>,
        source_layer: Option<String>,
        source_layer_path: Option<PathBuf>,
        source_asset: Option<String>,
        source_file: Option<PathBuf>,
        overrides: Option<String>,
    ) -> Self {
        Self {
            hash: hash.into(),
            source_layer,
            source_layer_path,
            source_asset,
            source_file,
            overrides,
            is_binary: false,
        }
    }

    /// Mark this entry as a binary file
    pub fn with_binary(mut self, is_binary: bool) -> Self {
        self.is_binary = is_binary;
        self
    }

    /// Get the hash
    pub fn hash(&self) -> &str {
        &self.hash
    }

    pub fn source_layer(&self) -> Option<&str> {
        self.source_layer.as_deref()
    }

    pub fn source_layer_path(&self) -> Option<&Path> {
        self.source_layer_path.as_deref()
    }

    pub fn source_asset(&self) -> Option<&str> {
        self.source_asset.as_deref()
    }

    pub fn source_file(&self) -> Option<&Path> {
        self.source_file.as_deref()
    }

    pub fn overrides(&self) -> Option<&str> {
        self.overrides.as_deref()
    }

    /// Check if this entry is a binary file
    pub fn is_binary(&self) -> bool {
        self.is_binary
    }
}

/// The lockfile tracks deployed file hashes
///
/// Keys are formatted as `{namespace}:{path}` where namespace is "home" or "project".
/// This allows a single lockfile to track multiple deployment scopes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Lockfile {
    /// Format version
    version: u32,
    /// Map of keys to entries
    entries: BTreeMap<String, LockfileEntry>,
}

impl Lockfile {
    /// Create a new empty lockfile
    pub fn new() -> Self {
        Self {
            version: 1,
            entries: BTreeMap::new(),
        }
    }

    /// Get the version
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Check if the lockfile is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Generate a lockfile key from scope and path
    ///
    /// This must match the logic in `sync/lockfile.rs::lockfile_key` for compatibility:
    /// - Paths starting with `~` always use `home:` prefix (regardless of scope)
    /// - User scope paths that don't start with `~` get `~/` prepended
    /// - Backslashes are normalized to forward slashes for cross-platform consistency
    pub fn make_key(scope: Scope, path: &str) -> String {
        // Normalize path separators for cross-platform consistency (Windows uses \)
        let path = path.replace('\\', "/");

        // Paths starting with ~ always use home: prefix (even if scope is Project)
        if path == "~" || path.starts_with("~/") {
            return format!("home:{}", path);
        }

        match scope {
            Scope::Project => format!("project:{}", path),
            Scope::User => format!("home:~/{}", path),
        }
    }

    /// Parse a lockfile key into scope and path
    pub fn parse_key(key: &str) -> Option<(Scope, &str)> {
        if let Some(path) = key.strip_prefix("project:") {
            Some((Scope::Project, path))
        } else if let Some(path) = key.strip_prefix("home:") {
            Some((Scope::User, path))
        } else {
            None
        }
    }

    /// Get an entry by key
    pub fn get(&self, key: &str) -> Option<&LockfileEntry> {
        self.entries.get(key)
    }

    /// Get hash by key
    pub fn get_hash(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(|e| e.hash())
    }

    /// Set an entry
    pub fn set(&mut self, key: impl Into<String>, hash: impl Into<String>) {
        self.entries
            .insert(key.into(), LockfileEntry::new(hash.into()));
    }

    pub fn set_entry(&mut self, key: impl Into<String>, entry: LockfileEntry) {
        self.entries.insert(key.into(), entry);
    }

    pub fn set_with_provenance(
        &mut self,
        key: impl Into<String>,
        hash: impl Into<String>,
        provenance: OutputProvenance,
    ) {
        self.set_entry(key, LockfileEntry::with_provenance(hash, provenance));
    }

    /// Set an entry by hash (alias for `set()` for sync module compatibility)
    pub fn set_hash(&mut self, key: impl Into<String>, hash: impl Into<String>) {
        self.set(key, hash);
    }

    /// Check if a key exists
    pub fn contains(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Remove an entry
    pub fn remove(&mut self, key: &str) -> Option<LockfileEntry> {
        self.entries.remove(key)
    }

    /// Get all keys
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(|s| s.as_str())
    }

    /// Get all entries
    pub fn entries(&self) -> impl Iterator<Item = (&str, &LockfileEntry)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Get keys matching a scope
    pub fn keys_for_scope(&self, scope: Scope) -> impl Iterator<Item = &str> {
        let prefix = match scope {
            Scope::Project => "project:",
            Scope::User => "home:",
        };
        self.entries
            .keys()
            .filter(move |k| k.starts_with(prefix))
            .map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests;
