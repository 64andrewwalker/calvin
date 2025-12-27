//! Ignore patterns value object
//!
//! Handles loading and matching `.calvinignore` patterns using gitignore semantics.

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// Maximum file size for `.calvinignore` (64KB)
const MAX_FILE_SIZE: u64 = 65536;

/// Maximum number of patterns allowed
const MAX_PATTERNS: usize = 1000;

/// Patterns loaded from a `.calvinignore` file.
///
/// Uses the `ignore` crate for gitignore-compatible pattern matching.
#[derive(Debug)]
pub struct IgnorePatterns {
    matcher: Gitignore,
    pattern_count: usize,
}

impl Default for IgnorePatterns {
    fn default() -> Self {
        Self::empty()
    }
}

impl IgnorePatterns {
    /// Create an empty pattern set (matches nothing).
    pub fn empty() -> Self {
        // Build an empty matcher
        let builder = GitignoreBuilder::new("");
        let matcher = builder
            .build()
            .expect("empty gitignore should always build");
        Self {
            matcher,
            pattern_count: 0,
        }
    }

    /// Load patterns from a `.calvinignore` file in the given promptpack directory.
    ///
    /// Returns `Ok(empty)` if the file doesn't exist.
    /// Returns `Err` if the file is too large, has too many patterns, or contains invalid syntax.
    pub fn load(promptpack_path: &Path) -> Result<Self, IgnoreError> {
        let ignore_path = promptpack_path.join(".calvinignore");

        if !ignore_path.exists() {
            return Ok(Self::empty());
        }

        // Check file size
        let metadata = fs::metadata(&ignore_path).map_err(IgnoreError::Io)?;
        if metadata.len() > MAX_FILE_SIZE {
            return Err(IgnoreError::FileTooLarge {
                path: ignore_path,
                size: metadata.len(),
                limit: MAX_FILE_SIZE,
            });
        }

        // Read and parse
        let content = fs::read_to_string(&ignore_path).map_err(IgnoreError::Io)?;
        Self::from_content(promptpack_path, &ignore_path, &content)
    }

    /// Parse patterns from string content (for testing).
    pub fn from_content(
        root: &Path,
        source_path: &Path,
        content: &str,
    ) -> Result<Self, IgnoreError> {
        let mut builder = GitignoreBuilder::new(root);
        let mut pattern_count = 0;

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            pattern_count += 1;
            if pattern_count > MAX_PATTERNS {
                return Err(IgnoreError::TooManyPatterns {
                    path: source_path.to_path_buf(),
                    count: pattern_count,
                    limit: MAX_PATTERNS,
                });
            }

            // Add the pattern
            if let Err(e) = builder.add_line(Some(source_path.to_path_buf()), line) {
                return Err(IgnoreError::InvalidPattern {
                    path: source_path.to_path_buf(),
                    line: line_num + 1,
                    pattern: line.to_string(),
                    message: e.to_string(),
                });
            }
        }

        let matcher = builder
            .build()
            .map_err(|e| IgnoreError::BuildFailed(e.to_string()))?;

        Ok(Self {
            matcher,
            pattern_count,
        })
    }

    /// Check if a path should be ignored.
    ///
    /// `is_dir` should be true if the path is a directory.
    pub fn is_ignored(&self, rel_path: &Path, is_dir: bool) -> bool {
        self.matcher
            .matched_path_or_any_parents(rel_path, is_dir)
            .is_ignore()
    }

    /// Get the number of patterns loaded.
    pub fn pattern_count(&self) -> usize {
        self.pattern_count
    }

    /// Check if this is an empty pattern set.
    pub fn is_empty(&self) -> bool {
        self.pattern_count == 0
    }
}

/// Errors that can occur when loading ignore patterns.
#[derive(Debug)]
pub enum IgnoreError {
    /// The `.calvinignore` file exceeds the size limit.
    FileTooLarge {
        path: PathBuf,
        size: u64,
        limit: u64,
    },
    /// Too many patterns in the file.
    TooManyPatterns {
        path: PathBuf,
        count: usize,
        limit: usize,
    },
    /// A pattern has invalid syntax.
    InvalidPattern {
        path: PathBuf,
        line: usize,
        pattern: String,
        message: String,
    },
    /// Failed to build the gitignore matcher.
    BuildFailed(String),
    /// IO error reading the file.
    Io(std::io::Error),
}

impl fmt::Display for IgnoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileTooLarge { path, size, limit } => {
                write!(
                    f,
                    ".calvinignore exceeds {}KB limit ({} bytes): {}",
                    limit / 1024,
                    size,
                    path.display()
                )
            }
            Self::TooManyPatterns { path, count, limit } => {
                write!(
                    f,
                    ".calvinignore has {} patterns, exceeds {} limit: {}",
                    count,
                    limit,
                    path.display()
                )
            }
            Self::InvalidPattern {
                path,
                line,
                pattern,
                message,
            } => {
                write!(
                    f,
                    "Invalid pattern at {}:{}: '{}' - {}",
                    path.display(),
                    line,
                    pattern,
                    message
                )
            }
            Self::BuildFailed(msg) => write!(f, "Failed to build ignore matcher: {}", msg),
            Self::Io(e) => write!(f, "IO error reading .calvinignore: {}", e),
        }
    }
}

