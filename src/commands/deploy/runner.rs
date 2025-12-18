//! Core deploy runner using two-stage sync

use std::path::{Path, PathBuf};

use anyhow::Result;
use calvin::adapters::OutputFile;
use calvin::config::Config;
use calvin::models::PromptAsset;
use calvin::parser::parse_directory;
use calvin::sync::{
    compile_assets, execute_sync_with_callback, plan_sync, plan_sync_remote, resolve_conflicts_interactive,
    Lockfile, ResolveResult, SyncDestination, SyncEvent, SyncResult, SyncStrategy,
};
use calvin::fs::{LocalFileSystem, RemoteFileSystem};

use super::targets::{DeployTarget, ScopePolicy};
use super::options::DeployOptions;
use crate::ui::context::UiContext;

/// Deploy runner using two-stage sync
pub struct DeployRunner {
    /// Source directory (.promptpack)
    source: PathBuf,
    /// Deployment target
    target: DeployTarget,
    /// Scope policy for assets
    scope_policy: ScopePolicy,
    /// Deploy options
    options: DeployOptions,
    /// Configuration
    config: Config,
    /// UI context
    ui: UiContext,
}

impl DeployRunner {
    /// Create a new deploy runner
    pub fn new(
        source: PathBuf,
        target: DeployTarget,
        scope_policy: ScopePolicy,
        options: DeployOptions,
        ui: UiContext,
    ) -> Self {
        let config = Config::load_or_default(Some(source.as_path()));
        Self {
            source,
            target,
            scope_policy,
            options,
            config,
            ui,
        }
    }

    /// Run the deploy operation
    pub fn run(&self) -> Result<SyncResult> {
        self.run_with_callback::<fn(SyncEvent)>(None)
    }

    /// Run the deploy operation with a callback for sync events
    pub fn run_with_callback<F>(&self, callback: Option<F>) -> Result<SyncResult>
    where
        F: FnMut(SyncEvent),
    {
        // Step 1: Scan and parse assets
        let assets = self.scan_assets()?;
        
        // Step 2: Compile to output files
        let outputs = self.compile_outputs(&assets)?;
        
        // Step 3: Two-stage sync
        let result = self.sync_outputs(&outputs, callback)?;
        
        Ok(result)
    }

    /// Scan source directory for assets
    fn scan_assets(&self) -> Result<Vec<PromptAsset>> {
        let assets = parse_directory(&self.source)?;
        
        // Apply scope policy
        let filtered = match self.scope_policy {
            ScopePolicy::Keep => assets,
            ScopePolicy::UserOnly => assets.into_iter()
                .filter(|a| a.frontmatter.scope == calvin::models::Scope::User)
                .collect(),
            ScopePolicy::ForceUser => assets.into_iter()
                .map(|mut a| {
                    a.frontmatter.scope = calvin::models::Scope::User;
                    a
                })
                .collect(),
        };
        
        Ok(filtered)
    }

    /// Compile assets to output files
    fn compile_outputs(&self, assets: &[PromptAsset]) -> Result<Vec<OutputFile>> {
        let targets = if self.options.targets.is_empty() {
            vec![
                calvin::Target::ClaudeCode,
                calvin::Target::Cursor,
                calvin::Target::VSCode,
                calvin::Target::Antigravity,
                calvin::Target::Codex,
            ]
        } else {
            self.options.targets.clone()
        };
        
        Ok(compile_assets(assets, &targets, &self.config)?)
    }

