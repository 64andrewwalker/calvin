//! Error types for Calvin
//!
//! Uses `thiserror` for library errors following TD-10.
//!
//! Error messages include:
//! - Clear description of what went wrong
//! - Suggestion for how to fix it (when possible)
//! - Link to relevant documentation
//!
//! **Note**: Documentation URLs are centralized in `src/docs.rs`.
//! If the documentation site moves, update `docs::DOCS_BASE_URL`.

use crate::config::levenshtein;
use crate::docs;
use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for Calvin operations
pub type CalvinResult<T> = Result<T, CalvinError>;

/// Main error type for Calvin operations
#[derive(Error, Debug)]
pub enum CalvinError {
    /// Missing required field in frontmatter
    #[error("missing required field '{field}' in {file}:{line}\n  → Fix: Add '{field}: <value>' to frontmatter\n  → Docs: https://64andrewwalker.github.io/calvin/docs/api/frontmatter")]
    MissingField {
        field: String,
        file: PathBuf,
        line: usize,
    },

    /// Invalid frontmatter YAML
    #[error("invalid frontmatter in {file}: {message}")]
    InvalidFrontmatter { file: PathBuf, message: String },

    /// No frontmatter found (missing `---` delimiters)
    #[error("no frontmatter found in {file}\n  → Fix: File must start with '---' delimiter\n  → Example:\n    ---\n    description: My asset\n    ---\n  → Docs: https://64andrewwalker.github.io/calvin/docs/api/frontmatter")]
    NoFrontmatter { file: PathBuf },

    /// Frontmatter not properly closed
    #[error("unclosed frontmatter in {file}\n  → Fix: Add closing '---' after frontmatter YAML\n  → Docs: https://64andrewwalker.github.io/calvin/docs/api/frontmatter")]
    UnclosedFrontmatter { file: PathBuf },

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// YAML parsing error
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),

    /// Directory not found
    #[error("directory not found: {path}\n  → Fix: Create the directory or check the path\n  → Run: mkdir -p {path}")]
    DirectoryNotFound { path: PathBuf },

    /// Invalid asset kind
    #[error("{}", format_invalid_asset_kind(kind, file))]
    InvalidAssetKind { kind: String, file: PathBuf },

    /// Path escapes project boundary (security issue)
    #[error("path '{path}' escapes project boundary '{root}'\n  → Fix: Use relative paths within the project\n  → Security: Path traversal (../) is not allowed")]
    PathEscape { path: PathBuf, root: PathBuf },

    /// Sync was aborted by user in interactive mode
    #[error("sync aborted by user")]
    SyncAborted,

    /// Compilation error
    #[error("compile error: {message}")]
    Compile { message: String },

    /// File system error (from domain::ports::file_system)
    #[error("file system error: {0}")]
    FileSystem(String),

    /// Configuration security violation (project config attempted forbidden settings)
    #[error("config security violation in {file}: {message}\n  → Security: Project config may only disable layers\n  → Fix: Move layer paths to user config (~/.config/calvin/config.toml)")]
    ConfigSecurityViolation { file: PathBuf, message: String },

    /// No promptpack layers found (multi-layer)
    #[error(
        "no promptpack layers found\n  → Fix: Create a .promptpack/ directory or configure user layer\n  → Run: calvin init --user\n  → Docs: https://64andrewwalker.github.io/calvin/docs/guides/multi-layer"
    )]
    NoLayersFound,

    /// Registry file corrupted (multi-layer)
    #[error(
        "registry file corrupted: {path}\n  → Fix: Delete and rebuild registry\n  → Run: rm {path} && calvin deploy"
    )]
    RegistryCorrupted { path: PathBuf },
}

use std::path::Path;

/// Valid asset kinds for suggestions
const VALID_ASSET_KINDS: &[&str] = &["policy", "action", "agent"];

