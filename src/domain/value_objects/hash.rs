//! Content Hash Value Object
//!
//! A validated, immutable hash representing the content of a file.
//! Used for change detection in the lockfile system.

use std::fmt;

/// Content hash value object
///
/// Wraps a SHA-256 hash string with the `sha256:` prefix.
/// This is an immutable value object that ensures hash format consistency.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentHash(String);

impl ContentHash {
    /// Prefix for SHA-256 hashes
    pub const PREFIX: &'static str = "sha256:";

    /// Create a new ContentHash from a raw hash string (without prefix)
    pub fn new(raw_hash: &str) -> Self {
        if raw_hash.starts_with(Self::PREFIX) {
            Self(raw_hash.to_string())
        } else {
            Self(format!("{}{}", Self::PREFIX, raw_hash))
        }
    }

    /// Create a ContentHash by computing SHA-256 of content
    pub fn from_content(content: &str) -> Self {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(content.as_bytes());
        Self(format!("{}:{:x}", Self::PREFIX.trim_end_matches(':'), hash))
    }

    /// Get the full hash string with prefix
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get just the hex part without prefix
    pub fn hex(&self) -> &str {
        self.0.strip_prefix(Self::PREFIX).unwrap_or(&self.0)
    }

    /// Check if this hash matches another
    pub fn matches(&self, other: &ContentHash) -> bool {
        self.0 == other.0
    }

    /// Check if this hash matches a raw string (with or without prefix)
    pub fn matches_str(&self, s: &str) -> bool {
        if s.starts_with(Self::PREFIX) {
            self.0 == s
        } else {
            self.hex() == s
        }
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ContentHash {
    fn from(s: String) -> Self {
        Self::new(&s)
    }
}

impl From<&str> for ContentHash {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl AsRef<str> for ContentHash {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_adds_prefix_if_missing() {
        let hash = ContentHash::new("abc123");
        assert_eq!(hash.as_str(), "sha256:abc123");
    }

    #[test]
    fn new_keeps_prefix_if_present() {
        let hash = ContentHash::new("sha256:abc123");
        assert_eq!(hash.as_str(), "sha256:abc123");
    }

    #[test]
    fn from_content_computes_sha256() {
        let hash = ContentHash::from_content("hello");
        assert!(hash.as_str().starts_with("sha256:"));
        assert_eq!(hash.hex().len(), 64); // SHA-256 is 64 hex chars
    }

    #[test]
    fn same_content_same_hash() {
        let h1 = ContentHash::from_content("test");
        let h2 = ContentHash::from_content("test");
        assert!(h1.matches(&h2));
    }

    #[test]
    fn different_content_different_hash() {
        let h1 = ContentHash::from_content("test1");
        let h2 = ContentHash::from_content("test2");
        assert!(!h1.matches(&h2));
    }

    #[test]
    fn hex_returns_without_prefix() {
        let hash = ContentHash::new("abc123");
        assert_eq!(hash.hex(), "abc123");
    }

    #[test]
    fn matches_str_with_prefix() {
        let hash = ContentHash::new("abc123");
        assert!(hash.matches_str("sha256:abc123"));
    }

    #[test]
    fn matches_str_without_prefix() {
        let hash = ContentHash::new("abc123");
        assert!(hash.matches_str("abc123"));
    }

    #[test]
    fn display_shows_full_hash() {
        let hash = ContentHash::new("abc123");
        assert_eq!(format!("{}", hash), "sha256:abc123");
    }

    #[test]
    fn from_string() {
        let hash: ContentHash = "abc123".into();
        assert_eq!(hash.as_str(), "sha256:abc123");
    }

    #[test]
    fn eq_works_correctly() {
        let h1 = ContentHash::new("abc");
        let h2 = ContentHash::new("abc");
        let h3 = ContentHash::new("def");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }
}
