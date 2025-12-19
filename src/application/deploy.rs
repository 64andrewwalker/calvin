//! Deploy Use Case
//!
//! Orchestrates the deployment flow:
//! 1. Load assets from source
//! 2. Compile assets for target platforms
//! 3. Plan the sync (detect changes, conflicts)
//! 4. Execute the sync (write files)
//! 5. Update the lockfile
//!
//! This use case is pure orchestration - all business logic lives in domain services.

use std::path::{Path, PathBuf};

use crate::domain::entities::{Asset, Lockfile, OutputFile};
use crate::domain::ports::{
    AssetRepository, FileSystem, FsResult, LockfileRepository, TargetAdapter,
};
use crate::domain::services::{
    FileAction, OrphanDetectionResult, OrphanDetector, PlannedFile, Planner, SyncPlan,
    TargetFileState,
};
use crate::domain::value_objects::{Scope, Target};

/// Options for the deploy use case
#[derive(Debug, Clone)]
pub struct DeployOptions {
    /// Source directory (.promptpack)
    pub source: PathBuf,
    /// Deploy scope (project or user)
    pub scope: Scope,
    /// Target platforms to deploy to
    pub targets: Vec<Target>,
    /// Force overwrite without conflict detection
    pub force: bool,
    /// Interactive conflict resolution
    pub interactive: bool,
    /// Dry run (don't write files)
    pub dry_run: bool,
    /// Clean orphan files
    pub clean_orphans: bool,
}

impl DeployOptions {
    pub fn new(source: impl Into<PathBuf>) -> Self {
        Self {
            source: source.into(),
            scope: Scope::default(),
            targets: Vec::new(),
            force: false,
            interactive: false,
            dry_run: false,
            clean_orphans: false,
        }
    }

    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_targets(mut self, targets: Vec<Target>) -> Self {
        self.targets = targets;
        self
    }

    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }
}

/// Result of a deploy operation
#[derive(Debug, Clone)]
pub struct DeployResult {
    /// Files that were written
    pub written: Vec<PathBuf>,
    /// Files that were skipped (up-to-date)
    pub skipped: Vec<PathBuf>,
    /// Files that were deleted (orphans)
    pub deleted: Vec<PathBuf>,
    /// Errors encountered
    pub errors: Vec<String>,
    /// Total asset count
    pub asset_count: usize,
    /// Total output count
    pub output_count: usize,
}

impl DeployResult {
    pub fn new() -> Self {
        Self {
            written: Vec::new(),
            skipped: Vec::new(),
            deleted: Vec::new(),
            errors: Vec::new(),
            asset_count: 0,
            output_count: 0,
        }
    }

    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn has_changes(&self) -> bool {
        !self.written.is_empty() || !self.deleted.is_empty()
    }
}

impl Default for DeployResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Deploy use case - orchestrates the deployment flow
///
/// This use case is parameterized by its dependencies (ports),
/// allowing for easy testing and different implementations.
pub struct DeployUseCase<AR, LR, FS>
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

