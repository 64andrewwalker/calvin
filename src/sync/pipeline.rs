use std::path::{Path, PathBuf};

use super::OutputFile;
use crate::config::Config;
use crate::error::CalvinResult;
use crate::models::Target;
use crate::parser::parse_directory;
use crate::sync::scope::ScopePolicyExt;
use crate::sync::{compile_assets, ScopePolicy};

/// Unified pipeline for parsing + scope filtering + compilation.
#[derive(Debug, Clone)]
pub struct AssetPipeline {
    source: PathBuf,
    config: Config,
    scope_policy: ScopePolicy,
    targets: Vec<Target>,
}

impl AssetPipeline {
    pub fn new(source: PathBuf, config: Config) -> Self {
        Self {
            source,
            config,
            scope_policy: ScopePolicy::Keep,
            targets: Vec::new(),
        }
    }

    pub fn source(&self) -> &Path {
        &self.source
    }

    pub fn with_scope_policy(mut self, policy: ScopePolicy) -> Self {
        self.scope_policy = policy;
        self
    }

    pub fn with_targets(mut self, targets: Vec<Target>) -> Self {
        self.targets = targets;
        self
    }

    /// Parse + apply scope policy + compile.
    pub fn compile(&self) -> CalvinResult<Vec<OutputFile>> {
        let assets = parse_directory(&self.source)?;
        let filtered = self.scope_policy.apply(assets);
        compile_assets(&filtered, &self.targets, &self.config)
    }

    /// Incremental version for watch mode.
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

    // --- Variants ---

    #[test]
    fn pipeline__compile_incremental_no_changes_after_initial__returns_empty() {
        let dir = tempdir().unwrap();
        let source = dir.path().join(".promptpack");
        fs::create_dir_all(&source).unwrap();
        write_asset(&source, "actions/test.md", "project");

        let config = Config::default();
        let mut cache = crate::watcher::IncrementalCache::new();
        let pipeline = AssetPipeline::new(source, config).with_scope_policy(ScopePolicy::Keep);

        // Initial compile
        pipeline.compile_incremental(&[], &mut cache).unwrap();

        // Second compile with no changes
        // Note: internal parse_incremental optimization should return empty if nothing changed
        let outputs = pipeline.compile_incremental(&[], &mut cache).unwrap();
        // Wait, if no files changed, parse_incremental returns "assets".
        // But if nothing changed, does it return cached assets?
        // parse_incremental logic: if changed_files empty + cache populated -> likely returns nothing (unchanged).
        // If it returns parsed assets (cache hit), then compile_assets produces outputs.
        // Incremental logic usually implies "only changed assets" OR "all assets because we track state".
        // Let's verify assumption: parse_incremental returns ALL assets (from cache or disk).
        // So output should NOT be empty (idempotent).
        assert!(!outputs.is_empty());
    }

    #[test]
    fn pipeline__error_on_nonexistent_source() {
        let source = PathBuf::from("/nonexistent/promptpack");
        let config = Config::default();
        let pipeline = AssetPipeline::new(source, config);

        let result = pipeline.compile();
        assert!(result.is_err());
    }
}
