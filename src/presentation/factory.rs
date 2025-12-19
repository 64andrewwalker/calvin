//! Use Case Factory
//!
//! Creates use cases with infrastructure dependencies wired up.
//! This is the dependency injection point for the application.

use crate::application::DeployUseCase;
use crate::domain::ports::TargetAdapter;
use crate::infrastructure::{
    all_adapters, ClaudeCodeAdapter, CursorAdapter, FsAssetRepository, LocalFs,
    TomlLockfileRepository,
};

/// Type alias for the concrete DeployUseCase with all dependencies
pub type ConcreteDeployUseCase = DeployUseCase<FsAssetRepository, TomlLockfileRepository, LocalFs>;

/// Create a deploy use case with all dependencies wired up
///
/// This is the main entry point for creating a deploy use case.
/// All infrastructure dependencies are automatically injected.
pub fn create_deploy_use_case() -> ConcreteDeployUseCase {
    let asset_repo = FsAssetRepository::new();
    let lockfile_repo = TomlLockfileRepository::new();
    let file_system = LocalFs::new();
    let adapters = all_adapters();

    DeployUseCase::new(asset_repo, lockfile_repo, file_system, adapters)
}

/// Create a deploy use case with specific adapters
///
/// Useful when you only want to deploy to specific targets.
pub fn create_deploy_use_case_with_adapters(
    adapters: Vec<Box<dyn TargetAdapter>>,
) -> ConcreteDeployUseCase {
    let asset_repo = FsAssetRepository::new();
    let lockfile_repo = TomlLockfileRepository::new();
    let file_system = LocalFs::new();

    DeployUseCase::new(asset_repo, lockfile_repo, file_system, adapters)
}

/// Create adapters for specific targets
pub fn create_adapters_for_targets(
    targets: &[crate::domain::value_objects::Target],
) -> Vec<Box<dyn TargetAdapter>> {
    use crate::domain::value_objects::Target;

    let mut adapters: Vec<Box<dyn TargetAdapter>> = Vec::new();

    for target in targets {
        match target {
            Target::ClaudeCode => adapters.push(Box::new(ClaudeCodeAdapter::new())),
            Target::Cursor => adapters.push(Box::new(CursorAdapter::new())),
            // TODO: Add other adapters as they are migrated
            Target::VSCode | Target::Antigravity | Target::Codex => {
                // Not yet migrated - use legacy adapters
            }
            Target::All => {
                // Return all available adapters
                return all_adapters();
            }
        }
    }

    adapters
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::Target;

    #[test]
    fn create_deploy_use_case_returns_valid_use_case() {
        let _use_case = create_deploy_use_case();
        // If this compiles, the factory is correctly wiring dependencies
    }

    #[test]
    fn create_adapters_for_claude_code() {
        let adapters = create_adapters_for_targets(&[Target::ClaudeCode]);
        assert_eq!(adapters.len(), 1);
        assert_eq!(adapters[0].target(), Target::ClaudeCode);
    }

    #[test]
    fn create_adapters_for_cursor() {
        let adapters = create_adapters_for_targets(&[Target::Cursor]);
        assert_eq!(adapters.len(), 1);
        assert_eq!(adapters[0].target(), Target::Cursor);
    }

    #[test]
    fn create_adapters_for_multiple() {
        let adapters = create_adapters_for_targets(&[Target::ClaudeCode, Target::Cursor]);
        assert_eq!(adapters.len(), 2);
    }

    #[test]
    fn create_adapters_for_all_returns_all() {
        let adapters = create_adapters_for_targets(&[Target::All]);
        assert_eq!(adapters.len(), 5); // All 5 adapters migrated
    }
}
