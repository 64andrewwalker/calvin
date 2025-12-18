//! Core deploy runner using two-stage sync

use std::path::{Path, PathBuf};

use anyhow::Result;
use calvin::adapters::OutputFile;
use calvin::config::Config;
use calvin::models::PromptAsset;
use calvin::parser::parse_directory;
use calvin::sync::{
    compile_assets, execute_sync, plan_sync, resolve_conflicts_interactive,
    Lockfile, ResolveResult, SyncDestination, SyncResult, SyncStrategy,
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
        // Step 1: Scan and parse assets
        let assets = self.scan_assets()?;
        
        // Step 2: Compile to output files
        let outputs = self.compile_outputs(&assets)?;
        
        // Step 3: Two-stage sync
        let result = self.sync_outputs(&outputs)?;
        
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
    fn sync_outputs(&self, outputs: &[OutputFile]) -> Result<SyncResult> {
        let dest = self.target.to_sync_destination();
        
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
        let result = execute_sync(&final_plan, &dest, strategy)?;
        
        // Update lockfile with new hashes
        self.update_lockfile(&lockfile_path, &result);
        
        Ok(result)
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
        match &self.target {
            DeployTarget::Remote(remote) => {
                let (host, _) = if let Some((h, p)) = remote.split_once(':') {
                    (h, p)
                } else {
                    (remote.as_str(), ".")
                };
                let fs = RemoteFileSystem::new(host);
                Lockfile::load_or_new(path, &fs)
            }
            _ => {
                let fs = LocalFileSystem;
                Lockfile::load_or_new(path, &fs)
            }
        }
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
                Ok(plan_sync(outputs, dest, lockfile, &fs)?)
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
    fn update_lockfile(&self, path: &Path, result: &SyncResult) {
        // For now, skip lockfile update
        // TODO: Implement proper lockfile update with new hashes
        let _ = (path, result);
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
