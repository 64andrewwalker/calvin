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
pub mod engine;
pub mod execute;
pub mod lockfile;
pub mod orphan;
pub mod pipeline;
pub mod plan;
pub mod remote;
pub mod scope;
pub mod writer;

use std::path::{Path, PathBuf};

use crate::error::{CalvinError, CalvinResult};
use crate::models::Target;

pub use compile::compile_assets;
pub use lockfile::{lockfile_key, Lockfile, LockfileNamespace};
pub use pipeline::AssetPipeline;
pub use scope::{DeploymentTarget, ScopePolicy};
pub use writer::atomic_write;

// Re-export new two-stage sync API
pub use engine::{SyncEngine, SyncEngineOptions};
pub use execute::{execute_sync, execute_sync_with_callback, SyncStrategy};
pub use plan::{plan_sync, plan_sync_remote, resolve_conflicts_interactive};
pub use plan::{Conflict, ResolveResult, SyncDestination, SyncPlan};

// Re-export conflict resolution types (ConflictReason is shared between plan and conflict modules)
pub use conflict::{ConflictChoice, ConflictReason, ConflictResolver, InteractiveResolver};

/// Expand ~/ prefix to user home directory
pub fn expand_home_dir(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    if path_str.starts_with("~/") || path_str == "~" {
        if let Some(home) = dirs_home() {
            return home.join(path_str.strip_prefix("~/").unwrap_or(""));
        }
    }
    path.to_path_buf()
}

/// Get home directory (platform-independent)
///
/// Uses `dirs` crate for robust cross-platform support:
/// - macOS: /Users/<username>
/// - Linux: /home/<username>
/// - Windows: C:\Users\<username>
fn dirs_home() -> Option<PathBuf> {
    // Use dirs crate for proper cross-platform support
    dirs::home_dir()
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

// ============================================================================
// DEPRECATED: Old sync functions removed
//
// The following functions have been removed in favor of SyncEngine:
// - sync_outputs()
// - sync_with_fs()
// - sync_with_fs_with_prompter()
// - sync_with_fs_with_callback()
// - sync_outputs_with_callback()
// - sync()
//
// Use SyncEngine instead:
//   let engine = SyncEngine::local(&outputs, root, options);
//   let result = engine.sync()?;
// ============================================================================

#[cfg(test)]
mod tests;
