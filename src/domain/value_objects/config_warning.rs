//! Configuration warning value object.

use std::path::PathBuf;

/// Non-fatal configuration warning surfaced to CLI users.
///
/// This is a domain value object representing a warning that occurred
/// during configuration loading (e.g., unknown keys in config file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigWarning {
    /// The unknown or problematic key
    pub key: String,
    /// The file where the warning occurred
    pub file: PathBuf,
    /// The line number (1-indexed) if available
    pub line: Option<usize>,
    /// A suggested correction if available
    pub suggestion: Option<String>,
}