    /// Two-stage sync: plan -> resolve -> execute
    fn sync_outputs<F>(&self, outputs: &[OutputFile], callback: Option<F>) -> Result<SyncResult>
    where
        F: FnMut(SyncEvent),
    {
        let dest = self.target.to_sync_destination();
        
        // Fast path: Remote + force mode -> skip planning, use rsync directly
        // Planning for remote is slow because it requires SSH per-file checks
        if matches!(&self.target, DeployTarget::Remote(_)) && self.options.force {
            let result = self.sync_remote_fast(outputs, callback)?;
            // Update lockfile for fast path too
            let lockfile_path = self.get_lockfile_path();
            self.update_lockfile(&lockfile_path, outputs, &result);
            return Ok(result);
        }
        
        // Load or create lockfile
        let lockfile_path = self.get_lockfile_path();
        let lockfile = self.load_lockfile(&lockfile_path);
        
        // Stage 1: Plan (detect conflicts)
        let plan = self.plan_sync(outputs, &dest, &lockfile)?;
        
        // Stage 2: Resolve conflicts
        let final_plan = if self.options.force || plan.conflicts.is_empty() {
            // Force mode or no conflicts
            plan.overwrite_all()
        } else if self.options.interactive {
            // Interactive conflict resolution
            let (resolved, status) = self.resolve_conflicts(plan, &dest);
            if status == ResolveResult::Aborted {
                anyhow::bail!("Sync aborted by user");
            }
            resolved
        } else {
            // Non-interactive, non-force - skip conflicts
            plan.skip_all()
        };

        // Dry run - just return what would be done
        if self.options.dry_run {
            return Ok(SyncResult {
                written: final_plan.to_write.iter()
                    .map(|o| o.path.display().to_string())
                    .collect(),
                skipped: final_plan.to_skip,
                errors: vec![],
            });
        }

        // Stage 3: Execute (batch transfer using optimal strategy)
        let strategy = self.select_strategy(&final_plan);
        let result = execute_sync_with_callback(&final_plan, &dest, strategy, callback)?;
        
        // Update lockfile with new hashes
        // Use final_plan.to_write because it includes resolved conflicts
        self.update_lockfile(&lockfile_path, &final_plan.to_write, &result);
        
        Ok(result)
    }
    
    /// Fast path for remote + force: skip planning, use rsync directly
    fn sync_remote_fast<F>(&self, outputs: &[OutputFile], callback: Option<F>) -> Result<SyncResult>
    where
        F: FnMut(SyncEvent),
    {
        let remote_str = match &self.target {
            DeployTarget::Remote(r) => r.clone(),
            _ => unreachable!("sync_remote_fast called with non-remote target"),
        };
        
        if self.options.dry_run {
            return Ok(SyncResult {
                written: outputs.iter()
                    .map(|o| o.path.display().to_string())
                    .collect(),
                skipped: vec![],
                errors: vec![],
            });
        }
        
        let use_rsync = calvin::sync::remote::has_rsync() && callback.is_none();
        
        if use_rsync {
            // Fast path: rsync batch transfer
            let options = calvin::sync::SyncOptions {
                force: true,
                dry_run: false,
                interactive: false,
                targets: vec![],
            };
            Ok(calvin::sync::remote::sync_remote_rsync(&remote_str, outputs, &options, self.options.json)?)
        } else {
            // File-by-file via SSH with callback
            let dest = self.target.to_sync_destination();
            let mut plan = calvin::sync::SyncPlan::new();
            plan.to_write = outputs.to_vec();
            Ok(execute_sync_with_callback(&plan, &dest, SyncStrategy::FileByFile, callback)?)
        }
    }

