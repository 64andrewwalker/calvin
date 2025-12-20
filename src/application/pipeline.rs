//! Asset compilation pipeline
//!
//! Provides a unified pipeline for parsing, filtering, and compiling assets.
//! This is used by both deploy and watch commands.
//!
//! ## Flow
//!
//! 1. Parse assets from source directory
//! 2. Apply scope policy (Keep, ForceUser, ProjectOnly, UserOnly)
//! 3. Compile assets to OutputFile via adapters

use std::path::{Path, PathBuf};

use crate::application::compile_assets;
use crate::config::Config;
use crate::domain::entities::OutputFile;
use crate::domain::policies::ScopePolicy;
use crate::error::CalvinResult;
use crate::models::Target;
use crate::parser::parse_directory;

// Re-export for backward compatibility
pub use crate::domain::policies::ScopePolicy as ScopePolicyType;

// Import the trait for the apply() method
use crate::domain::policies::ScopePolicyExt;

/// Unified pipeline for parsing + scope filtering + compilation.
///
/// This is the application layer abstraction that coordinates:
/// - Parsing (infrastructure)
/// - Scope filtering (domain policy)
/// - Compilation (domain service + adapters)
#[derive(Debug, Clone)]
pub struct AssetPipeline {
    source: PathBuf,
    config: Config,
    scope_policy: ScopePolicy,
    targets: Vec<Target>,
}

impl AssetPipeline {
    /// Create a new pipeline with default settings.
    pub fn new(source: PathBuf, config: Config) -> Self {
        Self {
            source,
            config,
            scope_policy: ScopePolicy::Keep,
            targets: Vec::new(),
        }
    }

    /// Get the source directory.
    pub fn source(&self) -> &Path {
        &self.source
    }

    /// Set the scope policy.
    pub fn with_scope_policy(mut self, policy: ScopePolicy) -> Self {
        self.scope_policy = policy;
        self
    }

    /// Set target filters.
    pub fn with_targets(mut self, targets: Vec<Target>) -> Self {
        self.targets = targets;
        self
    }

    /// Parse + apply scope policy + compile.
    ///
    /// This is the main entry point for full compilation.
    pub fn compile(&self) -> CalvinResult<Vec<OutputFile>> {
        let assets = parse_directory(&self.source)?;
        let filtered = self.scope_policy.apply(assets);
        compile_assets(&filtered, &self.targets, &self.config)
    }

    /// Incremental version for watch mode.
    ///
    /// Uses a cache to only reparse changed files.
    pub fn compile_incremental(
        &self,
        changed_files: &[PathBuf],
        cache: &mut crate::watcher::IncrementalCache,
    ) -> CalvinResult<Vec<OutputFile>> {
        let assets = crate::watcher::parse_incremental(&self.source, changed_files, cache)?;
        let filtered = self.scope_policy.apply(assets);
        compile_assets(&filtered, &self.targets, &self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_asset(dir: &Path, rel: &str, scope: &str) {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(
            &path,
            format!(
                r#"---
description: Test asset
kind: action
scope: {scope}
---
# Title

Hello
"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn pipeline_compile_changes_output_paths_when_forcing_user_scope() {
        let dir = tempdir().unwrap();
        let source = dir.path().join(".promptpack");
        fs::create_dir_all(&source).unwrap();

        write_asset(&source, "actions/test.md", "project");

        let config = Config::default();

        let outputs_keep = AssetPipeline::new(source.clone(), config.clone())
            .with_scope_policy(ScopePolicy::Keep)
            .compile()
            .unwrap();
        assert!(outputs_keep
            .iter()
            .any(|o| o.path() == &PathBuf::from(".codex/prompts/test.md")));

        let outputs_force_user = AssetPipeline::new(source, config)
            .with_scope_policy(ScopePolicy::ForceUser)
            .compile()
            .unwrap();
        assert!(outputs_force_user
            .iter()
            .any(|o| o.path() == &PathBuf::from("~/.codex/prompts/test.md")));
    }

    #[test]
    fn pipeline_compile_incremental_initial_full_parse_works() {
        let dir = tempdir().unwrap();
        let source = dir.path().join(".promptpack");
        fs::create_dir_all(&source).unwrap();

        write_asset(&source, "actions/test.md", "project");

        let config = Config::default();
        let mut cache = crate::watcher::IncrementalCache::new();

        let outputs = AssetPipeline::new(source, config)
            .with_scope_policy(ScopePolicy::Keep)
            .compile_incremental(&[], &mut cache)
            .unwrap();

        assert!(!outputs.is_empty());
    }

    #[test]
    fn pipeline_error_on_nonexistent_source() {
        let source = PathBuf::from("/nonexistent/promptpack");
        let config = Config::default();
        let pipeline = AssetPipeline::new(source, config);

        let result = pipeline.compile();
        assert!(result.is_err());
    }
}
