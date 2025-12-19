//! Bridge module for transitioning from DeployRunner to DeployUseCase
//!
//! This module provides conversion functions between the old and new architectures,
//! allowing for a gradual migration.
//!
//! These functions are currently unused but will be used when we switch to the new engine.
#![allow(dead_code)]

use calvin::application::{DeployOptions as UseCaseOptions, DeployResult as UseCaseResult};
use calvin::domain::value_objects::{Scope, Target as DomainTarget};
use calvin::presentation::factory::{create_adapters_for_targets, ConcreteDeployUseCase};
use calvin::sync::SyncResult;

use super::options::DeployOptions as RunnerOptions;
use super::targets::DeployTarget;

/// Convert runner options to use case options
pub fn convert_options(
    source: &std::path::Path,
    target: &DeployTarget,
    runner_options: &RunnerOptions,
) -> UseCaseOptions {
    // Determine scope based on target
    let scope = match target {
        DeployTarget::Home => Scope::User,
        DeployTarget::Project(_) | DeployTarget::Remote(_) => Scope::Project,
    };

    // Convert targets
    let targets: Vec<DomainTarget> = runner_options
        .targets
        .iter()
        .map(|t| match t {
            calvin::Target::ClaudeCode => DomainTarget::ClaudeCode,
            calvin::Target::Cursor => DomainTarget::Cursor,
            calvin::Target::VSCode => DomainTarget::VSCode,
            calvin::Target::Antigravity => DomainTarget::Antigravity,
            calvin::Target::Codex => DomainTarget::Codex,
            calvin::Target::All => DomainTarget::All,
        })
        .collect();

    UseCaseOptions {
        source: source.to_path_buf(),
        scope,
        targets,
        force: runner_options.force,
        interactive: runner_options.interactive,
        dry_run: runner_options.dry_run,
        clean_orphans: false, // Handled separately in cmd_deploy
    }
}

/// Convert use case result to runner result (for compatibility)
pub fn convert_result(use_case_result: &UseCaseResult) -> SyncResult {
    SyncResult {
        written: use_case_result
            .written
            .iter()
            .map(|p| p.display().to_string())
            .collect(),
        skipped: use_case_result
            .skipped
            .iter()
            .map(|p| p.display().to_string())
            .collect(),
        errors: use_case_result.errors.clone(),
    }
}

/// Check if we should use the new engine
///
/// Currently controlled by environment variable for testing.
pub fn should_use_new_engine() -> bool {
    std::env::var("CALVIN_NEW_ENGINE").is_ok_and(|v| v == "1" || v.to_lowercase() == "true")
}

/// Create a deploy use case for the given targets
pub fn create_use_case_for_targets(targets: &[calvin::Target]) -> ConcreteDeployUseCase {
    let domain_targets: Vec<DomainTarget> = targets
        .iter()
        .map(|t| match t {
            calvin::Target::ClaudeCode => DomainTarget::ClaudeCode,
            calvin::Target::Cursor => DomainTarget::Cursor,
            calvin::Target::VSCode => DomainTarget::VSCode,
            calvin::Target::Antigravity => DomainTarget::Antigravity,
            calvin::Target::Codex => DomainTarget::Codex,
            calvin::Target::All => DomainTarget::All,
        })
        .collect();

    let adapters = create_adapters_for_targets(&domain_targets);
    calvin::presentation::factory::create_deploy_use_case_with_adapters(adapters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn convert_options_project_target() {
        let runner_options = RunnerOptions::new();
        let options = convert_options(
            std::path::Path::new("/project/.promptpack"),
            &DeployTarget::Project(PathBuf::from("/project")),
            &runner_options,
        );

        assert_eq!(options.scope, Scope::Project);
        assert_eq!(options.source, PathBuf::from("/project/.promptpack"));
    }

    #[test]
    fn convert_options_home_target() {
        let runner_options = RunnerOptions::new();
        let options = convert_options(
            std::path::Path::new("/project/.promptpack"),
            &DeployTarget::Home,
            &runner_options,
        );

        assert_eq!(options.scope, Scope::User);
    }

    #[test]
    fn convert_options_preserves_force() {
        let mut runner_options = RunnerOptions::new();
        runner_options.force = true;

        let options = convert_options(
            std::path::Path::new("/project/.promptpack"),
            &DeployTarget::Project(PathBuf::from("/project")),
            &runner_options,
        );

        assert!(options.force);
    }

    #[test]
    fn convert_options_preserves_dry_run() {
        let mut runner_options = RunnerOptions::new();
        runner_options.dry_run = true;

        let options = convert_options(
            std::path::Path::new("/project/.promptpack"),
            &DeployTarget::Project(PathBuf::from("/project")),
            &runner_options,
        );

        assert!(options.dry_run);
    }

    #[test]
    fn should_use_new_engine_default_false() {
        // Clear any existing env var
        std::env::remove_var("CALVIN_NEW_ENGINE");
        assert!(!should_use_new_engine());
    }
}