    /// Get lockfile path based on target
    fn get_lockfile_path(&self) -> PathBuf {
        match &self.target {
            DeployTarget::Project(root) => root.join(".promptpack/.calvin.lock"),
            DeployTarget::Home => {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".promptpack/.calvin.lock")
            }
            DeployTarget::Remote(_) => {
                // For remote, use local source lockfile
                self.source.join(".calvin.lock")
            }
        }
    }

    /// Load lockfile using appropriate filesystem
    fn load_lockfile(&self, path: &Path) -> Lockfile {
        // Lockfile is always stored locally (even for remote targets)
        // For remote targets, the lockfile path points to local source directory
        let fs = LocalFileSystem;
        Lockfile::load_or_new(path, &fs)
    }

    /// Plan sync using appropriate filesystem
    fn plan_sync(
        &self,
        outputs: &[OutputFile],
        dest: &SyncDestination,
        lockfile: &Lockfile,
    ) -> Result<calvin::sync::SyncPlan> {
        match &self.target {
            DeployTarget::Remote(remote) => {
                let (host, _) = if let Some((h, p)) = remote.split_once(':') {
                    (h, p)
                } else {
                    (remote.as_str(), ".")
                };
                let fs = RemoteFileSystem::new(host);
                // Use batch version for remote - single SSH call instead of per-file
                Ok(plan_sync_remote(outputs, dest, lockfile, &fs)?)
            }
            _ => {
                let fs = LocalFileSystem;
                Ok(plan_sync(outputs, dest, lockfile, &fs)?)
            }
        }
    }

    /// Resolve conflicts using appropriate filesystem
    fn resolve_conflicts(
        &self,
        plan: calvin::sync::SyncPlan,
        dest: &SyncDestination,
    ) -> (calvin::sync::SyncPlan, ResolveResult) {
        match &self.target {
            DeployTarget::Remote(remote) => {
                let (host, _) = if let Some((h, p)) = remote.split_once(':') {
                    (h, p)
                } else {
                    (remote.as_str(), ".")
                };
                let fs = RemoteFileSystem::new(host);
                resolve_conflicts_interactive(plan, dest, &fs)
            }
            _ => {
                let fs = LocalFileSystem;
                resolve_conflicts_interactive(plan, dest, &fs)
            }
        }
    }

    /// Select sync strategy based on options and file count
    fn select_strategy(&self, plan: &calvin::sync::SyncPlan) -> SyncStrategy {
        // Use rsync for batch transfer when:
        // 1. More than 10 files
        // 2. rsync is available
        // 3. Not in JSON mode (rsync output would interfere)
        if plan.to_write.len() > 10 
            && calvin::sync::remote::has_rsync() 
            && !self.options.json 
        {
            SyncStrategy::Rsync
        } else {
            SyncStrategy::FileByFile
        }
    }

    /// Update lockfile after successful sync
    fn update_lockfile(&self, path: &Path, outputs: &[OutputFile], result: &SyncResult) {
        use std::collections::HashSet;
        
        // Load existing lockfile
        let fs = LocalFileSystem;
        let mut lockfile = Lockfile::load_or_new(path, &fs);
        
        // Build set of written paths for fast lookup
        let written_set: HashSet<&str> = result.written.iter().map(|s| s.as_str()).collect();
        
        let mut updated_count = 0;
        
        // Update hashes for written files
        for output in outputs {
            let path_str = output.path.display().to_string();
            if written_set.contains(path_str.as_str()) {
                // This file was written, update its hash
                let hash = calvin::sync::lockfile::hash_content(&output.content);
                lockfile.set_hash(&path_str, &hash);
                updated_count += 1;
            }
        }
        
        // Save lockfile if any updates were made
        if updated_count > 0 {
            if let Err(e) = lockfile.save(path, &fs) {
                // Log error but don't fail the deploy
                eprintln!("Warning: Failed to update lockfile: {}", e);
            }
        }
    }

    // Getters for UI rendering
    pub fn source(&self) -> &Path { &self.source }
    pub fn target(&self) -> &DeployTarget { &self.target }
    pub fn options(&self) -> &DeployOptions { &self.options }
    pub fn ui(&self) -> &UiContext { &self.ui }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::terminal::TerminalCapabilities;

    fn test_ui() -> UiContext {
        let caps = TerminalCapabilities {
            is_tty: false,
            supports_color: false,
            supports_256_color: false,
            supports_true_color: false,
            supports_unicode: false,
            is_ci: true,
            width: 80,
            height: 24,
        };
        UiContext {
            json: false,
            verbose: 0,
            caps,
            color: false,
            unicode: false,
            animation: false,
        }
    }

    #[test]
    fn runner_with_project_target() {
        let options = DeployOptions::new();
        let ui = test_ui();
        let runner = DeployRunner::new(
            PathBuf::from(".promptpack"),
            DeployTarget::Project(PathBuf::from("/project")),
            ScopePolicy::Keep,
            options,
            ui,
        );
        
        assert!(runner.target.is_local());
    }

    #[test]
    fn runner_with_remote_target() {
        let options = DeployOptions::new();
        let ui = test_ui();
        let runner = DeployRunner::new(
            PathBuf::from(".promptpack"),
            DeployTarget::Remote("server:~".to_string()),
            ScopePolicy::Keep,
            options,
            ui,
        );
        
        assert!(!runner.target.is_local());
    }
}
