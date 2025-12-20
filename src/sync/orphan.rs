//! Orphan file detection and cleanup
//!
//! Detects files that were previously deployed by Calvin but are no longer
//! in the current output set. These "orphan" files may need cleanup.
//!
//! **Migration Status**: Core types are now in `domain::services::orphan_detector`.
//! This module re-exports those types and provides backward-compatible adapters.
//!
//! For new code, use:
//! - `domain::services::OrphanDetector::detect()` with `domain::entities::Lockfile`
//! - `domain::value_objects::Scope` instead of `LockfileNamespace`

use std::collections::HashSet;

use super::OutputFile;
use crate::sync::lockfile::{lockfile_key, Lockfile, LockfileNamespace};

// Re-export domain types for migration
pub use crate::domain::services::{
    extract_path_from_key, has_calvin_signature, OrphanDetectionResult, OrphanFile,
    CALVIN_SIGNATURES,
};

/// Detect orphan files by comparing lockfile entries against current outputs
///
/// **Migration Note**: This is a backward-compatible wrapper around
/// `domain::services::OrphanDetector::detect()`. New code should use that directly.
///
/// # Arguments
/// * `lockfile` - Current lockfile with previously deployed files (`sync::lockfile::Lockfile`)
/// * `outputs` - Current output files that will be deployed
/// * `namespace` - The namespace to filter lockfile entries (Project or Home)
pub fn detect_orphans(
    lockfile: &Lockfile,
    outputs: &[OutputFile],
    namespace: LockfileNamespace,
) -> OrphanDetectionResult {
    // Build set of current output keys
    let current_keys: HashSet<String> = outputs
        .iter()
        .map(|o| lockfile_key(namespace, o.path()))
        .collect();

    // Determine namespace prefix to filter lockfile entries
    let prefix = match namespace {
        LockfileNamespace::Project => "project:",
        LockfileNamespace::Home => "home:",
    };

    let mut result = OrphanDetectionResult::default();

    for key in lockfile.paths() {
        // Only check entries matching our namespace
        if !key.starts_with(prefix) {
            continue;
        }

        if current_keys.contains(key) {
            result.retained.push(key.to_string());
        } else {
            // This is an orphan - file was previously deployed but not in current outputs
            result.orphans.push(OrphanFile::from_key(key));
        }
    }

    result
}

/// Check orphan files for Calvin signature and existence
///
/// This requires filesystem access to read the actual file content.
pub fn check_orphan_status<FS: crate::fs::FileSystem + ?Sized>(orphan: &mut OrphanFile, fs: &FS) {
    // Use the pre-extracted path from the orphan
    let expanded = crate::sync::expand_home_dir(&std::path::PathBuf::from(&orphan.path));

    orphan.exists = fs.exists(&expanded);

    if orphan.exists {
        if let Ok(content) = fs.read_to_string(&expanded) {
            orphan.has_signature = has_calvin_signature(&content);
        }
    }
}

/// Result of delete_orphans operation
#[derive(Debug, Clone, Default)]
pub struct DeleteResult {
    /// Files that were successfully deleted
    pub deleted: Vec<String>,
    /// Files that were skipped (no signature and not force)
    pub skipped: Vec<String>,
}

