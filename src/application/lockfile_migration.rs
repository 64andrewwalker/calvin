//! Lockfile path resolution and migration.
//!
//! The lockfile moved from `./.promptpack/.calvin.lock` to `./calvin.lock` (project root).
//! This module provides a shared helper to:
//! - prefer the new location when present
//! - auto-migrate from the legacy location when needed

use std::path::{Path, PathBuf};

use crate::domain::ports::LockfileRepository;

/// Resolve the global lockfile path for user/home deployments.
///
/// This is stored under the Calvin user state directory:
/// - `{HOME}/.calvin/calvin.lock`
///
/// Returns `None` if the home directory cannot be resolved.
pub fn global_lockfile_path() -> Option<PathBuf> {
    crate::infrastructure::calvin_home_dir().map(|h| h.join(".calvin/calvin.lock"))
}

/// Resolve the lockfile path for a project and migrate legacy lockfile if present.
///
/// - New location: `{project_root}/calvin.lock`
/// - Legacy location: `{source}/.calvin.lock` (typically `{project_root}/.promptpack/.calvin.lock`)
///
/// Returns `(path_to_use, optional_message)`.
pub fn resolve_lockfile_path<LR: LockfileRepository>(
    project_root: &Path,
    source: &Path,
    lockfile_repo: &LR,
) -> (PathBuf, Option<String>) {
    let new_path = project_root.join("calvin.lock");
    let old_path = source.join(".calvin.lock");

    if new_path.exists() {
        return (new_path, None);
    }

    if old_path.exists() {
        // Load legacy lockfile
        let lockfile = match lockfile_repo.load(&old_path) {
            Ok(lockfile) => lockfile,
            Err(e) => {
                return (
                    old_path,
                    Some(format!(
                        "Failed to migrate lockfile to {}: {}",
                        new_path.display(),
                        e
                    )),
                );
            }
        };

        // Save to new location
        if let Err(e) = lockfile_repo.save(&lockfile, &new_path) {
            return (
                old_path,
                Some(format!(
                    "Failed to migrate lockfile to {}: {}",
                    new_path.display(),
                    e
                )),
            );
        }

        // Delete legacy lockfile (best-effort; new lockfile is the source of truth)
        match std::fs::remove_file(&old_path) {
            Ok(()) => {
                let message = format!("Migrated lockfile to {}", new_path.display());
                (new_path, Some(message))
            }
            Err(e) => {
                let message = format!(
                    "Migrated lockfile to {}, but failed to delete legacy lockfile: {}",
                    new_path.display(),
                    e
                );
                (new_path, Some(message))
            }
        }
    } else {
        (new_path, None)
    }
}
