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
//!
//! # Size Justification
//!
//! calvin-no-split: This file is intentionally kept as a single unit because:
//! - All 20 methods belong to a single `DeployUseCase` struct
//! - Methods form a cohesive deployment pipeline
//! - Splitting would break encapsulation of private helper methods
//! - The struct follows the UseCase pattern from Clean Architecture

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::domain::entities::{Asset, Lockfile, OutputFile};
use crate::domain::ports::{
    AssetRepository, ConflictChoice, ConflictContext, ConflictResolver, DeployEvent,
    DeployEventSink, FileSystem, ForceResolver, FsResult, LockfileRepository, NoopEventSink,
    SafeResolver, TargetAdapter,
};
use crate::domain::services::{
    has_calvin_signature, FileAction, OrphanDetectionResult, OrphanDetector, PlannedFile, Planner,
    SyncPlan, TargetFileState,
};
use crate::domain::value_objects::{Scope, Target};

use super::options::{DeployOptions, DeployOutputOptions};
use super::result::DeployResult;

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
        // Select appropriate resolver based on options
        let resolver: Arc<dyn ConflictResolver> = if options.force {
            Arc::new(ForceResolver)
        } else {
            Arc::new(SafeResolver)
        };
        self.execute_full(options, Arc::new(NoopEventSink), resolver)
    }

    /// Execute the deploy use case with event reporting
    ///
    /// This method emits events during execution, enabling:
    /// - Progress reporting
    /// - JSON event streaming
    /// - Debugging and observability
    pub fn execute_with_events(
        &self,
        options: &DeployOptions,
        event_sink: Arc<dyn DeployEventSink>,
    ) -> DeployResult {
        let resolver: Arc<dyn ConflictResolver> = if options.force {
            Arc::new(ForceResolver)
        } else {
            Arc::new(SafeResolver)
        };
        self.execute_full(options, event_sink, resolver)
    }

    /// Execute the deploy use case with a custom conflict resolver
    ///
    /// Use this for interactive conflict resolution.
    pub fn execute_with_resolver(
        &self,
        options: &DeployOptions,
        resolver: Arc<dyn ConflictResolver>,
    ) -> DeployResult {
        self.execute_full(options, Arc::new(NoopEventSink), resolver)
    }

    /// Deploy pre-compiled outputs directly
    ///
    /// This method is used by the watcher command for incremental sync.
    /// It skips asset loading and compilation, starting directly from OutputFile[].
    pub fn deploy_outputs(
        &self,
        outputs: Vec<OutputFile>,
        options: &DeployOutputOptions,
    ) -> DeployResult {
        self.deploy_outputs_full(
            outputs,
            options,
            Arc::new(NoopEventSink),
            Arc::new(SafeResolver),
        )
    }

    /// Deploy pre-compiled outputs with custom resolver
    pub fn deploy_outputs_with_resolver(
        &self,
        outputs: Vec<OutputFile>,
        options: &DeployOutputOptions,
        resolver: Arc<dyn ConflictResolver>,
    ) -> DeployResult {
        self.deploy_outputs_full(outputs, options, Arc::new(NoopEventSink), resolver)
    }

    /// Full deploy outputs with all customization options
    fn deploy_outputs_full(
        &self,
        outputs: Vec<OutputFile>,
        options: &DeployOutputOptions,
        event_sink: Arc<dyn DeployEventSink>,
        resolver: Arc<dyn ConflictResolver>,
    ) -> DeployResult {
        let mut result = DeployResult::new();
        result.output_count = outputs.len();

        // Emit started event
        event_sink.on_event(DeployEvent::Started {
            source: options.lockfile_path.clone(),
            destination: format!("{:?}", options.scope),
            asset_count: outputs.len(),
        });

        // Emit compiled event (already compiled)
        event_sink.on_event(DeployEvent::Compiled {
            output_count: outputs.len(),
        });

        // Step 1: Load lockfile
        let lockfile = self.lockfile_repo.load_or_new(&options.lockfile_path);

        // Step 2: Plan sync
        let plan = self.plan_sync(
            &outputs,
            &lockfile,
            &DeployOptions {
                source: options.lockfile_path.clone(),
                scope: options.scope,
                targets: vec![],
                force: false,
                interactive: false,
                dry_run: options.dry_run,
                clean_orphans: options.clean_orphans,
            },
        );

        // Step 3: Resolve conflicts
        let resolved_plan = match self.resolve_conflicts(plan, &resolver, options.scope) {
            Ok(plan) => plan,
            Err(_) => {
                result.errors.push("Operation aborted by user".to_string());
                return result;
            }
        };

        // Step 4: Detect orphans
        let orphans = if options.clean_orphans {
            self.detect_orphans(&lockfile, &outputs, options.scope)
        } else {
            OrphanDetectionResult::default()
        };

        // Step 5: Execute (if not dry run)
        if !options.dry_run {
            self.execute_plan_with_events(&resolved_plan, &mut result, &event_sink);
            self.delete_orphans_with_events(&orphans, &mut result, &event_sink);
            if let Some(warning) = self.update_lockfile(
                &options.lockfile_path,
                &resolved_plan,
                &result,
                options.scope,
            ) {
                result.add_warning(warning);
            }
        } else {
            for file in resolved_plan.to_write() {
                result.written.push(file.path.clone());
            }
            for orphan in &orphans.orphans {
                result.deleted.push(PathBuf::from(&orphan.path));
            }
        }

        // Emit completed event
        event_sink.on_event(DeployEvent::Completed {
            written_count: result.written.len(),
            skipped_count: result.skipped.len(),
            error_count: result.errors.len(),
            deleted_count: result.deleted.len(),
        });

        result
    }

    /// Full execute with all customization options
    fn execute_full(
        &self,
        options: &DeployOptions,
        event_sink: Arc<dyn DeployEventSink>,
        resolver: Arc<dyn ConflictResolver>,
    ) -> DeployResult {
        let mut result = DeployResult::new();

        // Step 1: Load assets
        let assets = match self.load_assets(&options.source) {
            Ok(assets) => assets,
            Err(e) => {
                result.errors.push(format!("Failed to load assets: {}", e));
                return result;
            }
        };

        // Step 1.5: Apply scope policy - when deploying to User scope, force all assets to User
        let assets = self.apply_scope_policy(assets, options.scope);
        result.asset_count = assets.len();

        // Emit started event
        event_sink.on_event(DeployEvent::Started {
            source: options.source.clone(),
            destination: format!("{:?}", options.scope),
            asset_count: assets.len(),
        });

        // Step 2: Compile assets
        let outputs = match self.compile_assets(&assets, &options.targets) {
            Ok(outputs) => outputs,
            Err(e) => {
                result.errors.push(format!("Compilation failed: {}", e));
                return result;
            }
        };
        result.output_count = outputs.len();

        // Emit compiled event
        event_sink.on_event(DeployEvent::Compiled {
            output_count: outputs.len(),
        });

        // Step 3: Load lockfile
        let lockfile_path = self.get_lockfile_path(&options.source, options.scope);
        let lockfile = self.lockfile_repo.load_or_new(&lockfile_path);

        // Step 4: Plan sync
        let plan = self.plan_sync(&outputs, &lockfile, options);

        // Step 4.5: Resolve conflicts
        let resolved_plan = match self.resolve_conflicts(plan, &resolver, options.scope) {
            Ok(plan) => plan,
            Err(_) => {
                // User aborted
                result.errors.push("Operation aborted by user".to_string());
                return result;
            }
        };

        // Step 5: Detect orphans
        let orphans = if options.clean_orphans {
            self.detect_orphans(&lockfile, &outputs, options.scope)
        } else {
            OrphanDetectionResult::default()
        };

        // Step 6: Execute (if not dry run)
        if !options.dry_run {
            self.execute_plan_with_events(&resolved_plan, &mut result, &event_sink);
            self.delete_orphans_with_events(&orphans, &mut result, &event_sink);
            if let Some(warning) =
                self.update_lockfile(&lockfile_path, &resolved_plan, &result, options.scope)
            {
                result.add_warning(warning);
            }
        } else {
            // Dry run - just collect what would happen
            for file in resolved_plan.to_write() {
                result.written.push(file.path.clone());
            }
            for orphan in &orphans.orphans {
                result.deleted.push(PathBuf::from(&orphan.path));
            }
        }

        // Emit completed event
        event_sink.on_event(DeployEvent::Completed {
            written_count: result.written.len(),
            skipped_count: result.skipped.len(),
            error_count: result.errors.len(),
            deleted_count: result.deleted.len(),
        });

        result
    }

    /// Load assets from source directory
    fn load_assets(&self, source: &Path) -> Result<Vec<Asset>, String> {
        self.asset_repo
            .load_all(source)
            .map_err(|e| format!("{}", e))
    }

    /// Apply scope policy to assets
    ///
    /// When deploying to User scope (--home), force all assets to use User scope.
    /// This matches the behavior of the old engine's ScopePolicy::ForceUser.
    fn apply_scope_policy(&self, assets: Vec<Asset>, target_scope: Scope) -> Vec<Asset> {
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
            // Get the effective targets for this asset (respects asset-level targets field)
            let asset_targets = asset.effective_targets();

            for adapter in &active_adapters {
                // Skip if this adapter's target is not enabled for this asset
                if !asset_targets.contains(&adapter.target()) {
                    continue;
                }

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
    /// Resolve conflicts in the plan using the provided resolver
    fn resolve_conflicts(
        &self,
        mut plan: SyncPlan,
        resolver: &Arc<dyn ConflictResolver>,
        _scope: Scope,
    ) -> Result<SyncPlan, ()> {
        // Check if there are any conflicts
        if !plan.has_conflicts() {
            return Ok(plan);
        }

        // Track "apply all" state
        let mut apply_all: Option<ConflictChoice> = None;

        // Collect files that need to be resolved
        let mut resolved_files = Vec::new();

        for file in plan.files.drain(..) {
            if !file.is_conflict() {
                resolved_files.push(file);
                continue;
            }

            // Get the conflict reason
            let conflict_reason = match &file.action {
                FileAction::Conflict(r) => *r,
                _ => continue, // Not a conflict
            };

            // Check "apply all" first
            if let Some(choice) = apply_all {
                let resolved = match choice {
                    ConflictChoice::Overwrite => file.resolve_overwrite(),
                    ConflictChoice::Skip => file.resolve_skip(),
                    _ => file,
                };
                resolved_files.push(resolved);
                continue;
            }

            // Read existing content for context
            let existing_content = self
                .file_system
                .read(&file.path)
                .unwrap_or_else(|_| String::new());

            // Map planner's ConflictReason to port's ConflictReason
            let port_reason = match conflict_reason {
                crate::domain::services::ConflictReason::Modified => {
                    crate::domain::ports::ConflictReason::Modified
                }
                crate::domain::services::ConflictReason::Untracked => {
                    crate::domain::ports::ConflictReason::Untracked
                }
            };

            // Create context
            let context = ConflictContext {
                path: &file.path,
                reason: port_reason,
                existing_content: &existing_content,
                new_content: &file.content,
            };

            // Resolve in a loop (to handle Diff choice)
            loop {
                let choice = resolver.resolve(&context);

                match choice {
                    ConflictChoice::Overwrite => {
                        resolved_files.push(file.resolve_overwrite());
                        break;
                    }
                    ConflictChoice::Skip => {
                        resolved_files.push(file.resolve_skip());
                        break;
                    }
                    ConflictChoice::Diff => {
                        // Generate and show diff
                        let diff = self.generate_diff(&file.path, &existing_content, &file.content);
                        resolver.show_diff(&diff);
                        // Continue loop to ask again
                    }
                    ConflictChoice::Abort => {
                        return Err(());
                    }
                    ConflictChoice::OverwriteAll => {
                        apply_all = Some(ConflictChoice::Overwrite);
                        resolved_files.push(file.resolve_overwrite());
                        break;
                    }
                    ConflictChoice::SkipAll => {
                        apply_all = Some(ConflictChoice::Skip);
                        resolved_files.push(file.resolve_skip());
                        break;
                    }
                }
            }
        }

        // Rebuild plan with resolved files
        let mut new_plan = SyncPlan::new();
        for file in resolved_files {
            new_plan.add(file);
        }

        Ok(new_plan)
    }

    /// Generate a unified diff between old and new content
    fn generate_diff(&self, path: &Path, old: &str, new: &str) -> String {
        use similar::TextDiff;
        TextDiff::from_lines(old, new)
            .unified_diff()
            .header(
                &format!("a/{}", path.display()),
                &format!("b/{}", path.display()),
            )
            .to_string()
    }

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
            let action = if options.force {
                // Force mode - skip content-identical files, overwrite all others
                if target_state.exists && target_state.current_hash.as_ref() == Some(&new_hash) {
                    FileAction::Skip
                } else {
                    FileAction::Write
                }
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
        let mut result = OrphanDetector::detect(lockfile, outputs, scope);

        // Check status of each orphan (exists, has_signature)
        for orphan in &mut result.orphans {
            let path = self.file_system.expand_home(&PathBuf::from(&orphan.path));

            orphan.exists = self.file_system.exists(&path);

            if orphan.exists {
                if let Ok(content) = self.file_system.read(&path) {
                    orphan.has_signature = has_calvin_signature(&content);
                }
            }
        }

        result
    }

    /// Execute the sync plan
    /// Execute the sync plan with event reporting
    fn execute_plan_with_events(
        &self,
        plan: &SyncPlan,
        result: &mut DeployResult,
        event_sink: &Arc<dyn DeployEventSink>,
    ) {
        for (index, file) in plan.files.iter().enumerate() {
            match &file.action {
                FileAction::Write => match self.write_file(&file.path, &file.content) {
                    Ok(_) => {
                        result.written.push(file.path.clone());
                        event_sink.on_event(DeployEvent::FileWritten {
                            index,
                            path: file.path.clone(),
                        });
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to write {}: {}", file.path.display(), e);
                        result.errors.push(error_msg.clone());
                        event_sink.on_event(DeployEvent::FileError {
                            index,
                            path: file.path.clone(),
                            error: error_msg,
                        });
                    }
                },
                FileAction::Skip => {
                    result.skipped.push(file.path.clone());
                    event_sink.on_event(DeployEvent::FileSkipped {
                        index,
                        path: file.path.clone(),
                        reason: "unchanged".to_string(),
                    });
                }
                FileAction::Conflict(reason) => {
                    // Conflicts are treated as skipped in non-interactive mode
                    result.skipped.push(file.path.clone());
                    event_sink.on_event(DeployEvent::FileSkipped {
                        index,
                        path: file.path.clone(),
                        reason: format!("conflict: {:?}", reason),
                    });
                }
            }
        }
    }

    /// Delete orphan files
    /// Delete orphan files with event reporting
    fn delete_orphans_with_events(
        &self,
        orphans: &OrphanDetectionResult,
        result: &mut DeployResult,
        event_sink: &Arc<dyn DeployEventSink>,
    ) {
        // Emit orphans detected event
        if !orphans.orphans.is_empty() {
            let safe_count = orphans
                .orphans
                .iter()
                .filter(|o| o.is_safe_to_delete())
                .count();
            event_sink.on_event(DeployEvent::OrphansDetected {
                total: orphans.orphans.len(),
                safe_to_delete: safe_count,
            });
        }

        for orphan in &orphans.orphans {
            let path = PathBuf::from(&orphan.path);
            if orphan.exists && orphan.is_safe_to_delete() {
                if let Err(e) = self.file_system.remove(&path) {
                    result
                        .errors
                        .push(format!("Failed to delete {}: {}", path.display(), e));
                } else {
                    result.deleted.push(path.clone());
                    event_sink.on_event(DeployEvent::OrphanDeleted { path });
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
    ///
    /// Returns an optional warning message if the lockfile could not be saved
    fn update_lockfile(
        &self,
        path: &Path,
        plan: &SyncPlan,
        result: &DeployResult,
        scope: Scope,
    ) -> Option<String> {
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
            Some(format!("Failed to save lockfile: {}", e))
        } else {
            None
        }
    }
}