/// Delete orphan files
///
/// # Arguments
/// * `orphans` - List of orphan files to consider for deletion
/// * `force` - If true, delete all orphans regardless of signature
/// * `fs` - Filesystem to use for deletion
///
/// # Returns
/// Result containing counts of deleted and skipped files
pub fn delete_orphans<FS: crate::fs::FileSystem + ?Sized>(
    orphans: &[OrphanFile],
    force: bool,
    fs: &FS,
) -> crate::error::CalvinResult<DeleteResult> {
    use std::path::PathBuf;

    let mut result = DeleteResult::default();

    for orphan in orphans {
        // Skip non-existent files
        if !orphan.exists {
            continue;
        }

        // Determine if we should delete
        let should_delete = force || orphan.is_safe_to_delete();

        if should_delete {
            let path = PathBuf::from(&orphan.path);
            let expanded = crate::sync::expand_home_dir(&path);

            // Attempt deletion
            if let Err(e) = fs.remove_file(&expanded) {
                // Log but don't fail - continue with other files
                eprintln!("Warning: Failed to delete {}: {}", expanded.display(), e);
            } else {
                result.deleted.push(orphan.key.clone());
            }
        } else {
            // File exists but no signature - skip
            result.skipped.push(orphan.key.clone());
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ========================================================================
    // Tests for sync-specific functions (detect_orphans with sync::Lockfile)
    //
    // Note: Tests for re-exported domain types (has_calvin_signature,
    // extract_path_from_key, OrphanFile) are in domain::services::orphan_detector
    // ========================================================================

    #[test]
    fn detect_orphans_finds_removed_files() {
        let mut lockfile = Lockfile::new();
        lockfile.set_hash("home:~/.claude/commands/old.md", "sha256:abc");
        lockfile.set_hash("home:~/.claude/commands/current.md", "sha256:def");

        let outputs = vec![OutputFile::new(
            PathBuf::from("~/.claude/commands/current.md"),
            "content",
            crate::domain::value_objects::Target::All,
        )];

        let result = detect_orphans(&lockfile, &outputs, LockfileNamespace::Home);

        assert_eq!(result.orphans.len(), 1);
        assert_eq!(result.orphans[0].key, "home:~/.claude/commands/old.md");
        assert_eq!(result.retained.len(), 1);
    }

    #[test]
    fn detect_orphans_filters_by_namespace() {
        let mut lockfile = Lockfile::new();
        lockfile.set_hash("home:~/.claude/commands/home.md", "sha256:abc");
        lockfile.set_hash("project:.claude/commands/project.md", "sha256:def");

        let outputs: Vec<OutputFile> = vec![];

        // When checking home namespace, only home entries are orphans
        let result = detect_orphans(&lockfile, &outputs, LockfileNamespace::Home);
        assert_eq!(result.orphans.len(), 1);
        assert!(result.orphans[0].key.starts_with("home:"));

        // When checking project namespace, only project entries are orphans
        let result = detect_orphans(&lockfile, &outputs, LockfileNamespace::Project);
        assert_eq!(result.orphans.len(), 1);
        assert!(result.orphans[0].key.starts_with("project:"));
    }

    // ========================================================================
    // SC-6 delete_orphans tests (sync-specific functionality)
    // ========================================================================
    #[test]
    fn delete_orphans_deletes_safe_files_only() {
        use crate::infrastructure::fs::LocalFs;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let safe_file = dir.path().join("safe.md");
        let unsafe_file = dir.path().join("unsafe.md");

        // Create test files
        std::fs::write(&safe_file, "<!-- Generated by Calvin -->").unwrap();
        std::fs::write(&unsafe_file, "User created content").unwrap();

        let orphan_safe = OrphanFile::from_key(&format!("project:{}", safe_file.display()))
            .with_status(true, true);
        let mut orphan_safe_mut = orphan_safe;
        orphan_safe_mut.path = safe_file.to_string_lossy().to_string();

        let orphan_unsafe = OrphanFile::from_key(&format!("project:{}", unsafe_file.display()))
            .with_status(true, false);
        let mut orphan_unsafe_mut = orphan_unsafe;
        orphan_unsafe_mut.path = unsafe_file.to_string_lossy().to_string();

        let orphans = vec![orphan_safe_mut, orphan_unsafe_mut];

        let fs = LocalFs::new();
        let result = delete_orphans(&orphans, false, &fs).unwrap();

        assert_eq!(result.deleted.len(), 1);
        assert_eq!(result.skipped.len(), 1);
        assert!(!safe_file.exists());
        assert!(unsafe_file.exists());
    }

    #[test]
    fn delete_orphans_force_deletes_all() {
        use crate::infrastructure::fs::LocalFs;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let safe_file = dir.path().join("safe.md");
        let unsafe_file = dir.path().join("unsafe.md");

        std::fs::write(&safe_file, "<!-- Generated by Calvin -->").unwrap();
        std::fs::write(&unsafe_file, "User content").unwrap();

        let orphan_safe = OrphanFile::from_key(&format!("project:{}", safe_file.display()))
            .with_status(true, true);
        let mut orphan_safe_mut = orphan_safe;
        orphan_safe_mut.path = safe_file.to_string_lossy().to_string();

        let orphan_unsafe = OrphanFile::from_key(&format!("project:{}", unsafe_file.display()))
            .with_status(true, false);
        let mut orphan_unsafe_mut = orphan_unsafe;
        orphan_unsafe_mut.path = unsafe_file.to_string_lossy().to_string();

        let orphans = vec![orphan_safe_mut, orphan_unsafe_mut];

        let fs = LocalFs::new();
        let result = delete_orphans(&orphans, true, &fs).unwrap();

        assert_eq!(result.deleted.len(), 2);
        assert_eq!(result.skipped.len(), 0);
        assert!(!safe_file.exists());
        assert!(!unsafe_file.exists());
    }

    #[test]
    fn delete_orphans_skips_nonexistent() {
        use crate::infrastructure::fs::LocalFs;

        let orphan = OrphanFile::from_key("project:/nonexistent/path.md");
        // Leave has_signature and exists as false (default)
        let orphans = vec![orphan];

        let fs = LocalFs::new();
        let result = delete_orphans(&orphans, false, &fs).unwrap();

        // Should skip because exists=false
        assert_eq!(result.deleted.len(), 0);
        assert_eq!(result.skipped.len(), 0); // Not counted as skipped, just ignored
    }

    // ========================================================================
    // SC-7 Scope Isolation Tests
    // Note: Comprehensive scope isolation tests are in domain::services::orphan_detector
    // This single test verifies detect_orphans (sync version) respects scope
    // ========================================================================

    #[test]
    fn scope_isolation_with_sync_lockfile() {
        // Test that detect_orphans with sync::Lockfile respects scope boundaries
        let mut lockfile = Lockfile::new();
        lockfile.set_hash("project:.claude/commands/local.md", "sha256:abc");
        lockfile.set_hash("home:~/.claude/commands/global.md", "sha256:def");

        let outputs = vec![OutputFile::new(
            PathBuf::from("~/.claude/commands/global.md"),
            "content",
            crate::domain::value_objects::Target::All,
        )];

        let result = detect_orphans(&lockfile, &outputs, LockfileNamespace::Home);

        // home file is retained, project file is NOT detected as orphan
        assert_eq!(result.retained.len(), 1);
        assert_eq!(result.orphans.len(), 0);
    }
}
