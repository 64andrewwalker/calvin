//! Diff Use Case
//!
//! Orchestrates the diff flow:
//! 1. Load assets from source
//! 2. Compile assets for target platforms
//! 3. Compare with current target state
//! 4. Return what would change
//!
//! This is essentially a dry-run of the deploy use case.

use std::path::PathBuf;

use crate::domain::entities::{Lockfile, OutputFile};
use crate::domain::ports::{AssetRepository, FileSystem, LockfileRepository, TargetAdapter};
use crate::domain::services::{FileAction, Planner, TargetFileState};
use crate::domain::value_objects::{Scope, Target};

/// Options for the diff operation
#[derive(Debug, Clone)]
pub struct DiffOptions {
    /// Source directory (.promptpack)
    pub source: PathBuf,
    /// Project root directory
    pub project_root: PathBuf,
    /// Target platforms to compile for
    pub targets: Vec<Target>,
    /// Deploy scope (User = home, Project = local)
    pub scope: Scope,
}

impl DiffOptions {
    pub fn new(source: impl Into<PathBuf>) -> Self {
        Self {
            source: source.into(),
            project_root: PathBuf::from("."),
            targets: vec![Target::All],
            scope: Scope::Project,
        }
    }

    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_project_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.project_root = root.into();
        self
    }
}

/// A file that would be modified
#[derive(Debug, Clone)]
pub struct DiffEntry {
    /// Path to the file
    pub path: PathBuf,
    /// Type of change
    pub change_type: ChangeType,
    /// Preview of new content (if applicable)
    pub new_content: Option<String>,
}

/// Type of change for a file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    /// New file would be created
    Create,
    /// Existing file would be updated
    Update,
    /// File would be skipped (already up-to-date)
    Skip,
    /// Conflict detected
    Conflict,
}

/// Result of the diff operation
#[derive(Debug, Clone, Default)]
pub struct DiffResult {
    /// Files that would be created
    pub creates: Vec<DiffEntry>,
    /// Files that would be updated
    pub updates: Vec<DiffEntry>,
    /// Files that would be skipped
    pub skipped: Vec<DiffEntry>,
    /// Conflicts detected
    pub conflicts: Vec<DiffEntry>,
    /// Orphan files (exist in lockfile but not in current output)
    pub orphans: Vec<PathBuf>,
    /// Total assets processed
    pub asset_count: usize,
    /// Total outputs generated
    pub output_count: usize,
}

impl DiffResult {
    /// Check if there are any changes
    pub fn has_changes(&self) -> bool {
        !self.creates.is_empty() || !self.updates.is_empty()
    }

    /// Check if there are any conflicts
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }

    /// Total number of files that would be affected
    pub fn total_affected(&self) -> usize {
        self.creates.len() + self.updates.len()
    }
}

/// Diff Use Case
///
/// Shows what would change during a deploy operation.
/// Uses the same dependency injection pattern as DeployUseCase.
pub struct DiffUseCase<AR, LR, FS>
where
    AR: AssetRepository,
    LR: LockfileRepository,
    FS: FileSystem,
{
    asset_repo: AR,
    lockfile_repo: LR,
    file_system: FS,
    adapters: Vec<Box<dyn TargetAdapter>>,
}

