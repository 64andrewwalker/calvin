//! Clean Use Case
//!
//! Orchestrates the file cleaning process.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::domain::entities::Lockfile;
use crate::domain::ports::{FileSystem, LockfileRepository};
use crate::domain::services::has_calvin_signature;
use crate::domain::value_objects::Scope;

use super::options::CleanOptions;
use super::result::{CleanResult, SkipReason};

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
        let lockfile = self.lockfile_repo.load_or_new(lockfile_path);
        self.process_lockfile(&lockfile, options, false)
    }

    /// Execute the clean operation with actual deletion
    ///
    /// This should be called after user confirms the preview.
    pub fn execute_confirmed(&self, lockfile_path: &Path, options: &CleanOptions) -> CleanResult {
        let lockfile = self.lockfile_repo.load_or_new(lockfile_path);
        let result = self.process_lockfile(&lockfile, options, !options.dry_run);

        // Update lockfile to remove deleted entries
        if !options.dry_run && !result.deleted.is_empty() {
            let mut updated_lockfile = lockfile.clone();
            for path in &result.deleted {
                // Find the key for this path
                if let Some(scope) = options.scope {
                    let key = Lockfile::make_key(scope, &path.display().to_string());
                    updated_lockfile.remove(&key);
                } else {
                    // Try both scopes
                    let home_key = Lockfile::make_key(Scope::User, &path.display().to_string());
                    let project_key =
                        Lockfile::make_key(Scope::Project, &path.display().to_string());
                    updated_lockfile.remove(&home_key);
                    updated_lockfile.remove(&project_key);
                }
            }

            // Save updated lockfile
            if let Err(e) = self.lockfile_repo.save(&updated_lockfile, lockfile_path) {
                eprintln!("Warning: Failed to update lockfile: {}", e);
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
                result.add_skipped(path, SkipReason::Missing);
                continue;
            }

            // Read file content
            let content = match self.fs.read(&path) {
                Ok(c) => c,
                Err(_) => {
                    result.add_skipped(path, SkipReason::PermissionDenied);
                    continue;
                }
            };

            // Check signature (unless force)
            if !options.force && !has_calvin_signature(&content) {
                result.add_skipped(path, SkipReason::NoSignature);
                continue;
            }

            // Check hash (unless force)
            if !options.force {
                let mut hasher = Sha256::new();
                hasher.update(content.as_bytes());
                let actual_hash = format!("sha256:{:x}", hasher.finalize());
                if actual_hash != expected_hash {
                    result.add_skipped(path, SkipReason::Modified);
                    continue;
                }
            }

            // Delete the file
            if actually_delete {
                if let Err(e) = self.fs.remove(&path) {
                    result.add_error(format!("Failed to delete {}: {}", path.display(), e));
                    continue;
                }
            }

            result.add_deleted(path);
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

    #[test]
    fn clean_deletes_files_from_lockfile() {
        let dir = tempdir().unwrap();
        let lockfile_path = dir.path().join(".promptpack/.calvin.lock");
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

        // Create lockfile with absolute path in key (simulating project: prefix resolution)
        let abs_path = file_path.to_string_lossy();
        std::fs::write(
            &lockfile_path,
            format!(
                r#"
version = 1

[files."project:{}"]
hash = "{}"
"#,
                abs_path, hash
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
        let lockfile_path = dir.path().join(".promptpack/.calvin.lock");
        std::fs::create_dir_all(lockfile_path.parent().unwrap()).unwrap();

        // Create file with signature but different hash (modified)
        let file_path = dir.path().join(".cursor/rules/test.mdc");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        std::fs::write(&file_path, "<!-- Generated by Calvin -->\nModified content").unwrap();

        // Create lockfile with different hash
        let abs_path = file_path.to_string_lossy();
        std::fs::write(
            &lockfile_path,
            format!(
                r#"
version = 1

[files."project:{}"]
hash = "sha256:original_hash_that_does_not_match"
"#,
                abs_path
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
        let lockfile_path = dir.path().join(".promptpack/.calvin.lock");
        std::fs::create_dir_all(lockfile_path.parent().unwrap()).unwrap();

        let file_path = dir.path().join(".cursor/rules/test.mdc");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        let content = "<!-- Generated by Calvin -->\nTest content";
        std::fs::write(&file_path, content).unwrap();

        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = format!("sha256:{:x}", hasher.finalize());

        let abs_path = file_path.to_string_lossy();
        std::fs::write(
            &lockfile_path,
            format!(
                r#"
version = 1

[files."project:{}"]
hash = "{}"
"#,
                abs_path, hash
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
        let lockfile_path = dir.path().join(".promptpack/.calvin.lock");
        std::fs::create_dir_all(lockfile_path.parent().unwrap()).unwrap();

        // Create file WITHOUT signature
        let file_path = dir.path().join(".cursor/rules/test.mdc");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        let content = "User created content without Calvin signature";
        std::fs::write(&file_path, content).unwrap();

        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = format!("sha256:{:x}", hasher.finalize());

        let abs_path = file_path.to_string_lossy();
        std::fs::write(
            &lockfile_path,
            format!(
                r#"
version = 1

[files."project:{}"]
hash = "{}"
"#,
                abs_path, hash
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
        let lockfile_path = dir.path().join(".promptpack/.calvin.lock");
        std::fs::create_dir_all(lockfile_path.parent().unwrap()).unwrap();

        // Create file without signature (normally skipped)
        let file_path = dir.path().join(".cursor/rules/test.mdc");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        let content = "User modified content";
        std::fs::write(&file_path, content).unwrap();

        let abs_path = file_path.to_string_lossy();
        std::fs::write(
            &lockfile_path,
            format!(
                r#"
version = 1

[files."project:{}"]
hash = "sha256:original_hash"
"#,
                abs_path
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
}