/// Format InvalidAssetKind error with suggestion
fn format_invalid_asset_kind(kind: &str, file: &Path) -> String {
    let mut msg = format!("invalid asset kind '{}' in {}", kind, file.display());

    // Try to suggest a correction using Levenshtein distance
    let input = kind.to_lowercase();
    let mut best: Option<(&str, usize)> = None;

    for &valid in VALID_ASSET_KINDS {
        let dist = levenshtein(&input, valid);
        match best {
            None => best = Some((valid, dist)),
            Some((_, best_dist)) if dist < best_dist => best = Some((valid, dist)),
            _ => {}
        }
    }

    // Suggest if close match found
    if let Some((suggested, dist)) = best {
        if dist <= 2 && dist > 0 {
            msg.push_str(&format!("\n  → Did you mean '{}'?", suggested));
        }
    }

    msg.push_str("\n  → Valid kinds: policy, action, agent");
    msg.push_str(&format!("\n  → Docs: {}", docs::frontmatter_kind_url()));

    msg
}

impl From<crate::domain::ports::file_system::FsError> for CalvinError {
    fn from(err: crate::domain::ports::file_system::FsError) -> Self {
        CalvinError::FileSystem(err.to_string())
    }
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
        let msg = err.to_string();
        assert!(msg.contains("missing required field 'description'"));
        assert!(msg.contains("policies/test.md:2"));
        assert!(msg.contains("Fix:"), "Should include fix suggestion");
        assert!(msg.contains("Docs:"), "Should include docs link");
    }

    #[test]
    fn test_error_display_no_frontmatter() {
        let err = CalvinError::NoFrontmatter {
            file: PathBuf::from("actions/test.md"),
        };
        let msg = err.to_string();
        assert!(msg.contains("no frontmatter found"));
        assert!(msg.contains("actions/test.md"));
        assert!(msg.contains("Fix:"), "Should include fix suggestion");
        assert!(msg.contains("---"), "Should show example delimiter");
    }

    #[test]
    fn test_error_display_invalid_asset_kind_with_suggestion() {
        let err = CalvinError::InvalidAssetKind {
            kind: "polcy".to_string(), // typo
            file: PathBuf::from("test.md"),
        };
        let msg = err.to_string();
        assert!(msg.contains("invalid asset kind 'polcy'"));
        assert!(
            msg.contains("Did you mean 'policy'?"),
            "Should suggest correction: {}",
            msg
        );
        assert!(msg.contains("Valid kinds:"), "Should list valid kinds");
    }

    #[test]
    fn test_error_display_invalid_asset_kind_no_suggestion_for_distant() {
        let err = CalvinError::InvalidAssetKind {
            kind: "something_random".to_string(),
            file: PathBuf::from("test.md"),
        };
        let msg = err.to_string();
        assert!(msg.contains("invalid asset kind"));
        assert!(
            !msg.contains("Did you mean"),
            "Should not suggest for distant values"
        );
        assert!(msg.contains("Valid kinds:"));
    }

    #[test]
    fn test_error_display_directory_not_found() {
        let err = CalvinError::DirectoryNotFound {
            path: PathBuf::from(".promptpack"),
        };
        let msg = err.to_string();
        assert!(msg.contains("directory not found"));
        assert!(msg.contains(".promptpack"));
        assert!(msg.contains("mkdir"), "Should suggest fix command");
    }

    #[test]
    fn test_error_display_path_escape() {
        let err = CalvinError::PathEscape {
            path: PathBuf::from("../etc/passwd"),
            root: PathBuf::from("/home/user/project"),
        };
        let msg = err.to_string();
        assert!(msg.contains("escapes project boundary"));
        assert!(msg.contains("Security:"), "Should mention security");
    }

    #[test]
    fn test_error_display_unclosed_frontmatter() {
        let err = CalvinError::UnclosedFrontmatter {
            file: PathBuf::from("test.md"),
        };
        let msg = err.to_string();
        assert!(msg.contains("unclosed frontmatter"));
        assert!(msg.contains("Fix:"));
        assert!(msg.contains("---"), "Should mention closing delimiter");
    }

    #[test]
    fn test_error_display_no_layers_found() {
        let err = CalvinError::NoLayersFound;
        let msg = err.to_string();
        assert!(msg.contains("no promptpack layers found"));
        assert!(msg.contains("calvin init --user"));
    }

    #[test]
    fn test_error_display_registry_corrupted() {
        let err = CalvinError::RegistryCorrupted {
            path: PathBuf::from("~/.calvin/registry.toml"),
        };
        let msg = err.to_string();
        assert!(msg.contains("registry file corrupted"));
        assert!(msg.contains("rm"));
        assert!(msg.contains("calvin deploy"));
    }
}
