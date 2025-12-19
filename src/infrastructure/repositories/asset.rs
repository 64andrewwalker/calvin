//! File System Asset Repository
//!
//! Loads assets from the file system by parsing PromptPack files.

use crate::domain::entities::Asset;
use crate::domain::ports::AssetRepository;
use crate::domain::value_objects::{Scope, Target};
use anyhow::Result;
use std::path::Path;

/// Asset repository that loads from the file system
///
/// Parses `.md` files with YAML frontmatter from a PromptPack directory.
/// Uses the existing parser for now - will be refactored later.
pub struct FsAssetRepository;

impl FsAssetRepository {
    /// Create a new repository
    pub fn new() -> Self {
        Self
    }
}

impl Default for FsAssetRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl FsAssetRepository {
    /// Convert a legacy PromptAsset to domain Asset
    fn convert_prompt_asset(pa: crate::models::PromptAsset) -> Asset {
        let scope = match pa.frontmatter.scope {
            crate::models::Scope::Project => Scope::Project,
            crate::models::Scope::User => Scope::User,
        };

        let targets: Vec<Target> = pa
            .frontmatter
            .targets
            .iter()
            .map(|t| match t {
                crate::models::Target::ClaudeCode => Target::ClaudeCode,
                crate::models::Target::Cursor => Target::Cursor,
                crate::models::Target::VSCode => Target::VSCode,
                crate::models::Target::Antigravity => Target::Antigravity,
                crate::models::Target::Codex => Target::Codex,
                crate::models::Target::All => Target::All,
            })
            .collect();

        Asset::new(
            &pa.id,
            &pa.source_path,
            &pa.frontmatter.description,
            &pa.content,
        )
        .with_scope(scope)
        .with_targets(targets)
    }
}

impl AssetRepository for FsAssetRepository {
    fn load_all(&self, source: &Path) -> Result<Vec<Asset>> {
        // Use the existing parser for now
        let prompt_assets = crate::parser::parse_directory(source)?;

        // Convert from PromptAsset to domain Asset
        let assets = prompt_assets
            .into_iter()
            .map(Self::convert_prompt_asset)
            .collect();

        Ok(assets)
    }

    fn load_by_path(&self, path: &Path) -> Result<Asset> {
        // Use the existing parser
        let pa = crate::parser::parse_file(path)?;
        Ok(Self::convert_prompt_asset(pa))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_asset(dir: &Path, name: &str, content: &str) {
        let file = dir.join(format!("{}.md", name));
        std::fs::write(&file, content).unwrap();
    }

    #[test]
    fn load_all_from_empty_dir() {
        let dir = tempdir().unwrap();
        let repo = FsAssetRepository::new();

        let assets = repo.load_all(dir.path()).unwrap();

        assert!(assets.is_empty());
    }

    #[test]
    fn load_all_parses_assets() {
        let dir = tempdir().unwrap();
        create_test_asset(
            dir.path(),
            "test-policy",
            r#"---
description: A test policy
scope: project
---
# Policy Content

This is the policy body.
"#,
        );

        let repo = FsAssetRepository::new();
        let assets = repo.load_all(dir.path()).unwrap();

        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].id(), "test-policy");
        assert_eq!(assets[0].description(), "A test policy");
        assert_eq!(assets[0].scope(), Scope::Project);
    }

    #[test]
    fn load_by_path_parses_single_asset() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("single.md");
        std::fs::write(
            &file,
            r#"---
description: Single asset
scope: user
targets:
  - cursor
---
Content here
"#,
        )
        .unwrap();

        let repo = FsAssetRepository::new();
        let asset = repo.load_by_path(&file).unwrap();

        assert_eq!(asset.id(), "single");
        assert_eq!(asset.scope(), Scope::User);
        assert!(asset.targets().contains(&Target::Cursor));
    }
}
