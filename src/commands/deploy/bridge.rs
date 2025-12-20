//! Bridge module for transitioning from DeployRunner to DeployUseCase
//!
//! This module provides conversion functions between the old and new architectures,
//! allowing for a gradual migration.
#![allow(dead_code)]

use calvin::application::{DeployOptions as UseCaseOptions, DeployResult as UseCaseResult};
use calvin::domain::value_objects::{Scope, Target as DomainTarget};
use calvin::presentation::factory::{
    create_adapters_for_targets, create_deploy_use_case_for_remote_with_adapters,
    ConcreteDeployUseCase,
};

use super::options::DeployOptions as RunnerOptions;
use super::targets::DeployTarget;

/// Convert runner options to use case options
pub fn convert_options(
    source: &std::path::Path,
    target: &DeployTarget,
    runner_options: &RunnerOptions,
    cleanup: bool,
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
        clean_orphans: cleanup, // Pass through cleanup flag
    }
}

/// Create a deploy use case for the given targets (local destinations)
pub fn create_use_case_for_targets(targets: &[calvin::Target]) -> ConcreteDeployUseCase {
    let adapters = create_adapters_for_legacy_targets(targets);
    calvin::presentation::factory::create_deploy_use_case_with_adapters(adapters)
}

/// Run remote deployment using new engine
pub fn run_remote_deployment(
    remote_spec: &str,
    source: &std::path::Path,
    options: &UseCaseOptions,
    targets: &[calvin::Target],
) -> UseCaseResult {
    let adapters = create_adapters_for_legacy_targets(targets);
    let use_case = create_deploy_use_case_for_remote_with_adapters(
        remote_spec,
        source.to_path_buf(),
        adapters,
    );
    use_case.execute(options)
}

/// Convert legacy targets to domain targets and create adapters
fn create_adapters_for_legacy_targets(
    targets: &[calvin::Target],
) -> Vec<Box<dyn calvin::domain::ports::TargetAdapter>> {
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

    create_adapters_for_targets(&domain_targets)
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
            false,
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
            false,
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
            false,
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
            false,
        );

        assert!(options.dry_run);
    }

    #[test]
    fn convert_options_preserves_cleanup() {
        let runner_options = RunnerOptions::new();

        let options = convert_options(
            std::path::Path::new("/project/.promptpack"),
            &DeployTarget::Project(PathBuf::from("/project")),
            &runner_options,
            true,
        );

        assert!(options.clean_orphans);
    }
}
