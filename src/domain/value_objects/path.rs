//! Safe Path Value Object
//!
//! A validated path that ensures security constraints:
//! - No path traversal attacks (../)
//! - Stays within project boundary
//! - Normalized and canonical

use std::fmt;
use std::path::{Path, PathBuf};

/// Error when path validation fails
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
    /// Path contains traversal components (..)
    ContainsTraversal,
    /// Path escapes the root boundary
    EscapesBoundary { path: PathBuf, root: PathBuf },
    /// Path is absolute when relative is required
    AbsoluteNotAllowed,
    /// Path is empty
    Empty,
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathError::ContainsTraversal => {
                write!(f, "Path contains traversal components (..)")
            }
            PathError::EscapesBoundary { path, root } => {
                write!(
                    f,
                    "Path '{}' escapes boundary '{}'",
                    path.display(),
                    root.display()
                )
            }
            PathError::AbsoluteNotAllowed => {
                write!(f, "Absolute paths are not allowed")
            }
            PathError::Empty => {
                write!(f, "Path is empty")
            }
        }
    }
}

impl std::error::Error for PathError {}

/// A validated safe path
///
/// This value object ensures:
/// - Path is relative (no leading /)
/// - No traversal attacks (..)
/// - Non-empty
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SafePath(PathBuf);

impl SafePath {
    /// Create a new SafePath after validation
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, PathError> {
        let path = path.as_ref();

        // Check empty
        if path.as_os_str().is_empty() {
            return Err(PathError::Empty);
        }

        // Check absolute
        if path.is_absolute() {
            return Err(PathError::AbsoluteNotAllowed);
        }

        // Check for traversal
        for component in path.components() {
            use std::path::Component;
            if matches!(component, Component::ParentDir) {
                return Err(PathError::ContainsTraversal);
            }
        }

        Ok(Self(path.to_path_buf()))
    }

    /// Create from a path within a root, ensuring it doesn't escape
    pub fn within_root<P: AsRef<Path>, R: AsRef<Path>>(
        path: P,
        root: R,
    ) -> Result<Self, PathError> {
        let path = path.as_ref();
        let root = root.as_ref();

        // First, basic validation
        let safe = Self::new(path)?;

        // Then check if joining with root stays within root
        let full_path = root.join(&safe.0);
        if let (Ok(canonical_root), Ok(canonical_full)) =
            (root.canonicalize(), full_path.canonicalize())
        {
            if !canonical_full.starts_with(&canonical_root) {
                return Err(PathError::EscapesBoundary {
                    path: path.to_path_buf(),
                    root: root.to_path_buf(),
                });
            }
        }

        Ok(safe)
    }

    /// Create without validation (for internal use only)
    ///
    /// # Safety
    /// Caller must ensure the path is safe
    #[allow(dead_code)] // Available for internal use
    pub(crate) fn new_unchecked<P: AsRef<Path>>(path: P) -> Self {
        Self(path.as_ref().to_path_buf())
    }

    /// Get the inner path
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// Convert to PathBuf
    pub fn into_path_buf(self) -> PathBuf {
        self.0
    }

    /// Get the file name
    pub fn file_name(&self) -> Option<&std::ffi::OsStr> {
        self.0.file_name()
    }

    /// Get the file stem (name without extension)
    pub fn file_stem(&self) -> Option<&std::ffi::OsStr> {
        self.0.file_stem()
    }

    /// Get the extension
    pub fn extension(&self) -> Option<&std::ffi::OsStr> {
        self.0.extension()
    }

    /// Join with another path component
    pub fn join<P: AsRef<Path>>(&self, path: P) -> Result<SafePath, PathError> {
        SafePath::new(self.0.join(path))
    }
}

impl fmt::Display for SafePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

impl AsRef<Path> for SafePath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl TryFrom<PathBuf> for SafePath {
    type Error = PathError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for SafePath {
    type Error = PathError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_relative_path() {
        let path = SafePath::new("src/main.rs").unwrap();
        assert_eq!(path.as_path(), Path::new("src/main.rs"));
    }

    #[test]
    fn nested_path() {
        let path = SafePath::new("a/b/c/d.txt").unwrap();
        assert_eq!(path.as_path(), Path::new("a/b/c/d.txt"));
    }

    #[test]
    fn rejects_empty() {
        let result = SafePath::new("");
        assert!(matches!(result, Err(PathError::Empty)));
    }

    #[test]
    fn rejects_traversal() {
        let result = SafePath::new("../escape");
        assert!(matches!(result, Err(PathError::ContainsTraversal)));
    }

    #[test]
    fn rejects_hidden_traversal() {
        let result = SafePath::new("a/b/../../../escape");
        assert!(matches!(result, Err(PathError::ContainsTraversal)));
    }

    #[test]
    fn rejects_absolute() {
        // Use platform-appropriate absolute paths
        #[cfg(windows)]
        let absolute_path = "C:\\Windows\\System32";
        #[cfg(not(windows))]
        let absolute_path = "/etc/passwd";

        let result = SafePath::new(absolute_path);
        assert!(matches!(result, Err(PathError::AbsoluteNotAllowed)));
    }

    #[test]
    fn file_name_works() {
        let path = SafePath::new("dir/file.txt").unwrap();
        assert_eq!(path.file_name().unwrap(), "file.txt");
    }

    #[test]
    fn file_stem_works() {
        let path = SafePath::new("dir/file.txt").unwrap();
        assert_eq!(path.file_stem().unwrap(), "file");
    }

    #[test]
    fn extension_works() {
        let path = SafePath::new("dir/file.txt").unwrap();
        assert_eq!(path.extension().unwrap(), "txt");
    }

    #[test]
    fn join_valid() {
        let path = SafePath::new("dir").unwrap();
        let joined = path.join("file.txt").unwrap();
        assert_eq!(joined.as_path(), Path::new("dir/file.txt"));
    }

    #[test]
    fn join_rejects_traversal() {
        let path = SafePath::new("dir").unwrap();
        let result = path.join("../escape");
        assert!(matches!(result, Err(PathError::ContainsTraversal)));
    }

    #[test]
    fn display_works() {
        let path = SafePath::new("src/lib.rs").unwrap();
        assert_eq!(format!("{}", path), "src/lib.rs");
    }

    #[test]
    fn try_from_pathbuf() {
        let pb = PathBuf::from("test.md");
        let path: SafePath = pb.try_into().unwrap();
        assert_eq!(path.as_path(), Path::new("test.md"));
    }

    #[test]
    fn try_from_str() {
        let path: SafePath = "test.md".try_into().unwrap();
        assert_eq!(path.as_path(), Path::new("test.md"));
    }
}
