//! Error types for Calvin
//!
//! Uses `thiserror` for library errors following TD-10.

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for Calvin operations
pub type CalvinResult<T> = Result<T, CalvinError>;

/// Main error type for Calvin operations
#[derive(Error, Debug)]
pub enum CalvinError {
    /// Missing required field in frontmatter
    #[error("missing required field '{field}' in {file}:{line}")]
    MissingField {
        field: String,
        file: PathBuf,
        line: usize,
    },

    /// Invalid frontmatter YAML
    #[error("invalid frontmatter in {file}: {message}")]
    InvalidFrontmatter { file: PathBuf, message: String },

    /// No frontmatter found (missing `---` delimiters)
    #[error("no frontmatter found in {file} - file must start with '---'")]
    NoFrontmatter { file: PathBuf },

    /// Frontmatter not properly closed
    #[error("unclosed frontmatter in {file} - missing closing '---'")]
    UnclosedFrontmatter { file: PathBuf },

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// YAML parsing error
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// Directory not found
    #[error("directory not found: {path}")]
    DirectoryNotFound { path: PathBuf },

    /// Invalid asset kind
    #[error("invalid asset kind '{kind}' in {file}")]
    InvalidAssetKind { kind: String, file: PathBuf },

    /// Path escapes project boundary (security issue)
    #[error("path '{path}' escapes project boundary '{root}'")]
    PathEscape { path: PathBuf, root: PathBuf },

    /// Sync was aborted by user in interactive mode
    #[error("sync aborted by user")]
    SyncAborted,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_error_display_missing_field() {
        let err = CalvinError::MissingField {
            field: "description".to_string(),
            file: PathBuf::from("policies/test.md"),
            line: 2,
        };
        assert_eq!(
            err.to_string(),
            "missing required field 'description' in policies/test.md:2"
        );
    }

    #[test]
    fn test_error_display_no_frontmatter() {
        let err = CalvinError::NoFrontmatter {
            file: PathBuf::from("actions/test.md"),
        };
        assert_eq!(
            err.to_string(),
            "no frontmatter found in actions/test.md - file must start with '---'"
        );
    }
}
