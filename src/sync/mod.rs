//! Sync engine for writing compiled outputs
//!
//! **Note**: This module is being progressively migrated to the new layered architecture.
//!
//! New code location:
//! - `domain::services::planner` - Planning logic
//! - `domain::services::orphan_detector` - Orphan detection
//! - `domain::services::compiler` - Path generation
//! - `infrastructure::fs::LocalFs` - File operations
//!
//! This module remains for backward compatibility with existing commands.
//!
//! Implements:
//! - TD-14: Atomic writes via tempfile + rename
//! - TD-15: Lockfile system for change detection

mod compile;
mod conflict;
pub mod lockfile;
pub mod orphan;
pub mod pipeline;
pub mod scope;
pub mod writer;

use std::path::{Path, PathBuf};

use crate::error::{CalvinError, CalvinResult};
use crate::models::Target;

pub use compile::compile_assets;
pub use lockfile::{lockfile_key, Lockfile, LockfileNamespace};
pub use pipeline::AssetPipeline;
pub use scope::{DeploymentTarget, ScopePolicy, ScopePolicyExt};
pub use writer::atomic_write;

// Re-export OutputFile from domain entities
pub use crate::domain::entities::OutputFile;

// Re-export conflict resolution types (ConflictReason is shared between plan and conflict modules)
pub use conflict::{ConflictChoice, ConflictReason, ConflictResolver, InteractiveResolver};

/// Expand ~/ prefix to user home directory
///
/// **Migration Note**: This function now delegates to `infrastructure::fs::expand_home`.
/// For new code, prefer using that directly.
pub fn expand_home_dir(path: &Path) -> PathBuf {
    crate::infrastructure::fs::expand_home(path)
}

/// Check if a path is safe (doesn't escape project root)
///
/// Protects against path traversal attacks like `../../etc/passwd`
pub fn validate_path_safety(path: &Path, project_root: &Path) -> CalvinResult<()> {
    // Skip paths starting with ~ (user-level paths are intentionally outside project)
    let path_str = path.to_string_lossy();
    if path_str.starts_with("~") {
        return Ok(());
    }

    // Absolute paths output paths are dangerous as they ignore the project root when joined
    if path.is_absolute() {
        return Err(CalvinError::PathEscape {
            path: path.to_path_buf(),
            root: project_root.to_path_buf(),
        });
    }

    // Check for path traversal attempts
    if path_str.contains("..") {
        // Resolve to canonical form and check
        let resolved = project_root.join(path);
        if let Ok(canonical) = resolved.canonicalize() {
            let root_canonical = project_root
                .canonicalize()
                .unwrap_or_else(|_| project_root.to_path_buf());
            if !canonical.starts_with(&root_canonical) {
                return Err(CalvinError::PathEscape {
                    path: path.to_path_buf(),
                    root: project_root.to_path_buf(),
                });
            }
        }
        // If we can't canonicalize, check for obvious escapes
        else if path_str.starts_with("..") {
            return Err(CalvinError::PathEscape {
                path: path.to_path_buf(),
                root: project_root.to_path_buf(),
            });
        }
    }

    Ok(())
}

/// Options for sync operations
#[derive(Debug, Clone, Default)]
pub struct SyncOptions {
    /// Force overwrite of modified files
    pub force: bool,
    /// Dry run - don't actually write
    pub dry_run: bool,
    /// Prompt for confirmation on conflicts
    pub interactive: bool,
    /// Enabled targets (empty = all)
    pub targets: Vec<Target>,
}

/// Result of a sync operation
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Files written
    pub written: Vec<String>,
    /// Files skipped (modified by user)
    pub skipped: Vec<String>,
    /// Errors encountered
    pub errors: Vec<String>,
}

/// Progress callback event emitted while syncing outputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncEvent {
    ItemStart {
        index: usize,
        path: String,
    },
    ItemWritten {
        index: usize,
        path: String,
    },
    ItemSkipped {
        index: usize,
        path: String,
    },
    ItemError {
        index: usize,
        path: String,
        message: String,
    },
}

impl SyncResult {
    pub fn new() -> Self {
        Self {
            written: Vec::new(),
            skipped: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}

impl Default for SyncResult {
    fn default() -> Self {
        Self::new()
    }
}

impl From<crate::application::DeployResult> for SyncResult {
    fn from(result: crate::application::DeployResult) -> Self {
        Self {
            written: result
                .written
                .into_iter()
                .map(|p| p.display().to_string())
                .collect(),
            skipped: result
                .skipped
                .into_iter()
                .map(|p| p.display().to_string())
                .collect(),
            errors: result.errors,
        }
    }
}

// ============================================================================
// MIGRATION STATUS
//
// This module has been migrated to the new layered architecture.
// It now serves as a backward-compatibility layer for legacy code.
//
// Completed migrations:
// - AssetPipeline -> application::pipeline (re-exported here)
// - ScopePolicy -> domain::policies (re-exported here)
// - OrphanFile, OrphanDetectionResult -> domain::services (re-exported here)
// - LockfileNamespace, lockfile_key -> domain::value_objects (re-exported here)
//
// Still in sync module (backward compatibility):
// - sync::lockfile::Lockfile (with I/O methods for legacy code)
// - sync::orphan::detect_orphans (wrapper for sync::lockfile::Lockfile)
// - sync::orphan::delete_orphans, check_orphan_status
// - SyncOptions, SyncResult, SyncEvent
//
// For new code, prefer:
// - application::DeployUseCase for deployment
// - domain::services::OrphanDetector for orphan detection
// - domain::entities::Lockfile + TomlLockfileRepository for lockfile ops
// ============================================================================

#[cfg(test)]
mod tests;