impl<AR, LR, FS> DiffUseCase<AR, LR, FS>
where
    AR: AssetRepository,
    LR: LockfileRepository,
    FS: FileSystem,
{
    /// Create a new DiffUseCase with injected dependencies
    pub fn new(
        asset_repo: AR,
        lockfile_repo: LR,
        file_system: FS,
        adapters: Vec<Box<dyn TargetAdapter>>,
    ) -> Self {
        Self {
            asset_repo,
            lockfile_repo,
            file_system,
            adapters,
        }
    }

    /// Execute the diff operation
    ///
    /// Returns what would change without making any modifications.
    pub fn execute(&self, options: &DiffOptions) -> DiffResult {
        let mut result = DiffResult::default();

        // Determine project root (parent of source directory)
        let project_root = options
            .source
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // Step 1: Load assets
        let assets = match self.asset_repo.load_all(&options.source) {
            Ok(a) => a,
            Err(_) => return result,
        };

        // Step 1.5: Apply scope policy - when targeting User scope, force all assets to User
        let assets = self.apply_scope_policy(assets, options.scope);
        result.asset_count = assets.len();

        // Step 2: Compile assets using adapters
        let outputs = match self.compile_assets(&assets, options) {
            Ok(o) => o,
            Err(_) => return result,
        };
        result.output_count = outputs.len();

        // Step 3: Load lockfile
        let lockfile = self.lockfile_repo.load(&options.source).unwrap_or_default();

        // Step 4: Compare each output with target state
        for output in &outputs {
            let output_path = output.path();
            let path_str = output_path.display().to_string();

            // Resolve path: expand ~ for home paths, join with project root for project paths
            let resolved_path = if path_str.starts_with('~') {
                self.file_system.expand_home(output_path)
            } else {
                project_root.join(output_path)
            };

            let exists = self.file_system.exists(&resolved_path);
            let current_hash = if exists {
                self.file_system.hash(&resolved_path).ok()
            } else {
                None
            };

            // Get lockfile key (uses original output path, not resolved)
            let lockfile_key = Lockfile::make_key(options.scope, &path_str);

            // Compute new hash
            let mut output_clone = output.clone();
            let new_hash = output_clone.hash().to_string();

            // Build target state (using resolved path for existence check)
            let target_state = if exists {
                if let Some(hash) = current_hash.clone() {
                    TargetFileState::exists_with_hash(hash)
                } else {
                    TargetFileState {
                        exists: true,
                        current_hash: None,
                    }
                }
            } else {
                TargetFileState::not_exists()
            };

            // Plan this file
            let action = Planner::plan_file(&new_hash, &target_state, &lockfile, &lockfile_key);

            // Convert action to diff entry (uses original output path)
            let entry = DiffEntry {
                path: output_path.clone(),
                change_type: match &action {
                    FileAction::Write if !exists => ChangeType::Create,
                    FileAction::Write => ChangeType::Update,
                    FileAction::Skip => ChangeType::Skip,
                    FileAction::Conflict(_) => ChangeType::Conflict,
                },
                new_content: match &action {
                    FileAction::Write | FileAction::Conflict(_) => {
                        Some(output.content().to_string())
                    }
                    FileAction::Skip => None,
                },
            };

            // Add to appropriate bucket
            match entry.change_type {
                ChangeType::Create => result.creates.push(entry),
                ChangeType::Update => result.updates.push(entry),
                ChangeType::Skip => result.skipped.push(entry),
                ChangeType::Conflict => result.conflicts.push(entry),
            }
        }

        // Step 5: Detect orphans
        use crate::domain::services::OrphanDetector;
        let orphan_result = OrphanDetector::detect(&lockfile, &outputs, options.scope);
        result.orphans = orphan_result
            .orphans
            .iter()
            .map(|o| PathBuf::from(&o.path))
            .collect();

        result
    }

    /// Compile assets using adapters
    fn compile_assets(
        &self,
        assets: &[crate::domain::entities::Asset],
        options: &DiffOptions,
    ) -> Result<Vec<OutputFile>, String> {
        let mut outputs = Vec::new();

        // Filter adapters based on targets
        let active_adapters: Vec<_> = self
            .adapters
            .iter()
            .filter(|a| {
                options.targets.is_empty()
                    || options.targets.contains(&Target::All)
                    || options.targets.contains(&a.target())
            })
            .collect();

        // Compile each asset with each adapter
        for asset in assets {
            for adapter in &active_adapters {
                match adapter.compile(asset) {
                    Ok(adapter_outputs) => outputs.extend(adapter_outputs),
                    Err(e) => return Err(e.to_string()),
                }
            }
        }

        Ok(outputs)
    }

    /// Apply scope policy to assets
    ///
    /// When targeting User scope, force all assets to User scope.
    /// This ensures adapters generate home-relative paths (~/...).
    fn apply_scope_policy(
        &self,
        assets: Vec<crate::domain::entities::Asset>,
        target_scope: Scope,
    ) -> Vec<crate::domain::entities::Asset> {
        match target_scope {
            Scope::User => {
                // Force all assets to User scope
                assets
                    .into_iter()
                    .map(|a| a.with_scope(Scope::User))
                    .collect()
            }
            Scope::Project => {
                // Keep original scope from assets
                assets
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{Asset, Lockfile};
    use crate::domain::ports::{
        file_system::FsError, AdapterDiagnostic, AdapterError, AssetRepository, FileSystem,
        FsResult, LockfileError, LockfileRepository, TargetAdapter,
    };
    use anyhow::Result as AnyhowResult;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::Path;

    // === Unit Tests ===

    #[test]
    fn diff_options_new() {
        let options = DiffOptions::new(".promptpack");
        assert_eq!(options.source, PathBuf::from(".promptpack"));
        assert_eq!(options.scope, Scope::Project);
    }

    #[test]
    fn diff_options_with_scope() {
        let options = DiffOptions::new(".promptpack").with_scope(Scope::User);
        assert_eq!(options.scope, Scope::User);
    }

    #[test]
    fn diff_result_has_changes() {
        let mut result = DiffResult::default();
        assert!(!result.has_changes());

        result.creates.push(DiffEntry {
            path: PathBuf::from("test.md"),
            change_type: ChangeType::Create,
            new_content: Some("content".to_string()),
        });
        assert!(result.has_changes());
    }

    #[test]
    fn diff_result_has_conflicts() {
        let mut result = DiffResult::default();
        assert!(!result.has_conflicts());

        result.conflicts.push(DiffEntry {
            path: PathBuf::from("test.md"),
            change_type: ChangeType::Conflict,
            new_content: None,
        });
        assert!(result.has_conflicts());
    }

    #[test]
    fn diff_result_total_affected() {
        let mut result = DiffResult::default();
        assert_eq!(result.total_affected(), 0);

        result.creates.push(DiffEntry {
            path: PathBuf::from("a.md"),
            change_type: ChangeType::Create,
            new_content: None,
        });
        result.updates.push(DiffEntry {
            path: PathBuf::from("b.md"),
            change_type: ChangeType::Update,
            new_content: None,
        });
        assert_eq!(result.total_affected(), 2);
    }

    #[test]
    fn change_type_variants() {
        assert_ne!(ChangeType::Create, ChangeType::Update);
        assert_ne!(ChangeType::Update, ChangeType::Skip);
        assert_ne!(ChangeType::Skip, ChangeType::Conflict);
    }

    // === Integration Tests with Mocks ===

    struct MockAssetRepository {
        assets: Vec<Asset>,
    }

    impl AssetRepository for MockAssetRepository {
        fn load_all(&self, _source: &Path) -> AnyhowResult<Vec<Asset>> {
            Ok(self.assets.clone())
        }

        fn load_by_path(&self, path: &Path) -> AnyhowResult<Asset> {
            self.assets
                .iter()
                .find(|a| a.source_path() == path)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Asset not found: {}", path.display()))
        }
    }

    struct MockLockfileRepository {
        lockfile: RefCell<Lockfile>,
    }

    impl LockfileRepository for MockLockfileRepository {
        fn load_or_new(&self, _path: &Path) -> Lockfile {
            self.lockfile.borrow().clone()
        }

        fn load(&self, _path: &Path) -> Result<Lockfile, LockfileError> {
            Ok(self.lockfile.borrow().clone())
        }

        fn save(&self, lockfile: &Lockfile, _path: &Path) -> Result<(), LockfileError> {
            *self.lockfile.borrow_mut() = lockfile.clone();
            Ok(())
        }
    }

    struct MockFileSystem {
        files: RefCell<HashMap<PathBuf, String>>,
    }

    impl FileSystem for MockFileSystem {
        fn read(&self, path: &Path) -> FsResult<String> {
            self.files
                .borrow()
                .get(path)
                .cloned()
                .ok_or(FsError::NotFound(path.to_path_buf()))
        }

        fn write(&self, path: &Path, content: &str) -> FsResult<()> {
            self.files
                .borrow_mut()
                .insert(path.to_path_buf(), content.to_string());
            Ok(())
        }

        fn exists(&self, path: &Path) -> bool {
            self.files.borrow().contains_key(path)
        }

        fn remove(&self, path: &Path) -> FsResult<()> {
            self.files.borrow_mut().remove(path);
            Ok(())
        }

        fn create_dir_all(&self, _path: &Path) -> FsResult<()> {
            Ok(())
        }

        fn hash(&self, path: &Path) -> FsResult<String> {
            self.files
                .borrow()
                .get(path)
                .map(|content| {
                    use sha2::{Digest, Sha256};
                    let mut hasher = Sha256::new();
                    hasher.update(content.as_bytes());
                    format!("sha256:{:x}", hasher.finalize())
                })
                .ok_or(FsError::NotFound(path.to_path_buf()))
        }

        fn expand_home(&self, path: &Path) -> PathBuf {
            path.to_path_buf()
        }
    }

    struct MockAdapter {
        target: Target,
    }

    impl TargetAdapter for MockAdapter {
        fn target(&self) -> Target {
            self.target
        }

        fn compile(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError> {
            Ok(vec![OutputFile::new(
                format!(".test/{}.md", asset.id()),
                asset.content().to_string(),
                self.target,
            )])
        }

        fn validate(&self, _output: &OutputFile) -> Vec<AdapterDiagnostic> {
            vec![]
        }
    }

    fn create_diff_use_case(
    ) -> DiffUseCase<MockAssetRepository, MockLockfileRepository, MockFileSystem> {
        let asset_repo = MockAssetRepository {
            assets: vec![Asset::new(
                "test",
                "test.md",
                "Test asset",
                "# Test Content",
            )],
        };
        let lockfile_repo = MockLockfileRepository {
            lockfile: RefCell::new(Lockfile::new()),
        };
        let file_system = MockFileSystem {
            files: RefCell::new(HashMap::new()),
        };
        let adapters: Vec<Box<dyn TargetAdapter>> = vec![Box::new(MockAdapter {
            target: Target::ClaudeCode,
        })];

        DiffUseCase::new(asset_repo, lockfile_repo, file_system, adapters)
    }

    #[test]
    fn diff_use_case_detects_new_files() {
        let use_case = create_diff_use_case();
        let options = DiffOptions::new(".promptpack");

        let result = use_case.execute(&options);

        assert_eq!(result.asset_count, 1);
        assert_eq!(result.output_count, 1);
        // File doesn't exist, so it should be a create
        assert_eq!(result.creates.len(), 1);
        assert!(result.updates.is_empty());
        assert!(result.skipped.is_empty());
        assert!(result.has_changes());
    }

    #[test]
    fn diff_use_case_detects_no_changes_when_up_to_date() {
        let asset_repo = MockAssetRepository {
            assets: vec![Asset::new(
                "test",
                "test.md",
                "Test asset",
                "# Test Content",
            )],
        };
        let lockfile_repo = MockLockfileRepository {
            lockfile: RefCell::new(Lockfile::new()),
        };

        // Pre-populate file system with same content
        let mut files = HashMap::new();
        files.insert(PathBuf::from(".test/test.md"), "# Test Content".to_string());
        let file_system = MockFileSystem {
            files: RefCell::new(files),
        };

        let adapters: Vec<Box<dyn TargetAdapter>> = vec![Box::new(MockAdapter {
            target: Target::ClaudeCode,
        })];

        let use_case = DiffUseCase::new(asset_repo, lockfile_repo, file_system, adapters);
        let options = DiffOptions::new(".promptpack");

        let result = use_case.execute(&options);

        // File exists with same content, should be skipped
        assert!(result.creates.is_empty());
        assert!(result.updates.is_empty());
        assert_eq!(result.skipped.len(), 1);
        assert!(!result.has_changes());
    }

    #[test]
    fn diff_use_case_detects_updates_when_content_differs() {
        let asset_repo = MockAssetRepository {
            assets: vec![Asset::new("test", "test.md", "Test asset", "# New Content")],
        };

        // Create lockfile with old hash
        let mut lockfile = Lockfile::new();
        let old_content = "# Old Content";
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(old_content.as_bytes());
        let old_hash = format!("sha256:{:x}", hasher.finalize());
        lockfile.set("project:.test/test.md", &old_hash);

        let lockfile_repo = MockLockfileRepository {
            lockfile: RefCell::new(lockfile),
        };

        // File exists with old content (matching lockfile)
        let mut files = HashMap::new();
        files.insert(PathBuf::from(".test/test.md"), old_content.to_string());
        let file_system = MockFileSystem {
            files: RefCell::new(files),
        };

        let adapters: Vec<Box<dyn TargetAdapter>> = vec![Box::new(MockAdapter {
            target: Target::ClaudeCode,
        })];

        let use_case = DiffUseCase::new(asset_repo, lockfile_repo, file_system, adapters);
        let options = DiffOptions::new(".promptpack");

        let result = use_case.execute(&options);

        // File exists but content differs, should be an update
        assert!(result.creates.is_empty());
        assert_eq!(result.updates.len(), 1);
        assert!(result.skipped.is_empty());
        assert!(result.has_changes());
    }

    #[test]
    fn diff_use_case_detects_conflicts() {
        let asset_repo = MockAssetRepository {
            assets: vec![Asset::new("test", "test.md", "Test asset", "# New Content")],
        };

        // Create lockfile with different hash than current file
        let mut lockfile = Lockfile::new();
        lockfile.set("project:.test/test.md", "sha256:different_hash");

        let lockfile_repo = MockLockfileRepository {
            lockfile: RefCell::new(lockfile),
        };

        // File exists with modified content (different from lockfile)
        let mut files = HashMap::new();
        files.insert(
            PathBuf::from(".test/test.md"),
            "# Modified by user".to_string(),
        );
        let file_system = MockFileSystem {
            files: RefCell::new(files),
        };

        let adapters: Vec<Box<dyn TargetAdapter>> = vec![Box::new(MockAdapter {
            target: Target::ClaudeCode,
        })];

        let use_case = DiffUseCase::new(asset_repo, lockfile_repo, file_system, adapters);
        let options = DiffOptions::new(".promptpack");

        let result = use_case.execute(&options);

        // File modified since last sync - conflict
        assert!(result.has_conflicts());
        assert_eq!(result.conflicts.len(), 1);
    }
}
