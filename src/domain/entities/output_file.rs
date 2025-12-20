//! OutputFile entity - a compiled output file
//!
//! OutputFiles are the result of compiling Assets for a specific target platform.
//! They represent what will be written to the file system.

use crate::domain::value_objects::Target;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

/// A compiled output file ready to be written
#[derive(Debug, Clone, PartialEq)]
pub struct OutputFile {
    /// Path where this file should be written (relative to deploy target)
    path: PathBuf,
    /// Compiled content
    content: String,
    /// Target platform this was compiled for
    target: Target,
    /// Cached content hash
    hash: Option<String>,
}

impl OutputFile {
    /// Create a new OutputFile
    pub fn new(path: impl Into<PathBuf>, content: impl Into<String>, target: Target) -> Self {
        Self {
            path: path.into(),
            content: content.into(),
            target,
            hash: None,
        }
    }

    /// Create an OutputFile without specifying target (for legacy compatibility)
    pub fn new_simple(path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content: content.into(),
            target: Target::All, // Default, should be overridden
            hash: None,
        }
    }

    /// Get the output path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Get the content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the target
    pub fn target(&self) -> Target {
        self.target
    }

    /// Compute and cache the content hash (SHA256)
    pub fn hash(&mut self) -> &str {
        if self.hash.is_none() {
            self.hash = Some(self.compute_hash());
        }
        self.hash.as_ref().unwrap()
    }

    /// Get hash if already computed
    pub fn cached_hash(&self) -> Option<&str> {
        self.hash.as_deref()
    }

    /// Compute SHA256 hash of content
    fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.content.as_bytes());
        format!("sha256:{:x}", hasher.finalize())
    }

    /// Check if content is empty
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Get content length in bytes
    pub fn len(&self) -> usize {
        self.content.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === TDD: OutputFile Creation ===

    #[test]
    fn output_file_new_stores_path_content_target() {
        let output = OutputFile::new(
            ".claude/rules/test.md",
            "# Test\n\nContent",
            Target::ClaudeCode,
        );

        assert_eq!(output.path(), &PathBuf::from(".claude/rules/test.md"));
        assert_eq!(output.content(), "# Test\n\nContent");
        assert_eq!(output.target(), Target::ClaudeCode);
    }

    #[test]
    fn output_file_new_simple_uses_all_target() {
        let output = OutputFile::new_simple("test.md", "content");

        assert_eq!(output.target(), Target::All);
    }

    // === TDD: Hash ===

    #[test]
    fn output_file_hash_computes_sha256() {
        let mut output = OutputFile::new("test.md", "hello", Target::Cursor);

        let hash = output.hash();

        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 7 + 64); // "sha256:" + 64 hex chars
    }

    #[test]
    fn output_file_hash_is_cached() {
        let mut output = OutputFile::new("test.md", "hello", Target::Cursor);

        // First call computes
        let hash1 = output.hash().to_string();
        // Second call uses cache
        let hash2 = output.hash().to_string();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn output_file_cached_hash_none_before_compute() {
        let output = OutputFile::new("test.md", "hello", Target::Cursor);

        assert!(output.cached_hash().is_none());
    }

    #[test]
    fn output_file_cached_hash_some_after_compute() {
        let mut output = OutputFile::new("test.md", "hello", Target::Cursor);
        let _ = output.hash();

        assert!(output.cached_hash().is_some());
    }

    #[test]
    fn output_file_hash_deterministic() {
        let mut a = OutputFile::new("a.md", "same content", Target::ClaudeCode);
        let mut b = OutputFile::new("b.md", "same content", Target::Cursor);

        // Same content = same hash (regardless of path or target)
        assert_eq!(a.hash(), b.hash());
    }

    #[test]
    fn output_file_hash_different_for_different_content() {
        let mut a = OutputFile::new("test.md", "content a", Target::ClaudeCode);
        let mut b = OutputFile::new("test.md", "content b", Target::ClaudeCode);

        assert_ne!(a.hash(), b.hash());
    }

    // === TDD: Utility Methods ===

    #[test]
    fn output_file_is_empty() {
        let empty = OutputFile::new("test.md", "", Target::Cursor);
        let not_empty = OutputFile::new("test.md", "content", Target::Cursor);

        assert!(empty.is_empty());
        assert!(!not_empty.is_empty());
    }

    #[test]
    fn output_file_len() {
        let output = OutputFile::new("test.md", "hello", Target::Cursor);

        assert_eq!(output.len(), 5);
    }

    // === TDD: From<adapters::OutputFile> ===

    #[test]
    fn output_file_from_legacy() {
        use crate::adapters::OutputFile as LegacyOutputFile;

        let legacy = LegacyOutputFile::new("path/to/file.md", "content");
        let domain = OutputFile::from(legacy);

        assert_eq!(domain.path(), &PathBuf::from("path/to/file.md"));
        assert_eq!(domain.content(), "content");
        assert_eq!(domain.target(), Target::All); // Default when not specified
    }
}

// === From implementations ===

impl From<crate::adapters::OutputFile> for OutputFile {
    fn from(legacy: crate::adapters::OutputFile) -> Self {
        OutputFile::new_simple(legacy.path, legacy.content)
    }
}
