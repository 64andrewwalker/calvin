//! Lockfile entity - tracks deployed file hashes
//!
//! The lockfile is used to detect external modifications to deployed files.
//! It's a pure data structure - I/O operations are handled by LockfileRepository.

use std::collections::BTreeMap;

use crate::domain::value_objects::Scope;

/// Lockfile entry for a tracked file
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockfileEntry {
    /// SHA-256 hash of file content
    hash: String,
}

impl LockfileEntry {
    /// Create a new entry with the given hash
    pub fn new(hash: impl Into<String>) -> Self {
        Self { hash: hash.into() }
    }

    /// Get the hash
    pub fn hash(&self) -> &str {
        &self.hash
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
    pub fn make_key(scope: Scope, path: &str) -> String {
        let prefix = match scope {
            Scope::Project => "project:",
            Scope::User => "home:",
        };
        format!("{}{}", prefix, path)
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
mod tests {
    use super::*;

    // === TDD: Lockfile Creation ===

    #[test]
    fn lockfile_new_is_empty() {
        let lockfile = Lockfile::new();

        assert!(lockfile.is_empty());
        assert_eq!(lockfile.len(), 0);
        assert_eq!(lockfile.version(), 1);
    }

    // === TDD: Key Generation ===

    #[test]
    fn lockfile_make_key_project() {
        let key = Lockfile::make_key(Scope::Project, ".claude/rules/test.md");
        assert_eq!(key, "project:.claude/rules/test.md");
    }

    #[test]
    fn lockfile_make_key_user() {
        let key = Lockfile::make_key(Scope::User, "~/.claude/commands/test.md");
        assert_eq!(key, "home:~/.claude/commands/test.md");
    }

    #[test]
    fn lockfile_parse_key_project() {
        let result = Lockfile::parse_key("project:.claude/rules/test.md");
        assert_eq!(result, Some((Scope::Project, ".claude/rules/test.md")));
    }

    #[test]
    fn lockfile_parse_key_user() {
        let result = Lockfile::parse_key("home:~/.claude/commands/test.md");
        assert_eq!(result, Some((Scope::User, "~/.claude/commands/test.md")));
    }

    #[test]
    fn lockfile_parse_key_invalid() {
        assert!(Lockfile::parse_key("invalid:path").is_none());
        assert!(Lockfile::parse_key("no-prefix").is_none());
    }

    // === TDD: Entry Operations ===

    #[test]
    fn lockfile_set_and_get() {
        let mut lockfile = Lockfile::new();
        lockfile.set("project:test.md", "sha256:abc123");

        assert!(!lockfile.is_empty());
        assert_eq!(lockfile.len(), 1);

        let entry = lockfile.get("project:test.md").unwrap();
        assert_eq!(entry.hash(), "sha256:abc123");
    }

    #[test]
    fn lockfile_get_hash() {
        let mut lockfile = Lockfile::new();
        lockfile.set("project:test.md", "sha256:abc123");

        assert_eq!(lockfile.get_hash("project:test.md"), Some("sha256:abc123"));
        assert_eq!(lockfile.get_hash("missing"), None);
    }

    #[test]
    fn lockfile_set_overwrites() {
        let mut lockfile = Lockfile::new();
        lockfile.set("project:test.md", "sha256:old");
        lockfile.set("project:test.md", "sha256:new");

        assert_eq!(lockfile.get_hash("project:test.md"), Some("sha256:new"));
        assert_eq!(lockfile.len(), 1);
    }

    #[test]
    fn lockfile_remove() {
        let mut lockfile = Lockfile::new();
        lockfile.set("project:test.md", "sha256:abc");

        let removed = lockfile.remove("project:test.md");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().hash(), "sha256:abc");
        assert!(lockfile.is_empty());
    }

    #[test]
    fn lockfile_remove_nonexistent() {
        let mut lockfile = Lockfile::new();
        let removed = lockfile.remove("missing");
        assert!(removed.is_none());
    }

    // === TDD: Iteration ===

    #[test]
    fn lockfile_keys() {
        let mut lockfile = Lockfile::new();
        lockfile.set("project:a.md", "hash1");
        lockfile.set("home:b.md", "hash2");

        let keys: Vec<_> = lockfile.keys().collect();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"project:a.md"));
        assert!(keys.contains(&"home:b.md"));
    }

    #[test]
    fn lockfile_keys_for_scope() {
        let mut lockfile = Lockfile::new();
        lockfile.set("project:a.md", "hash1");
        lockfile.set("project:b.md", "hash2");
        lockfile.set("home:c.md", "hash3");

        let project_keys: Vec<_> = lockfile.keys_for_scope(Scope::Project).collect();
        assert_eq!(project_keys.len(), 2);

        let home_keys: Vec<_> = lockfile.keys_for_scope(Scope::User).collect();
        assert_eq!(home_keys.len(), 1);
    }

    #[test]
    fn lockfile_entries() {
        let mut lockfile = Lockfile::new();
        lockfile.set("project:test.md", "sha256:abc");

        let entries: Vec<_> = lockfile.entries().collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "project:test.md");
        assert_eq!(entries[0].1.hash(), "sha256:abc");
    }

    // === TDD: LockfileEntry ===

    #[test]
    fn lockfile_entry_new() {
        let entry = LockfileEntry::new("sha256:abc123");
        assert_eq!(entry.hash(), "sha256:abc123");
    }
}