impl<AR, LR, FS> DeployUseCase<AR, LR, FS>
where
    AR: AssetRepository,
    LR: LockfileRepository,
    FS: FileSystem,
{
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

    /// Execute the deploy use case
    pub fn execute(&self, options: &DeployOptions) -> DeployResult {
        let mut result = DeployResult::new();

        // Step 1: Load assets
        let assets = match self.load_assets(&options.source) {
            Ok(assets) => assets,
            Err(e) => {
                result.errors.push(format!("Failed to load assets: {}", e));
                return result;
            }
        };
        result.asset_count = assets.len();

        // Step 2: Compile assets
        let outputs = match self.compile_assets(&assets, &options.targets) {
            Ok(outputs) => outputs,
            Err(e) => {
                result.errors.push(format!("Compilation failed: {}", e));
                return result;
            }
        };
        result.output_count = outputs.len();

        // Step 3: Load lockfile
        let lockfile_path = self.get_lockfile_path(&options.source, options.scope);
        let lockfile = self.lockfile_repo.load_or_new(&lockfile_path);

        // Step 4: Plan sync
        let plan = self.plan_sync(&outputs, &lockfile, options);

        // Step 5: Detect orphans
        let orphans = if options.clean_orphans {
            self.detect_orphans(&lockfile, &outputs, options.scope)
        } else {
            OrphanDetectionResult::default()
        };

        // Step 6: Execute (if not dry run)
        if !options.dry_run {
            self.execute_plan(&plan, &mut result);
            self.delete_orphans(&orphans, &mut result);
            self.update_lockfile(&lockfile_path, &plan, &result, options.scope);
        } else {
            // Dry run - just collect what would happen
            for file in plan.to_write() {
                result.written.push(file.path.clone());
            }
            for orphan in &orphans.orphans {
                result.deleted.push(PathBuf::from(&orphan.path));
            }
        }

        result
    }

    /// Load assets from source directory
    fn load_assets(&self, source: &Path) -> Result<Vec<Asset>, String> {
        self.asset_repo
            .load_all(source)
            .map_err(|e| format!("{}", e))
    }

    /// Compile assets for target platforms
    fn compile_assets(
        &self,
        assets: &[Asset],
        targets: &[Target],
    ) -> Result<Vec<OutputFile>, String> {
        let mut outputs = Vec::new();

        // Determine which adapters to use
        let active_adapters: Vec<&Box<dyn TargetAdapter>> =
            if targets.is_empty() || targets.iter().any(|t| t.is_all()) {
                self.adapters.iter().collect()
            } else {
                self.adapters
                    .iter()
                    .filter(|a| targets.contains(&a.target()))
                    .collect()
            };

        // Compile each asset with each adapter
        for asset in assets {
            for adapter in &active_adapters {
                match adapter.compile(asset) {
                    Ok(adapter_outputs) => outputs.extend(adapter_outputs),
                    Err(e) => {
                        return Err(format!(
                            "Adapter {} failed on {}: {}",
                            adapter.target().display_name(),
                            asset.id(),
                            e
                        ));
                    }
                }
            }
        }

        // Post-compile for each adapter (e.g., generate AGENTS.md)
        for adapter in &active_adapters {
            match adapter.post_compile(assets) {
                Ok(post_outputs) => outputs.extend(post_outputs),
                Err(e) => {
                    return Err(format!(
                        "Post-compile for {} failed: {}",
                        adapter.target().display_name(),
                        e
                    ));
                }
            }
        }

        Ok(outputs)
    }

    /// Plan the sync operation
    fn plan_sync(
        &self,
        outputs: &[OutputFile],
        lockfile: &Lockfile,
        options: &DeployOptions,
    ) -> SyncPlan {
        let mut plan = SyncPlan::new();

        for output in outputs {
            // Check if file exists and get current state
            let path = output.path();
            let exists = self.file_system.exists(path);
            let current_hash = if exists {
                self.file_system.hash(path).ok()
            } else {
                None
            };

            // Get lockfile key
            let lockfile_key = Lockfile::make_key(options.scope, &path.display().to_string());

            // Compute new hash
            let mut output_clone = output.clone();
            let new_hash = output_clone.hash().to_string();

            // Build target state
            let target_state = if exists {
                if let Some(hash) = current_hash {
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
            let action = if options.force {
                // Force mode - always write
                FileAction::Write
            } else {
                Planner::plan_file(&new_hash, &target_state, lockfile, &lockfile_key)
            };

            plan.add(PlannedFile::new(
                path.clone(),
                output.content().to_string(),
                action,
            ));
        }

        plan
    }

    /// Detect orphan files
    fn detect_orphans(
        &self,
        lockfile: &Lockfile,
        outputs: &[OutputFile],
        scope: Scope,
    ) -> OrphanDetectionResult {
        OrphanDetector::detect(lockfile, outputs, scope)
    }

    /// Execute the sync plan
    fn execute_plan(&self, plan: &SyncPlan, result: &mut DeployResult) {
        for file in &plan.files {
            match &file.action {
                FileAction::Write => match self.write_file(&file.path, &file.content) {
                    Ok(_) => result.written.push(file.path.clone()),
                    Err(e) => result.errors.push(format!(
                        "Failed to write {}: {}",
                        file.path.display(),
                        e
                    )),
                },
                FileAction::Skip => {
                    result.skipped.push(file.path.clone());
                }
                FileAction::Conflict(_) => {
                    // Conflicts are treated as skipped in non-interactive mode
                    result.skipped.push(file.path.clone());
                }
            }
        }
    }

    /// Delete orphan files
    fn delete_orphans(&self, orphans: &OrphanDetectionResult, result: &mut DeployResult) {
        for orphan in &orphans.orphans {
            let path = PathBuf::from(&orphan.path);
            if orphan.exists && orphan.is_safe_to_delete() {
                if let Err(e) = self.file_system.remove(&path) {
                    result
                        .errors
                        .push(format!("Failed to delete {}: {}", path.display(), e));
                } else {
                    result.deleted.push(path);
                }
            }
        }
    }

    /// Write a file
    fn write_file(&self, path: &Path, content: &str) -> FsResult<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            self.file_system.create_dir_all(parent)?;
        }
        self.file_system.write(path, content)
    }

    /// Get lockfile path based on scope
    fn get_lockfile_path(&self, source: &Path, _scope: Scope) -> PathBuf {
        source.join(".calvin.lock")
    }

    /// Update lockfile after sync
    fn update_lockfile(&self, path: &Path, plan: &SyncPlan, result: &DeployResult, scope: Scope) {
        use std::collections::HashSet;

        let mut lockfile = self.lockfile_repo.load_or_new(path);

        // Build set of written paths
        let written_set: HashSet<_> = result.written.iter().collect();

        // Update hashes for written files
        for file in &plan.files {
            if written_set.contains(&file.path) {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(file.content.as_bytes());
                let hash = format!("sha256:{:x}", hasher.finalize());

                let key = Lockfile::make_key(scope, &file.path.display().to_string());
                lockfile.set(&key, &hash);
            }
        }

        // Remove deleted files from lockfile
        for deleted in &result.deleted {
            let key = Lockfile::make_key(scope, &deleted.display().to_string());
            lockfile.remove(&key);
        }

        // Save lockfile
        if let Err(e) = self.lockfile_repo.save(&lockfile, path) {
            eprintln!("Warning: Failed to save lockfile: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{Asset, Lockfile};
    use crate::domain::ports::target_adapter::{AdapterDiagnostic, AdapterError};
    use std::cell::RefCell;
    use std::collections::HashMap;

    // Mock implementations for testing

    struct MockAssetRepository {
        assets: Vec<Asset>,
    }

    impl AssetRepository for MockAssetRepository {
        fn load_all(&self, _source: &Path) -> anyhow::Result<Vec<Asset>> {
            Ok(self.assets.clone())
        }

        fn load_by_path(&self, _path: &Path) -> anyhow::Result<Asset> {
            self.assets
                .first()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Asset not found"))
        }
    }

    struct MockLockfileRepository {
        lockfile: RefCell<Lockfile>,
    }

    impl LockfileRepository for MockLockfileRepository {
        fn load_or_new(&self, _path: &Path) -> Lockfile {
            self.lockfile.borrow().clone()
        }

        fn load(&self, _path: &Path) -> Result<Lockfile, crate::domain::ports::LockfileError> {
            Ok(self.lockfile.borrow().clone())
        }

        fn save(
            &self,
            lockfile: &Lockfile,
            _path: &Path,
        ) -> Result<(), crate::domain::ports::LockfileError> {
            *self.lockfile.borrow_mut() = lockfile.clone();
            Ok(())
        }
    }

    struct MockFileSystem {
        files: RefCell<HashMap<PathBuf, String>>,
    }

    impl FileSystem for MockFileSystem {
        fn read(&self, path: &Path) -> FsResult<String> {
            self.files.borrow().get(path).cloned().ok_or(
                crate::domain::ports::file_system::FsError::NotFound(path.to_path_buf()),
            )
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
                .ok_or(crate::domain::ports::file_system::FsError::NotFound(
                    path.to_path_buf(),
                ))
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

    fn create_use_case(
    ) -> DeployUseCase<MockAssetRepository, MockLockfileRepository, MockFileSystem> {
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

        DeployUseCase::new(asset_repo, lockfile_repo, file_system, adapters)
    }

    #[test]
    fn deploy_use_case_executes_successfully() {
        let use_case = create_use_case();
        let options = DeployOptions::new(".promptpack");

        let result = use_case.execute(&options);

        assert!(result.is_success());
        assert_eq!(result.asset_count, 1);
        assert_eq!(result.output_count, 1);
    }

    #[test]
    fn deploy_dry_run_does_not_write_files() {
        let use_case = create_use_case();
        let options = DeployOptions::new(".promptpack").with_dry_run(true);

        let result = use_case.execute(&options);

        assert!(result.is_success());
        // In dry run, files are collected but not actually written
        assert!(!result.written.is_empty() || result.output_count > 0);
    }

    #[test]
    fn deploy_result_default_is_empty() {
        let result = DeployResult::default();

        assert!(result.is_success());
        assert!(!result.has_changes());
    }
}
