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

use crate::domain::entities::{Asset, BinaryOutputFile, Lockfile, OutputFile};
use crate::domain::ports::{
    AssetRepository, ConflictChoice, ConflictContext, ConflictResolver, DeployEvent,
    DeployEventSink, FileSystem, ForceResolver, FsResult, LockfileRepository, NoopEventSink,
    SafeResolver, TargetAdapter,
};
use crate::domain::services::{
    has_calvin_signature, merge_layers, FileAction, LayerResolveError, LayerResolver, MergedAsset,
    OrphanDetectionResult, OrphanDetector, PlannedFile, Planner, SyncPlan, TargetFileState,
};
use crate::domain::value_objects::{Scope, Target};

use super::options::{DeployOptions, DeployOutputOptions};
use super::result::DeployResult;
use crate::application::layer_ops::load_resolved_layers;
use crate::application::skills::skill_root_from_path;
use crate::application::RegistryUseCase;
use crate::config::default_user_layer_path;

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
    registry_use_case: Option<Arc<RegistryUseCase>>,
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
            registry_use_case: None,
        }
    }

    pub fn with_registry_use_case(mut self, registry_use_case: Arc<RegistryUseCase>) -> Self {
        self.registry_use_case = Some(registry_use_case);
        self
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
        let project_root = options
            .lockfile_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

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
        let lockfile = match self.lockfile_repo.load(&options.lockfile_path) {
            Ok(lockfile) => lockfile,
            Err(e) => {
                result
                    .errors
                    .push(format!("Failed to load lockfile: {}", e));
                return result;
            }
        };

        // Step 2: Plan sync
        let plan = self.plan_sync(
            &outputs,
            &lockfile,
            &DeployOptions {
                source: options.lockfile_path.clone(),
                project_root: project_root.clone(),
                use_project_layer: true,
                user_layer_path: None,
                use_user_layer: true,
                additional_layers: Vec::new(),
                use_additional_layers: true,
                scope: options.scope,
                targets: vec![],
                remote_mode: false,
                force: false,
                interactive: false,
                dry_run: options.dry_run,
                clean_orphans: options.clean_orphans,
            },
        );

        // Step 3: Resolve conflicts
        let resolved_plan =
            match self.resolve_conflicts(plan, &resolver, &project_root, /* remote */ false) {
                Ok(plan) => plan,
                Err(_) => {
                    result.errors.push("Operation aborted by user".to_string());
                    return result;
                }
            };

        // Step 4: Detect orphans
        let orphans = if options.clean_orphans {
            self.detect_orphans(
                &lockfile,
                &outputs,
                options.scope,
                &project_root,
                /* remote */ false,
            )
        } else {
            OrphanDetectionResult::default()
        };

        // Step 5: Execute (if not dry run)
        if !options.dry_run {
            self.execute_plan_with_events(
                &resolved_plan,
                &mut result,
                &event_sink,
                &project_root,
                /* remote */ false,
            );
            self.delete_orphans_with_events(
                &orphans,
                &mut result,
                &event_sink,
                &project_root,
                /* remote */ false,
            );
            if let Some(warning) = self.update_lockfile(
                &options.lockfile_path,
                &resolved_plan,
                &result,
                options.scope,
                None,
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
        let layered_assets = match self.load_assets_from_layers(options) {
            Ok(assets) => assets,
            Err(e) => {
                result.errors.push(format!("Failed to load assets: {}", e));
                return result;
            }
        };
        for warning in layered_assets.warnings {
            result.add_warning(warning);
        }
        let assets = layered_assets.assets;

        // Step 1.5: Apply scope policy - when deploying to User scope, force all assets to User
        let assets = self.apply_scope_policy(assets, options.scope);
        result.asset_count = assets.len();

        // Step 1.75: Validate skills targets (never fail silently)
        match validate_skill_targets(&assets) {
            Ok(warnings) => {
                for warning in warnings {
                    result.add_warning(warning);
                }
            }
            Err(err) => {
                result.errors.push(err);
                return result;
            }
        }

        // Step 1.8: Warn if deploy targets include platforms that don't support skills.
        // This prevents silent skips when deploying to VS Code / Antigravity.
        for warning in warn_skills_skipped_for_unsupported_deploy_targets(&assets, &options.targets)
        {
            result.add_warning(warning);
        }

        // Step 1.9: Validate OpenCode-specific fields (never fail silently).
        for warning in validate_opencode_assets(&assets, &options.targets) {
            result.add_warning(warning);
        }

        // Emit started event
        event_sink.on_event(DeployEvent::Started {
            source: options.source.clone(),
            destination: format!("{:?}", options.scope),
            asset_count: assets.len(),
        });

        // Step 2: Compile assets
        let (outputs, binary_outputs, provenance_by_output_path) = match self.compile_assets(
            &assets,
            &options.targets,
            &layered_assets.merged_assets_by_id,
        ) {
            Ok(result) => result,
            Err(e) => {
                result.errors.push(format!("Compilation failed: {}", e));
                return result;
            }
        };
        result.output_count = outputs.len();

        // Step 2.1: Surface adapter validation warnings for skill outputs.
        self.validate_skill_outputs(&outputs, &mut result);

        // Emit compiled event
        event_sink.on_event(DeployEvent::Compiled {
            output_count: outputs.len(),
        });

        // Step 3: Load lockfile
        //
        // Project deployments are tracked in `{project_root}/calvin.lock` (with legacy migration).
        // Home/user deployments are global and tracked in `{HOME}/.calvin/calvin.lock`.
        let (lockfile_path, lockfile_warning) = match options.scope {
            Scope::Project => crate::application::resolve_lockfile_path(
                &options.project_root,
                &options.source,
                &self.lockfile_repo,
            ),
            Scope::User => match crate::application::global_lockfile_path() {
                Some(path) => (path, None),
                None => {
                    result
                        .errors
                        .push("Failed to resolve home directory for global lockfile".to_string());
                    return result;
                }
            },
        };
        if let Some(warning) = lockfile_warning {
            result.add_warning(warning);
        }
        let lockfile = match self.lockfile_repo.load(&lockfile_path) {
            Ok(lockfile) => lockfile,
            Err(e) => {
                result
                    .errors
                    .push(format!("Failed to load lockfile: {}", e));
                return result;
            }
        };

        // Step 4: Plan sync
        let plan = self.plan_sync(&outputs, &lockfile, options);

        // Step 4.5: Resolve conflicts
        let resolved_plan = match self.resolve_conflicts(
            plan,
            &resolver,
            &options.project_root,
            options.remote_mode,
        ) {
            Ok(plan) => plan,
            Err(_) => {
                // User aborted
                result.errors.push("Operation aborted by user".to_string());
                return result;
            }
        };

        // Step 5: Detect orphans (including binary outputs)
        let orphans = if options.clean_orphans {
            self.detect_orphans_with_binaries(
                &lockfile,
                &outputs,
                &binary_outputs,
                options.scope,
                &options.project_root,
                options.remote_mode,
            )
        } else {
            OrphanDetectionResult::default()
        };

        // Step 6: Execute (if not dry run)
        if !options.dry_run {
            self.execute_plan_with_events(
                &resolved_plan,
                &mut result,
                &event_sink,
                &options.project_root,
                options.remote_mode,
            );
            // Step 6.1: Write binary outputs
            self.write_binary_outputs(
                &binary_outputs,
                &mut result,
                &options.project_root,
                options.remote_mode,
            );
            self.delete_orphans_with_events(
                &orphans,
                &mut result,
                &event_sink,
                &options.project_root,
                options.remote_mode,
            );
            if let Some(warning) = self.update_lockfile_with_binaries(
                &lockfile_path,
                &resolved_plan,
                &binary_outputs,
                &result,
                options.scope,
                Some(&provenance_by_output_path),
            ) {
                result.add_warning(warning);
            }

            if result.errors.is_empty() && matches!(options.scope, Scope::Project) {
                self.register_project(
                    &options.project_root,
                    &lockfile_path,
                    result.asset_count,
                    &mut result,
                );
            }
        } else {
            // Dry run - just collect what would happen
            for file in resolved_plan.to_write() {
                result.written.push(file.path.clone());
            }
            for binary_file in &binary_outputs {
                result.written.push(binary_file.path().clone());
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

    fn register_project(
        &self,
        project_root: &Path,
        lockfile_path: &Path,
        asset_count: usize,
        result: &mut DeployResult,
    ) {
        let Some(registry) = &self.registry_use_case else {
            return;
        };

        let project_root = project_root
            .canonicalize()
            .unwrap_or_else(|_| project_root.to_path_buf());
        let lockfile_path = lockfile_path
            .canonicalize()
            .unwrap_or_else(|_| lockfile_path.to_path_buf());

        if let Err(e) = registry.register_project(&project_root, &lockfile_path, asset_count) {
            result.add_warning(format!("Failed to update registry: {}", e));
        }
    }

    fn load_assets_from_layers(&self, options: &DeployOptions) -> Result<LayeredAssets, String> {
        let project_root = options.project_root.clone();

        let project_layer_path = if options.source.is_relative() {
            project_root.join(&options.source)
        } else {
            options.source.clone()
        };

        let mut layer_resolver = LayerResolver::new(project_root)
            .with_project_layer_path(project_layer_path)
            .with_disable_project_layer(!options.use_project_layer)
            .with_additional_layers(if options.use_additional_layers {
                options.additional_layers.clone()
            } else {
                Vec::new()
            })
            .with_remote_mode(options.remote_mode);
        if !options.remote_mode && options.use_user_layer {
            let user_layer_path = options
                .user_layer_path
                .clone()
                .unwrap_or_else(default_user_layer_path);
            layer_resolver = layer_resolver.with_user_layer_path(user_layer_path);
        }

        let resolution = layer_resolver.resolve().map_err(|e| match e {
            LayerResolveError::NoLayersFound => crate::CalvinError::NoLayersFound.to_string(),
            _ => e.to_string(),
        })?;

        let mut layers = resolution.layers;

        // Use shared layer_ops for consistent .calvinignore support
        load_resolved_layers(&self.asset_repo, &mut layers).map_err(|e| e.to_string())?;

        let merge_result = merge_layers(&layers);
        let assets: Vec<Asset> = merge_result
            .assets
            .values()
            .map(|m| m.asset.clone())
            .collect();

        let mut warnings = resolution.warnings;
        for override_info in &merge_result.overrides {
            warnings.push(format!(
                "Asset '{}' from {} overridden by {}",
                override_info.asset_id, override_info.from_layer, override_info.by_layer
            ));
        }

        // Collect warnings from individual assets (e.g., skipped binary files)
        for asset in &assets {
            warnings.extend(asset.warnings().iter().cloned());
        }

        Ok(LayeredAssets {
            assets,
            merged_assets_by_id: merge_result.assets,
            warnings,
        })
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
    ///
    /// Returns (text_outputs, binary_outputs, provenance_map)
    #[allow(clippy::type_complexity)]
    fn compile_assets(
        &self,
        assets: &[Asset],
        targets: &[Target],
        merged_assets_by_id: &std::collections::HashMap<String, MergedAsset>,
    ) -> Result<
        (
            Vec<OutputFile>,
            Vec<BinaryOutputFile>,
            std::collections::HashMap<PathBuf, crate::domain::entities::OutputProvenance>,
        ),
        String,
    > {
        use crate::domain::entities::AssetKind;
        use crate::domain::entities::OutputProvenance;
        use crate::domain::services::CompilerService;
        use std::path::PathBuf;

        fn merge_key_for_asset(asset: &Asset) -> String {
            match asset.kind() {
                AssetKind::Skill => format!("skill:{}", asset.id()),
                _ => asset.id().to_string(),
            }
        }

        let mut outputs = Vec::new();
        let mut binary_outputs: Vec<BinaryOutputFile> = Vec::new();
        let mut provenance_by_output_path: std::collections::HashMap<PathBuf, OutputProvenance> =
            std::collections::HashMap::new();

        // Determine which adapters to use
        // Empty targets list means "no targets" (not "all targets")
        let active_adapters: Vec<&Box<dyn TargetAdapter>> = if targets.is_empty() {
            // Empty targets = no deployment
            Vec::new()
        } else if targets.iter().any(|t| t.is_all()) {
            // Target::All = all adapters
            self.adapters.iter().collect()
        } else {
            // Specific targets = filter to matching adapters
            self.adapters
                .iter()
                .filter(|a| targets.contains(&a.target()))
                .collect()
        };

        // Check if Cursor needs to generate its own commands
        // (when Cursor is selected but Claude Code is not)
        let cursor_needs_commands = CompilerService::cursor_needs_commands(targets);
        let has_claude_code =
            targets.iter().any(|t| t.is_all()) || targets.contains(&Target::ClaudeCode);

        // Compile each asset with each adapter
        for asset in assets {
            // Get the effective targets for this asset (respects asset-level targets field)
            let asset_targets = asset.effective_targets();

            for adapter in &active_adapters {
                // Skip if this adapter's target is not enabled for this asset
                if !asset_targets.contains(&adapter.target()) {
                    continue;
                }

                // OpenCode reads Claude Code skills from `.claude/skills`, so avoid duplicate skill
                // outputs when Claude Code is also enabled for this deploy + this skill.
                if adapter.target() == Target::OpenCode
                    && asset.kind() == AssetKind::Skill
                    && has_claude_code
                    && asset_targets.contains(&Target::ClaudeCode)
                {
                    continue;
                }

                match adapter.compile(asset) {
                    Ok(adapter_outputs) => {
                        let provenance =
                            merged_assets_by_id
                                .get(&merge_key_for_asset(asset))
                                .map(|m| {
                                    let base = OutputProvenance::new(
                                        m.source_layer.clone(),
                                        m.source_layer_path.clone(),
                                        asset.id().to_string(),
                                        m.source_file.clone(),
                                    );
                                    match &m.overrides {
                                        Some(overrides) => base.with_overrides(overrides.clone()),
                                        None => base,
                                    }
                                });

                        for output in adapter_outputs {
                            if let Some(provenance) = provenance.clone() {
                                provenance_by_output_path.insert(output.path().clone(), provenance);
                            }
                            outputs.push(output);
                        }
                    }
                    Err(e) => {
                        return Err(format!(
                            "Adapter {} failed on {}: {}",
                            adapter.target().display_name(),
                            asset.id(),
                            e
                        ));
                    }
                }

                // Compile binary outputs for skills
                match adapter.compile_binary(asset) {
                    Ok(adapter_binary_outputs) => {
                        binary_outputs.extend(adapter_binary_outputs);
                    }
                    Err(e) => {
                        return Err(format!(
                            "Adapter {} binary compile failed on {}: {}",
                            adapter.target().display_name(),
                            asset.id(),
                            e
                        ));
                    }
                }

                // Special handling for Cursor: add commands if Claude Code is not selected
                if adapter.target() == Target::Cursor
                    && cursor_needs_commands
                    && matches!(asset.kind(), AssetKind::Action | AssetKind::Agent)
                {
                    let commands_base = match asset.scope() {
                        Scope::User => PathBuf::from("~/.cursor/commands"),
                        Scope::Project => PathBuf::from(".cursor/commands"),
                    };
                    let command_path = commands_base.join(format!("{}.md", asset.id()));
                    let footer = adapter.footer(&asset.source_path_normalized());
                    let content = CompilerService::generate_command_content(asset, &footer);
                    outputs.push(OutputFile::new(command_path, content, Target::Cursor));
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

        Ok((outputs, binary_outputs, provenance_by_output_path))
    }

    /// Plan the sync operation
    /// Resolve conflicts in the plan using the provided resolver
    fn resolve_conflicts(
        &self,
        mut plan: SyncPlan,
        resolver: &Arc<dyn ConflictResolver>,
        project_root: &Path,
        remote_mode: bool,
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
            let resolved_path = self.resolve_fs_path(project_root, &file.path, remote_mode);
            let existing_content = self
                .file_system
                .read(&resolved_path)
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
            let resolved_path =
                self.resolve_fs_path(&options.project_root, path, options.remote_mode);
            let exists = self.file_system.exists(&resolved_path);
            let current_hash = if exists {
                self.file_system.hash(&resolved_path).ok()
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
        project_root: &Path,
        remote_mode: bool,
    ) -> OrphanDetectionResult {
        self.detect_orphans_with_binaries(lockfile, outputs, &[], scope, project_root, remote_mode)
    }

    /// Detect orphan files including binary outputs
    fn detect_orphans_with_binaries(
        &self,
        lockfile: &Lockfile,
        outputs: &[OutputFile],
        binary_outputs: &[BinaryOutputFile],
        scope: Scope,
        project_root: &Path,
        remote_mode: bool,
    ) -> OrphanDetectionResult {
        let mut result =
            OrphanDetector::detect_with_binaries(lockfile, outputs, binary_outputs, scope);

        // Check status of each orphan (exists, has_signature)
        for orphan in &mut result.orphans {
            let original = PathBuf::from(&orphan.path);
            let resolved = self.resolve_fs_path(project_root, &original, remote_mode);
            orphan.exists = self.file_system.exists(&resolved);

            if orphan.exists {
                // Check if this was a binary file in the lockfile
                let lockfile_key = Lockfile::make_key(scope, &orphan.path);
                let is_lockfile_binary = lockfile
                    .get(&lockfile_key)
                    .is_some_and(|entry| entry.is_binary());

                if is_lockfile_binary {
                    // Binary files tracked in lockfile are safe to delete
                    orphan.has_signature = true;
                } else if let Ok(content) = self.file_system.read(&resolved) {
                    orphan.has_signature = has_calvin_signature(&content);
                    if !orphan.has_signature {
                        orphan.has_signature = self.is_part_of_calvin_skill(&resolved);
                    }
                }
            }
        }

        result
    }

    fn validate_skill_outputs(&self, outputs: &[OutputFile], result: &mut DeployResult) {
        use crate::domain::ports::DiagnosticSeverity;

        for output in outputs {
            if output
                .path()
                .file_name()
                .is_none_or(|n| n != std::ffi::OsStr::new("SKILL.md"))
            {
                continue;
            }

            let adapter = self
                .adapters
                .iter()
                .find(|a| a.target() == output.target())
                .map(|a| a.as_ref());
            let Some(adapter) = adapter else {
                continue;
            };

            let skill_id = output
                .path()
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            for diag in adapter.validate(output) {
                match diag.severity {
                    DiagnosticSeverity::Warning => result.add_warning(format!(
                        "Skill '{}' ({}): {}",
                        skill_id,
                        output.target().display_name(),
                        diag.message
                    )),
                    DiagnosticSeverity::Error => result.errors.push(format!(
                        "Skill '{}' ({}): {}",
                        skill_id,
                        output.target().display_name(),
                        diag.message
                    )),
                    DiagnosticSeverity::Info => {}
                }
            }
        }
    }

    /// Treat skill supplemental files as safe-to-delete if their SKILL.md is Calvin-signed.
    ///
    /// This allows `calvin deploy --cleanup` and `calvin clean` to fully remove skill directories
    /// without requiring Calvin signature markers inside arbitrary supplemental files (scripts, etc.).
    fn is_part_of_calvin_skill(&self, path: &Path) -> bool {
        let Some(skill_root) = skill_root_from_path(path) else {
            return false;
        };

        let skill_md = skill_root.join("SKILL.md");
        if !self.file_system.exists(&skill_md) {
            return false;
        }
        let Ok(content) = self.file_system.read(&skill_md) else {
            return false;
        };
        has_calvin_signature(&content)
    }

    /// Execute the sync plan
    /// Execute the sync plan with event reporting
    fn execute_plan_with_events(
        &self,
        plan: &SyncPlan,
        result: &mut DeployResult,
        event_sink: &Arc<dyn DeployEventSink>,
        project_root: &Path,
        remote_mode: bool,
    ) {
        for (index, file) in plan.files.iter().enumerate() {
            match &file.action {
                FileAction::Write => {
                    match self.write_file(project_root, remote_mode, &file.path, &file.content) {
                        Ok(_) => {
                            result.written.push(file.path.clone());
                            event_sink.on_event(DeployEvent::FileWritten {
                                index,
                                path: file.path.clone(),
                            });
                        }
                        Err(e) => {
                            let error_msg =
                                format!("Failed to write {}: {}", file.path.display(), e);
                            result.errors.push(error_msg.clone());
                            event_sink.on_event(DeployEvent::FileError {
                                index,
                                path: file.path.clone(),
                                error: error_msg,
                            });
                        }
                    }
                }
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

    /// Write binary outputs to disk
    fn write_binary_outputs(
        &self,
        binary_outputs: &[BinaryOutputFile],
        result: &mut DeployResult,
        project_root: &Path,
        remote_mode: bool,
    ) {
        for binary_file in binary_outputs {
            let resolved_path = self.resolve_fs_path(project_root, binary_file.path(), remote_mode);

            // Ensure parent directory exists
            if let Some(parent) = resolved_path.parent() {
                if let Err(e) = self.file_system.create_dir_all(parent) {
                    result.errors.push(format!(
                        "Failed to create directory for {}: {}",
                        binary_file.path().display(),
                        e
                    ));
                    continue;
                }
            }

            // Write binary content
            match self
                .file_system
                .write_binary(&resolved_path, binary_file.content())
            {
                Ok(_) => {
                    result.written.push(binary_file.path().clone());
                }
                Err(e) => {
                    result.errors.push(format!(
                        "Failed to write binary file {}: {}",
                        binary_file.path().display(),
                        e
                    ));
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
        project_root: &Path,
        remote_mode: bool,
    ) {
        use std::collections::HashSet;

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

        let mut skill_dirs_to_prune: HashSet<PathBuf> = HashSet::new();

        for orphan in &orphans.orphans {
            let original = PathBuf::from(&orphan.path);
            let resolved = self.resolve_fs_path(project_root, &original, remote_mode);
            if orphan.exists && orphan.is_safe_to_delete() {
                if let Err(e) = self.file_system.remove(&resolved) {
                    result
                        .errors
                        .push(format!("Failed to delete {}: {}", original.display(), e));
                } else {
                    result.deleted.push(original.clone());
                    event_sink.on_event(DeployEvent::OrphanDeleted {
                        path: original.clone(),
                    });

                    // Best-effort: prune empty skill directories after deleting orphan files.
                    // Skills are directory-based outputs (`.claude/skills/<id>/...`, `.codex/skills/<id>/...`).
                    if let Some(skill_root) = skill_root_from_path(&resolved) {
                        let mut current = resolved.parent();
                        while let Some(dir) = current {
                            if !dir.starts_with(&skill_root) {
                                break;
                            }
                            skill_dirs_to_prune.insert(dir.to_path_buf());
                            if dir == skill_root {
                                break;
                            }
                            current = dir.parent();
                        }
                    }
                }
            }
        }

        if !skill_dirs_to_prune.is_empty() {
            let mut dirs: Vec<PathBuf> = skill_dirs_to_prune.into_iter().collect();
            dirs.sort_by_key(|d| std::cmp::Reverse(d.components().count()));
            for dir in dirs {
                // Ignore errors: directories may not be empty or may be re-created later in the deploy.
                let _ = self.file_system.remove(&dir);
            }
        }
    }

    /// Write a file
    fn write_file(
        &self,
        project_root: &Path,
        remote_mode: bool,
        path: &Path,
        content: &str,
    ) -> FsResult<()> {
        let resolved = self.resolve_fs_path(project_root, path, remote_mode);
        if let Some(parent) = resolved.parent() {
            self.file_system.create_dir_all(parent)?;
        }
        self.file_system.write(&resolved, content)
    }

    fn resolve_fs_path(&self, project_root: &Path, path: &Path, remote_mode: bool) -> PathBuf {
        if remote_mode {
            return path.to_path_buf();
        }

        let path_str = path.to_string_lossy();
        if path_str.starts_with('~') {
            return self.file_system.expand_home(path);
        }

        if path.is_absolute() {
            return path.to_path_buf();
        }

        if project_root.as_os_str().is_empty() || project_root == Path::new(".") {
            return path.to_path_buf();
        }

        project_root.join(path)
    }

    /// Update lockfile after sync
    ///
    /// Returns an optional warning message if the lockfile could not be saved
    ///
    /// # Lockfile Recovery
    /// When files are skipped because their content is identical, we still need to
    /// ensure they're tracked in the lockfile. This handles the case where the lockfile
    /// was lost or empty but the files still exist with correct content.
    fn update_lockfile(
        &self,
        path: &Path,
        plan: &SyncPlan,
        result: &DeployResult,
        scope: Scope,
        provenance_by_output_path: Option<
            &std::collections::HashMap<PathBuf, crate::domain::entities::OutputProvenance>,
        >,
    ) -> Option<String> {
        self.update_lockfile_with_binaries(
            path,
            plan,
            &[],
            result,
            scope,
            provenance_by_output_path,
        )
    }

    /// Update lockfile including binary outputs
    fn update_lockfile_with_binaries(
        &self,
        path: &Path,
        plan: &SyncPlan,
        binary_outputs: &[BinaryOutputFile],
        result: &DeployResult,
        scope: Scope,
        provenance_by_output_path: Option<
            &std::collections::HashMap<PathBuf, crate::domain::entities::OutputProvenance>,
        >,
    ) -> Option<String> {
        use crate::domain::entities::LockfileEntry;
        use sha2::{Digest, Sha256};
        use std::collections::HashSet;

        let mut lockfile = match self.lockfile_repo.load(path) {
            Ok(lockfile) => lockfile,
            Err(e) => {
                return Some(format!("Failed to load lockfile for update: {}", e));
            }
        };

        // Build set of written and skipped paths
        let written_set: HashSet<_> = result.written.iter().collect();
        let skipped_set: HashSet<_> = result.skipped.iter().collect();

        // Update hashes for written and skipped files (and keep provenance in sync)
        for file in &plan.files {
            let key = Lockfile::make_key(scope, &file.path.display().to_string());

            if written_set.contains(&file.path) {
                let mut hasher = Sha256::new();
                hasher.update(file.content.as_bytes());
                let hash = format!("sha256:{:x}", hasher.finalize());
                match provenance_by_output_path
                    .and_then(|m| m.get(&file.path))
                    .cloned()
                {
                    Some(provenance) => lockfile.set_with_provenance(&key, &hash, provenance),
                    None => lockfile.set(&key, &hash),
                }
            } else if skipped_set.contains(&file.path) {
                // File was skipped (content identical, or conflict resolved to skip).
                // Still update lockfile so we track it going forward, and keep provenance in sync.
                let mut hasher = Sha256::new();
                hasher.update(file.content.as_bytes());
                let hash = format!("sha256:{:x}", hasher.finalize());
                match provenance_by_output_path
                    .and_then(|m| m.get(&file.path))
                    .cloned()
                {
                    Some(provenance) => lockfile.set_with_provenance(&key, &hash, provenance),
                    None => lockfile.set(&key, &hash),
                }
            }
        }

        // Update entries for binary files
        for binary_file in binary_outputs {
            if written_set.contains(binary_file.path()) {
                let key = Lockfile::make_key(scope, &binary_file.path().display().to_string());
                let entry = LockfileEntry::new(binary_file.content_hash()).with_binary(true);
                lockfile.set_entry(&key, entry);
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

#[derive(Debug)]
struct LayeredAssets {
    assets: Vec<Asset>,
    merged_assets_by_id: std::collections::HashMap<String, MergedAsset>,
    warnings: Vec<String>,
}

fn validate_skill_targets(assets: &[Asset]) -> Result<Vec<String>, String> {
    use crate::domain::entities::AssetKind;
    use crate::domain::value_objects::Target;

    let mut warnings = Vec::new();

    for asset in assets {
        if asset.kind() != AssetKind::Skill {
            continue;
        }

        let raw_targets = asset.targets();
        if raw_targets.is_empty() || raw_targets.iter().any(|t| t.is_all()) {
            continue;
        }

        let mut has_supported = false;
        let mut unsupported: Vec<Target> = Vec::new();

        for t in raw_targets {
            if t.is_all() {
                continue;
            }
            if t.supports_skills() {
                has_supported = true;
            } else {
                unsupported.push(*t);
            }
        }

        for t in unsupported {
            warnings.push(format!(
                "Skill '{}' targets {}, but skills are not supported on this platform; skipping.",
                asset.id(),
                t.display_name()
            ));
        }

        if !has_supported {
            return Err(format!(
                "Skill '{}' has no supported targets (supported: claude-code, codex, cursor, opencode).",
                asset.id()
            ));
        }
    }

    Ok(warnings)
}

fn warn_skills_skipped_for_unsupported_deploy_targets(
    assets: &[Asset],
    deploy_targets: &[crate::domain::value_objects::Target],
) -> Vec<String> {
    use crate::domain::entities::AssetKind;
    use crate::domain::value_objects::Target;

    if deploy_targets.is_empty() {
        return Vec::new();
    }

    let active_targets: Vec<Target> = if deploy_targets.iter().any(|t| t.is_all()) {
        Target::ALL_CONCRETE.to_vec()
    } else {
        deploy_targets.to_vec()
    };

    let mut unsupported: Vec<Target> = active_targets
        .into_iter()
        .filter(|t| !t.supports_skills())
        .collect();
    unsupported.sort_by_key(|t| t.display_name());
    unsupported.dedup();

    if unsupported.is_empty() {
        return Vec::new();
    }

    let has_skills_for_unsupported = assets.iter().any(|asset| {
        if asset.kind() != AssetKind::Skill {
            return false;
        }
        let effective = asset.effective_targets();
        unsupported.iter().any(|t| effective.contains(t))
    });

    if !has_skills_for_unsupported {
        return Vec::new();
    }

    let targets = unsupported
        .iter()
        .map(|t| t.display_name())
        .collect::<Vec<_>>()
        .join(", ");
    vec![format!(
        "Skills skipped for: {} (skills are not supported on these platforms).",
        targets
    )]
}

fn validate_opencode_assets(
    assets: &[Asset],
    deploy_targets: &[crate::domain::value_objects::Target],
) -> Vec<String> {
    use crate::domain::entities::AssetKind;
    use crate::domain::value_objects::Target;

    let deploy_includes_opencode = deploy_targets
        .iter()
        .any(|t| t.is_all() || *t == Target::OpenCode);
    if !deploy_includes_opencode {
        return Vec::new();
    }

    fn is_likely_opencode_model(value: &str) -> bool {
        let model = value.trim();
        if model.is_empty() {
            return false;
        }
        if model.contains('/') {
            return true;
        }
        // Allow common short names + common prefixes (best-effort).
        matches!(model, "sonnet" | "opus" | "haiku" | "inherit")
            || model.starts_with("anthropic/")
            || model.starts_with("openai/")
            || model.starts_with("google/")
            || model.starts_with("gpt")
            || model.starts_with("claude")
            || model.starts_with("gemini")
    }

    fn opencode_tool_key(tool: &str) -> Option<&'static str> {
        let normalized = tool.trim().to_lowercase();
        match normalized.as_str() {
            "read" => Some("read"),
            "write" => Some("write"),
            "edit" => Some("edit"),
            "bash" => Some("bash"),
            "grep" => Some("grep"),
            "glob" => Some("glob"),
            "webfetch" | "web-fetch" | "web_fetch" => Some("webfetch"),
            "task" => Some("task"),
            "skill" => Some("skill"),
            _ => None,
        }
    }

    let mut warnings = Vec::new();

    for asset in assets {
        if !asset.effective_targets().contains(&Target::OpenCode) {
            continue;
        }

        // mode (agent-only)
        if let Some(mode) = asset.opencode_mode() {
            if asset.kind() != AssetKind::Agent {
                warnings.push(format!(
                    "Asset '{}' sets 'mode', but this field is only used for OpenCode agents; it will be ignored.",
                    asset.id()
                ));
            } else if !matches!(mode, "primary" | "subagent") {
                warnings.push(format!(
                    "Agent '{}' has invalid OpenCode mode '{}'; defaulting to 'subagent'.",
                    asset.id(),
                    mode
                ));
            }
        }

        // temperature (agent-only)
        if let Some(temp) = asset.temperature() {
            if asset.kind() != AssetKind::Agent {
                warnings.push(format!(
                    "Asset '{}' sets 'temperature', but this field is only used for OpenCode agents; it will be ignored.",
                    asset.id()
                ));
            } else if !temp.is_finite() || !(0.0..=1.0).contains(&temp) {
                warnings.push(format!(
                    "Agent '{}' has invalid temperature {} (expected 0.0-1.0); omitting from OpenCode output.",
                    asset.id(),
                    temp
                ));
            }
        }

        // opencode-model (agents + commands)
        if let Some(model) = asset.opencode_model() {
            if !matches!(asset.kind(), AssetKind::Agent | AssetKind::Action) {
                warnings.push(format!(
                    "Asset '{}' sets 'opencode-model', but this field is only used for OpenCode agents/commands; it will be ignored.",
                    asset.id()
                ));
            } else if model.trim().is_empty() {
                warnings.push(format!(
                    "Asset '{}' has empty 'opencode-model'; omitting from OpenCode output.",
                    asset.id()
                ));
            } else if !is_likely_opencode_model(model) {
                warnings.push(format!(
                    "Model '{}' for OpenCode output may not be a standard provider/model ID; using as-is.",
                    model
                ));
            }
        } else if let Some(model) = asset.agent_model() {
            // Only warn about model values when compiling agents/commands for OpenCode.
            if matches!(asset.kind(), AssetKind::Agent | AssetKind::Action)
                && !is_likely_opencode_model(model)
            {
                warnings.push(format!(
                    "Model '{}' for OpenCode output may not be a standard provider/model ID; using as-is.",
                    model
                ));
            }
        }

        // command fields (action-only)
        if !matches!(asset.kind(), AssetKind::Action) {
            if asset.command_agent().is_some() {
                warnings.push(format!(
                    "Asset '{}' sets 'agent', but this field is only used for OpenCode commands; it will be ignored.",
                    asset.id()
                ));
            }
            if asset.command_subtask().is_some() {
                warnings.push(format!(
                    "Asset '{}' sets 'subtask', but this field is only used for OpenCode commands; it will be ignored.",
                    asset.id()
                ));
            }
        } else if let Some(agent) = asset.command_agent() {
            if agent.trim().is_empty() {
                warnings.push(format!(
                    "Action '{}' has empty 'agent' value; omitting from OpenCode output.",
                    asset.id()
                ));
            }
        }

        // tools (agent-only): warn on unknown tool names
        if asset.kind() == AssetKind::Agent && !asset.agent_tools().is_empty() {
            let mut unknown: Vec<String> = asset
                .agent_tools()
                .iter()
                .filter(|t| opencode_tool_key(t).is_none())
                .cloned()
                .collect();
            unknown.sort();
            unknown.dedup();
            for tool in unknown {
                warnings.push(format!(
                    "Unknown tool '{}' for OpenCode target (agent '{}'). Valid tools: read, write, edit, bash, grep, glob, webfetch, task, skill. Hint: This tool will be omitted from the OpenCode output.",
                    tool,
                    asset.id()
                ));
            }
        }

        // permission-mode mapping (agent-only): warn when unmappable
        if asset.kind() == AssetKind::Agent {
            if let Some(mode) = asset.agent_permission_mode() {
                let valid = [
                    "default",
                    "acceptEdits",
                    "dontAsk",
                    "bypassPermissions",
                    "plan",
                    "ignore",
                ];
                if !valid.contains(&mode) {
                    warnings.push(format!(
                        "permission-mode '{}' is not recognized; permission settings will be omitted from OpenCode output (agent '{}').",
                        mode,
                        asset.id()
                    ));
                } else if mode == "ignore" {
                    warnings.push(format!(
                        "permission-mode 'ignore' has no OpenCode equivalent; permission settings will be omitted (agent '{}').",
                        asset.id()
                    ));
                }
            }
        }
    }

    warnings
}
