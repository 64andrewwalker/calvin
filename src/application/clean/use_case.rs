//! Clean Use Case
//!
//! Orchestrates the file cleaning process.
//!
//! # Size Justification
//!
//! calvin-no-split: This module keeps CleanUseCase and its unit tests together to simplify
//! verification while migrating lockfile behavior for multi-layer promptpacks. Split later.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::domain::entities::Lockfile;
use crate::domain::ports::{FileSystem, LockfileRepository};
use crate::domain::services::has_calvin_signature;
use crate::domain::value_objects::Scope;

use super::options::CleanOptions;
use super::result::{CleanError, CleanResult, SkipReason};

/// Clean use case - removes deployed files tracked in lockfile
pub struct CleanUseCase<LR, FS>
where
    LR: LockfileRepository,
    FS: FileSystem,
{
    lockfile_repo: LR,
    fs: FS,
}

impl<LR, FS> CleanUseCase<LR, FS>
where
    LR: LockfileRepository,
    FS: FileSystem,
{
    /// Create a new clean use case
    pub fn new(lockfile_repo: LR, fs: FS) -> Self {
        Self { lockfile_repo, fs }
    }

    /// Execute the clean operation (preview mode - doesn't actually delete)
    ///
    /// Returns what would be deleted, allowing caller to confirm before actual deletion.
    pub fn execute(&self, lockfile_path: &Path, options: &CleanOptions) -> CleanResult {
        let lockfile = match self.lockfile_repo.load(lockfile_path) {
            Ok(lockfile) => lockfile,
            Err(e) => {
                let mut result = CleanResult::new();
                result
                    .errors
                    .push(CleanError::lockfile_error(e.to_string()));
                return result;
            }
        };
        self.process_lockfile(&lockfile, options, false)
    }

    /// Execute the clean operation with actual deletion
    ///
    /// This should be called after user confirms the preview.
    pub fn execute_confirmed(&self, lockfile_path: &Path, options: &CleanOptions) -> CleanResult {
        let lockfile = match self.lockfile_repo.load(lockfile_path) {
            Ok(lockfile) => lockfile,
            Err(e) => {
                let mut result = CleanResult::new();
                result
                    .errors
                    .push(CleanError::lockfile_error(e.to_string()));
                return result;
            }
        };
        let result = self.process_lockfile(&lockfile, options, !options.dry_run);

        // Update lockfile to remove entries
        if !options.dry_run {
            let mut updated_lockfile = lockfile.clone();

            // Remove deleted file entries
            for deleted_file in &result.deleted {
                updated_lockfile.remove(&deleted_file.key);
            }

            // Also remove entries for:
            // - Missing files (file doesn't exist anymore)
            // - Files without Calvin signature (Calvin never generated them)
            for skipped_file in &result.skipped {
                match skipped_file.reason {
                    SkipReason::Missing | SkipReason::NoSignature => {
                        updated_lockfile.remove(&skipped_file.key);
                    }
                    // Keep entries for modified files (user might want to keep track)
                    // Keep entries for permission denied (transient error)
                    SkipReason::Modified | SkipReason::PermissionDenied | SkipReason::Remote => {}
                }
            }

            // Save updated lockfile if anything changed
            if updated_lockfile.len() != lockfile.len() {
                if let Err(e) = self.lockfile_repo.save(&updated_lockfile, lockfile_path) {
                    eprintln!("Warning: Failed to update lockfile: {}", e);
                }
            }
        }

        result
    }

    /// Process the lockfile and return clean result
    fn process_lockfile(
        &self,
        lockfile: &Lockfile,
        options: &CleanOptions,
        actually_delete: bool,
    ) -> CleanResult {
        let mut result = CleanResult::new();

        // Get entries to process based on scope
        let entries_to_process: Vec<(&str, &str)> = match options.scope {
            Some(Scope::User) => lockfile
                .keys_for_scope(Scope::User)
                .filter_map(|key| lockfile.get_hash(key).map(|hash| (key, hash)))
                .collect(),
            Some(Scope::Project) => lockfile
                .keys_for_scope(Scope::Project)
                .filter_map(|key| lockfile.get_hash(key).map(|hash| (key, hash)))
                .collect(),
            None => lockfile
                .entries()
                .map(|(key, entry)| (key, entry.hash()))
                .collect(),
        };

        for (key, expected_hash) in entries_to_process {
            // If selected_keys is specified, skip keys not in the selection
            if let Some(ref selected) = options.selected_keys {
                if !selected.contains(key) {
                    continue;
                }
            }

            // Extract path from key
            let path_str = if let Some((_, p)) = Lockfile::parse_key(key) {
                p.to_string()
            } else {
                continue;
            };

            // Expand home directory if needed
            let path = if path_str.starts_with("~/") {
                self.fs.expand_home(&PathBuf::from(&path_str))
            } else {
                PathBuf::from(&path_str)
            };

            // Check if file exists
            if !self.fs.exists(&path) {
                result.add_skipped(path, SkipReason::Missing, key.to_string());
                continue;
            }

            // Read file content
            let content = match self.fs.read(&path) {
                Ok(c) => c,
                Err(_) => {
                    result.add_skipped(path, SkipReason::PermissionDenied, key.to_string());
                    continue;
                }
            };

            // Check signature (unless force)
            if !options.force && !has_calvin_signature(&content) {
                result.add_skipped(path, SkipReason::NoSignature, key.to_string());
                continue;
            }

            // Check hash (unless force)
            if !options.force {
                let mut hasher = Sha256::new();
                hasher.update(content.as_bytes());
                let actual_hash = format!("sha256:{:x}", hasher.finalize());
                if actual_hash != expected_hash {
                    result.add_skipped(path, SkipReason::Modified, key.to_string());
                    continue;
                }
            }

            // Delete the file
            if actually_delete {
                if let Err(e) = self.fs.remove(&path) {
                    result.add_error(CleanError::io_error(
                        path.clone(),
                        format!("Failed to delete: {}", e),
                    ));
                    continue;
                }
            }

            // Store path and original key for lockfile update
            result.add_deleted(path, key.to_string());
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::{LocalFs, TomlLockfileRepository};
    use tempfile::tempdir;

    /// The clean use case needs absolute paths in the lockfile to work correctly
    /// because it extracts the path from the lockfile key and uses it directly.
    /// In real usage, paths are stored as absolute (with ~) for home scope
    /// or relative for project scope, but relative paths need to be resolved
    /// against the current working directory.
    ///
    /// Helper to normalize path for lockfile key (use forward slashes for cross-platform)
    fn normalize_path_for_key(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    #[test]
    fn clean_deletes_files_from_lockfile() {
        let dir = tempdir().unwrap();
        let lockfile_path = dir.path().join("calvin.lock");
        std::fs::create_dir_all(lockfile_path.parent().unwrap()).unwrap();

        // Create file with correct hash - use absolute path
        let file_path = dir.path().join(".cursor/rules/test.mdc");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        let content = "<!-- Generated by Calvin -->\nTest content";
        std::fs::write(&file_path, content).unwrap();

        // Compute hash
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = format!("sha256:{:x}", hasher.finalize());

        // Create lockfile with normalized path (forward slashes for cross-platform TOML)
        let normalized_path = normalize_path_for_key(&file_path);
        std::fs::write(
            &lockfile_path,
            format!(
                r#"
version = 1

[files."project:{}"]
hash = "{}"
"#,
                normalized_path, hash
            ),
        )
        .unwrap();

        let repo = TomlLockfileRepository::new();
        let fs = LocalFs::new();
        let use_case = CleanUseCase::new(repo, fs);

        let options = CleanOptions::new().with_scope(Some(Scope::Project));
        let result = use_case.execute_confirmed(&lockfile_path, &options);

        assert_eq!(
            result.deleted.len(),
            1,
            "Should delete 1 file: {:?}",
            result
        );
        assert!(!file_path.exists(), "File should be deleted");
    }

    #[test]
    fn clean_skips_modified_files() {
        let dir = tempdir().unwrap();
        let lockfile_path = dir.path().join("calvin.lock");
        std::fs::create_dir_all(lockfile_path.parent().unwrap()).unwrap();

        // Create file with signature but different hash (modified)
        let file_path = dir.path().join(".cursor/rules/test.mdc");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        std::fs::write(&file_path, "<!-- Generated by Calvin -->\nModified content").unwrap();

        // Create lockfile with different hash (use normalized path for cross-platform)
        let normalized_path = normalize_path_for_key(&file_path);
        std::fs::write(
            &lockfile_path,
            format!(
                r#"
version = 1

[files."project:{}"]
hash = "sha256:original_hash_that_does_not_match"
"#,
                normalized_path
            ),
        )
        .unwrap();

        let repo = TomlLockfileRepository::new();
        let fs = LocalFs::new();
        let use_case = CleanUseCase::new(repo, fs);

        let options = CleanOptions::new().with_scope(Some(Scope::Project));
        let result = use_case.execute(&lockfile_path, &options);

        assert_eq!(result.skipped.len(), 1, "Should skip 1 file: {:?}", result);
        assert_eq!(result.skipped[0].reason, SkipReason::Modified);
        assert!(file_path.exists(), "File should still exist");
    }

    #[test]
    fn clean_dry_run_does_not_delete() {
        let dir = tempdir().unwrap();
        let lockfile_path = dir.path().join("calvin.lock");
        std::fs::create_dir_all(lockfile_path.parent().unwrap()).unwrap();

        let file_path = dir.path().join(".cursor/rules/test.mdc");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        let content = "<!-- Generated by Calvin -->\nTest content";
        std::fs::write(&file_path, content).unwrap();

        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = format!("sha256:{:x}", hasher.finalize());

        let normalized_path = normalize_path_for_key(&file_path);
        std::fs::write(
            &lockfile_path,
            format!(
                r#"
version = 1

[files."project:{}"]
hash = "{}"
"#,
                normalized_path, hash
            ),
        )
        .unwrap();

        let repo = TomlLockfileRepository::new();
        let fs = LocalFs::new();
        let use_case = CleanUseCase::new(repo, fs);

        let options = CleanOptions::new()
            .with_scope(Some(Scope::Project))
            .with_dry_run(true);
        let result = use_case.execute(&lockfile_path, &options);

        assert_eq!(
            result.deleted.len(),
            1,
            "Should show 1 file would be deleted"
        );
        assert!(file_path.exists(), "File should still exist in dry run");
    }

    #[test]
    fn clean_skips_files_without_signature() {
        let dir = tempdir().unwrap();
        let lockfile_path = dir.path().join("calvin.lock");
        std::fs::create_dir_all(lockfile_path.parent().unwrap()).unwrap();

        // Create file WITHOUT signature
        let file_path = dir.path().join(".cursor/rules/test.mdc");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        let content = "User created content without Calvin signature";
        std::fs::write(&file_path, content).unwrap();

        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = format!("sha256:{:x}", hasher.finalize());

        let normalized_path = normalize_path_for_key(&file_path);
        std::fs::write(
            &lockfile_path,
            format!(
                r#"
version = 1

[files."project:{}"]
hash = "{}"
"#,
                normalized_path, hash
            ),
        )
        .unwrap();

        let repo = TomlLockfileRepository::new();
        let fs = LocalFs::new();
        let use_case = CleanUseCase::new(repo, fs);

        let options = CleanOptions::new().with_scope(Some(Scope::Project));
        let result = use_case.execute(&lockfile_path, &options);

        assert_eq!(result.skipped.len(), 1, "Should skip 1 file");
        assert_eq!(result.skipped[0].reason, SkipReason::NoSignature);
        assert!(file_path.exists(), "File without signature should be kept");
    }

    #[test]
    fn clean_force_deletes_modified_files() {
        let dir = tempdir().unwrap();
        let lockfile_path = dir.path().join("calvin.lock");
        std::fs::create_dir_all(lockfile_path.parent().unwrap()).unwrap();

        // Create file without signature (normally skipped)
        let file_path = dir.path().join(".cursor/rules/test.mdc");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        let content = "User modified content";
        std::fs::write(&file_path, content).unwrap();

        let normalized_path = normalize_path_for_key(&file_path);
        std::fs::write(
            &lockfile_path,
            format!(
                r#"
version = 1

[files."project:{}"]
hash = "sha256:original_hash"
"#,
                normalized_path
            ),
        )
        .unwrap();

        let repo = TomlLockfileRepository::new();
        let fs = LocalFs::new();
        let use_case = CleanUseCase::new(repo, fs);

        let options = CleanOptions::new()
            .with_scope(Some(Scope::Project))
            .with_force(true);
        let result = use_case.execute_confirmed(&lockfile_path, &options);

        assert_eq!(result.deleted.len(), 1, "Should force delete 1 file");
        assert!(!file_path.exists(), "File should be deleted with --force");
    }

    #[test]
    fn clean_selected_keys_only_deletes_selected() {
        let dir = tempdir().unwrap();
        let lockfile_path = dir.path().join("calvin.lock");
        std::fs::create_dir_all(lockfile_path.parent().unwrap()).unwrap();

        // Create two files with correct hashes
        let file1 = dir.path().join(".cursor/rules/keep.mdc");
        let file2 = dir.path().join(".cursor/rules/delete.mdc");
        std::fs::create_dir_all(file1.parent().unwrap()).unwrap();

        let content1 = "<!-- Generated by Calvin -->\nKeep this file";
        let content2 = "<!-- Generated by Calvin -->\nDelete this file";
        std::fs::write(&file1, content1).unwrap();
        std::fs::write(&file2, content2).unwrap();

        // Compute hashes
        let mut hasher1 = Sha256::new();
        hasher1.update(content1.as_bytes());
        let hash1 = format!("sha256:{:x}", hasher1.finalize());

        let mut hasher2 = Sha256::new();
        hasher2.update(content2.as_bytes());
        let hash2 = format!("sha256:{:x}", hasher2.finalize());

        // Create lockfile with both entries (use normalized paths for cross-platform)
        let normalized_path1 = normalize_path_for_key(&file1);
        let normalized_path2 = normalize_path_for_key(&file2);
        let key2 = format!("project:{}", normalized_path2);

        std::fs::write(
            &lockfile_path,
            format!(
                r#"
version = 1

[files."project:{}"]
hash = "{}"

[files."project:{}"]
hash = "{}"
"#,
                normalized_path1, hash1, normalized_path2, hash2
            ),
        )
        .unwrap();

        let repo = TomlLockfileRepository::new();
        let fs = LocalFs::new();
        let use_case = CleanUseCase::new(repo, fs);

        // Only select file2 for deletion
        let options = CleanOptions::new()
            .with_scope(Some(Scope::Project))
            .with_selected_keys(vec![key2]);

        let result = use_case.execute_confirmed(&lockfile_path, &options);

        assert_eq!(result.deleted.len(), 1, "Should delete only 1 file");
        assert!(file1.exists(), "file1 should NOT be deleted");
        assert!(!file2.exists(), "file2 should be deleted");
    }

    #[test]
    fn clean_reports_error_on_lockfile_version_mismatch() {
        let dir = tempdir().unwrap();
        let lockfile_path = dir.path().join("calvin.lock");
        std::fs::write(
            &lockfile_path,
            r#"
version = 999

[files."project:test.md"]
hash = "sha256:abc"
"#,
        )
        .unwrap();

        let repo = TomlLockfileRepository::new();
        let fs = LocalFs::new();
        let use_case = CleanUseCase::new(repo, fs);

        let options = CleanOptions::new().with_scope(Some(Scope::Project));
        let result = use_case.execute(&lockfile_path, &options);

        assert!(
            !result.errors.is_empty(),
            "expected lockfile parse/version errors"
        );
        let msg = result.errors[0].to_string();
        assert!(msg.contains("lockfile format incompatible"), "msg: {}", msg);
    }
}