impl std::error::Error for IgnoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn empty_patterns_match_nothing() {
        let patterns = IgnorePatterns::empty();
        assert!(!patterns.is_ignored(Path::new("anything.md"), false));
        assert!(!patterns.is_ignored(Path::new("dir/file.md"), false));
        assert_eq!(patterns.pattern_count(), 0);
        assert!(patterns.is_empty());
    }

    #[test]
    fn missing_file_returns_empty() {
        let dir = tempdir().unwrap();
        let patterns = IgnorePatterns::load(dir.path()).unwrap();
        assert!(patterns.is_empty());
    }

    #[test]
    fn empty_file_returns_empty() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(".calvinignore"), "").unwrap();
        let patterns = IgnorePatterns::load(dir.path()).unwrap();
        assert!(patterns.is_empty());
    }

    #[test]
    fn comments_only_returns_empty() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".calvinignore"),
            "# just a comment\n\n# another",
        )
        .unwrap();
        let patterns = IgnorePatterns::load(dir.path()).unwrap();
        assert!(patterns.is_empty());
    }

    #[test]
    fn pattern_matches_exact_file() {
        let patterns = IgnorePatterns::from_content(
            Path::new("/root"),
            Path::new("/root/.calvinignore"),
            "README.md",
        )
        .unwrap();

        assert!(patterns.is_ignored(Path::new("README.md"), false));
        assert!(!patterns.is_ignored(Path::new("other.md"), false));
        assert_eq!(patterns.pattern_count(), 1);
    }

    #[test]
    fn pattern_matches_directory_recursively() {
        let patterns = IgnorePatterns::from_content(
            Path::new("/root"),
            Path::new("/root/.calvinignore"),
            "drafts/",
        )
        .unwrap();

        assert!(patterns.is_ignored(Path::new("drafts"), true));
        assert!(patterns.is_ignored(Path::new("drafts/file.md"), false));
        assert!(patterns.is_ignored(Path::new("drafts/nested/deep.md"), false));
        assert!(!patterns.is_ignored(Path::new("other/file.md"), false));
    }

    #[test]
    fn glob_pattern_matches() {
        let patterns = IgnorePatterns::from_content(
            Path::new("/root"),
            Path::new("/root/.calvinignore"),
            "*.bak",
        )
        .unwrap();

        assert!(patterns.is_ignored(Path::new("file.bak"), false));
        assert!(patterns.is_ignored(Path::new("dir/other.bak"), false));
        assert!(!patterns.is_ignored(Path::new("file.md"), false));
    }

    #[test]
    fn double_star_matches_any_depth() {
        let patterns = IgnorePatterns::from_content(
            Path::new("/root"),
            Path::new("/root/.calvinignore"),
            "**/test-*.md",
        )
        .unwrap();

        assert!(patterns.is_ignored(Path::new("test-foo.md"), false));
        assert!(patterns.is_ignored(Path::new("a/test-bar.md"), false));
        assert!(patterns.is_ignored(Path::new("a/b/c/test-baz.md"), false));
        assert!(!patterns.is_ignored(Path::new("foo.md"), false));
    }

    #[test]
    fn negation_re_includes_file() {
        let patterns = IgnorePatterns::from_content(
            Path::new("/root"),
            Path::new("/root/.calvinignore"),
            "*.md\n!important.md",
        )
        .unwrap();

        assert!(patterns.is_ignored(Path::new("test.md"), false));
        assert!(patterns.is_ignored(Path::new("other.md"), false));
        assert!(!patterns.is_ignored(Path::new("important.md"), false));
    }

    #[test]
    fn file_too_large_error() {
        let dir = tempdir().unwrap();
        // Create a file larger than 64KB
        let large_content = "x\n".repeat(40000); // ~80KB
        fs::write(dir.path().join(".calvinignore"), large_content).unwrap();

        let result = IgnorePatterns::load(dir.path());
        assert!(matches!(result, Err(IgnoreError::FileTooLarge { .. })));
    }

    #[test]
    fn too_many_patterns_error() {
        let patterns: String = (0..1100).map(|i| format!("file{}.md\n", i)).collect();
        let result = IgnorePatterns::from_content(
            Path::new("/root"),
            Path::new("/root/.calvinignore"),
            &patterns,
        );
        assert!(matches!(result, Err(IgnoreError::TooManyPatterns { .. })));
    }

    #[test]
    fn multiple_patterns_work() {
        let patterns = IgnorePatterns::from_content(
            Path::new("/root"),
            Path::new("/root/.calvinignore"),
            "drafts/\n*.bak\nREADME.md",
        )
        .unwrap();

        assert_eq!(patterns.pattern_count(), 3);
        assert!(patterns.is_ignored(Path::new("drafts/wip.md"), false));
        assert!(patterns.is_ignored(Path::new("old.bak"), false));
        assert!(patterns.is_ignored(Path::new("README.md"), false));
        assert!(!patterns.is_ignored(Path::new("policy.md"), false));
    }
}
