//! File system LayerLoader implementation

use crate::domain::entities::Layer;
use crate::domain::ports::layer_loader::{ensure_unique_asset_ids, LayerLoadError, LayerLoader};
use crate::domain::value_objects::IgnorePatterns;
use crate::infrastructure::repositories::FsAssetRepository;

pub struct FsLayerLoader {
    asset_repo: FsAssetRepository,
}

impl FsLayerLoader {
    pub fn new(asset_repo: FsAssetRepository) -> Self {
        Self { asset_repo }
    }
}

impl Default for FsLayerLoader {
    fn default() -> Self {
        Self::new(FsAssetRepository::new())
    }
}

impl LayerLoader for FsLayerLoader {
    fn load_layer_assets(&self, layer: &mut Layer) -> Result<(), LayerLoadError> {
        let layer_root = layer.path.resolved();
        if !layer_root.exists() {
            return Err(LayerLoadError::PathNotFound {
                path: layer_root.clone(),
            });
        }

        if !layer_root.is_dir() {
            return Err(LayerLoadError::InvalidLayerPath {
                path: layer_root.clone(),
            });
        }

        if let Err(e) = std::fs::read_dir(layer_root) {
            if matches!(e.kind(), std::io::ErrorKind::PermissionDenied) {
                return Err(LayerLoadError::PermissionDenied {
                    path: layer_root.clone(),
                });
            }
            return Err(LayerLoadError::LoadFailed {
                message: e.to_string(),
            });
        }

        // Load .calvinignore patterns for this layer
        let ignore = IgnorePatterns::load(layer_root).map_err(|e| LayerLoadError::LoadFailed {
            message: format!("Failed to load .calvinignore: {}", e),
        })?;

        // Load assets with ignore filtering
        let (assets, ignored_count) = self
            .asset_repo
            .load_all_with_ignore(layer_root, &ignore)
            .map_err(|e| LayerLoadError::LoadFailed {
                message: e.to_string(),
            })?;

        ensure_unique_asset_ids(&layer.name, &assets)?;
        layer.assets = assets;
        layer.ignored_count = ignored_count;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{LayerPath, LayerType};
    use tempfile::tempdir;

    #[test]
    fn fs_loader_loads_assets() {
        let dir = tempdir().unwrap();
        let layer_path = dir.path().join(".promptpack");
        std::fs::create_dir_all(&layer_path).unwrap();
        std::fs::write(
            layer_path.join("test.md"),
            r#"---
description: A test policy
scope: project
---
# Policy Content
"#,
        )
        .unwrap();

        let loader = FsLayerLoader::default();
        let mut layer = crate::domain::entities::Layer::new(
            "test",
            LayerPath::new(layer_path.clone(), layer_path.clone()),
            LayerType::Project,
        );

        loader.load_layer_assets(&mut layer).unwrap();
        assert_eq!(layer.assets.len(), 1);
        assert_eq!(layer.assets[0].id(), "test");
    }

    #[test]
    fn fs_loader_error_on_missing_path() {
        let loader = FsLayerLoader::default();
        let missing = std::path::PathBuf::from("/nonexistent/calvin-layer");
        let mut layer = crate::domain::entities::Layer::new(
            "test",
            LayerPath::new(missing.clone(), missing.clone()),
            LayerType::Project,
        );

        let result = loader.load_layer_assets(&mut layer);
        assert!(matches!(result, Err(LayerLoadError::PathNotFound { .. })));
    }

    #[test]
    fn fs_loader_error_on_invalid_layer_path() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".promptpack");
        std::fs::write(&path, "not a directory").unwrap();

        let loader = FsLayerLoader::default();
        let mut layer = crate::domain::entities::Layer::new(
            "test",
            LayerPath::new(path.clone(), path.clone()),
            LayerType::Project,
        );

        let result = loader.load_layer_assets(&mut layer);
        assert!(matches!(
            result,
            Err(LayerLoadError::InvalidLayerPath { .. })
        ));
    }

    #[test]
    fn fs_loader_error_on_duplicate_asset_id_in_layer() {
        let dir = tempdir().unwrap();
        let layer_path = dir.path().join(".promptpack");
        std::fs::create_dir_all(layer_path.join("policies")).unwrap();
        std::fs::create_dir_all(layer_path.join("actions")).unwrap();

        // Both files will produce the same asset ID ("test")
        std::fs::write(
            layer_path.join("policies/test.md"),
            r#"---
kind: policy
description: Policy test
scope: project
---
POLICY
"#,
        )
        .unwrap();
        std::fs::write(
            layer_path.join("actions/test.md"),
            r#"---
kind: action
description: Action test
scope: project
---
ACTION
"#,
        )
        .unwrap();

        let loader = FsLayerLoader::default();
        let mut layer = crate::domain::entities::Layer::new(
            "test-layer",
            LayerPath::new(layer_path.clone(), layer_path.clone()),
            LayerType::Project,
        );

        let result = loader.load_layer_assets(&mut layer);
        assert!(matches!(
            result,
            Err(LayerLoadError::DuplicateAssetInLayer { .. })
        ));
    }

    #[test]
    fn fs_loader_allows_skill_id_to_overlap_action_id() {
        let dir = tempdir().unwrap();
        let layer_path = dir.path().join(".promptpack");
        std::fs::create_dir_all(layer_path.join("actions")).unwrap();
        std::fs::create_dir_all(layer_path.join("skills/review")).unwrap();

        // Action asset: id = "review"
        std::fs::write(
            layer_path.join("actions/review.md"),
            r#"---
kind: action
description: Review action
scope: project
---
ACTION
"#,
        )
        .unwrap();

        // Skill asset: id = directory name "review" (overlaps action id)
        std::fs::write(
            layer_path.join("skills/review/SKILL.md"),
            r#"---
description: Review skill
scope: project
targets:
  - claude-code
---

# Instructions
"#,
        )
        .unwrap();

        let loader = FsLayerLoader::default();
        let mut layer = crate::domain::entities::Layer::new(
            "test-layer",
            LayerPath::new(layer_path.clone(), layer_path.clone()),
            LayerType::Project,
        );

        loader.load_layer_assets(&mut layer).unwrap();

        // Both assets should be present.
        assert_eq!(layer.assets.len(), 2);
        assert!(layer
            .assets
            .iter()
            .any(|a| a.kind() == crate::domain::entities::AssetKind::Action));
        assert!(layer
            .assets
            .iter()
            .any(|a| a.kind() == crate::domain::entities::AssetKind::Skill));
        assert!(layer.assets.iter().any(|a| a.id() == "review"));
    }

    // ============== .calvinignore tests ==============

    #[test]
    fn fs_loader_respects_calvinignore_for_files() {
        let dir = tempdir().unwrap();
        let layer_path = dir.path().join(".promptpack");
        std::fs::create_dir_all(&layer_path).unwrap();

        // Create two assets
        std::fs::write(
            layer_path.join("draft.md"),
            r#"---
description: A draft policy
scope: project
---
# Draft
"#,
        )
        .unwrap();
        std::fs::write(
            layer_path.join("final.md"),
            r#"---
description: A final policy
scope: project
---
# Final
"#,
        )
        .unwrap();

        // Ignore the draft
        std::fs::write(layer_path.join(".calvinignore"), "draft.md\n").unwrap();

        let loader = FsLayerLoader::default();
        let mut layer = Layer::new(
            "test",
            LayerPath::new(layer_path.clone(), layer_path.clone()),
            LayerType::Project,
        );

        loader.load_layer_assets(&mut layer).unwrap();

        assert_eq!(layer.assets.len(), 1);
        assert_eq!(layer.assets[0].id(), "final");
        assert_eq!(layer.ignored_count, 1);
    }

    #[test]
    fn fs_loader_respects_calvinignore_for_directories() {
        let dir = tempdir().unwrap();
        let layer_path = dir.path().join(".promptpack");
        std::fs::create_dir_all(layer_path.join("drafts")).unwrap();

        // Create assets in drafts/
        std::fs::write(
            layer_path.join("drafts/wip.md"),
            r#"---
description: Work in progress
scope: project
---
# WIP
"#,
        )
        .unwrap();
        std::fs::write(
            layer_path.join("final.md"),
            r#"---
description: Final
scope: project
---
# Final
"#,
        )
        .unwrap();

        // Ignore the drafts directory
        std::fs::write(layer_path.join(".calvinignore"), "drafts/\n").unwrap();

        let loader = FsLayerLoader::default();
        let mut layer = Layer::new(
            "test",
            LayerPath::new(layer_path.clone(), layer_path.clone()),
            LayerType::Project,
        );

        loader.load_layer_assets(&mut layer).unwrap();

        assert_eq!(layer.assets.len(), 1);
        assert_eq!(layer.assets[0].id(), "final");
    }

    #[test]
    fn fs_loader_respects_calvinignore_for_skills() {
        let dir = tempdir().unwrap();
        let layer_path = dir.path().join(".promptpack");
        std::fs::create_dir_all(layer_path.join("skills/experimental")).unwrap();
        std::fs::create_dir_all(layer_path.join("skills/stable")).unwrap();

        // Create two skills
        std::fs::write(
            layer_path.join("skills/experimental/SKILL.md"),
            r#"---
description: An experimental skill
scope: project
---
# Experimental
"#,
        )
        .unwrap();
        std::fs::write(
            layer_path.join("skills/stable/SKILL.md"),
            r#"---
description: A stable skill
scope: project
---
# Stable
"#,
        )
        .unwrap();

        // Ignore the experimental skill
        std::fs::write(layer_path.join(".calvinignore"), "skills/experimental/\n").unwrap();

        let loader = FsLayerLoader::default();
        let mut layer = Layer::new(
            "test",
            LayerPath::new(layer_path.clone(), layer_path.clone()),
            LayerType::Project,
        );

        loader.load_layer_assets(&mut layer).unwrap();

        assert_eq!(layer.assets.len(), 1);
        assert_eq!(layer.assets[0].id(), "stable");
        assert_eq!(layer.ignored_count, 1);
    }

    #[test]
    fn fs_loader_respects_calvinignore_for_skill_supplementals() {
        let dir = tempdir().unwrap();
        let layer_path = dir.path().join(".promptpack");
        std::fs::create_dir_all(layer_path.join("skills/my-skill")).unwrap();

        // Create a skill with supplementals
        std::fs::write(
            layer_path.join("skills/my-skill/SKILL.md"),
            r#"---
description: My skill
scope: project
---
# My Skill
"#,
        )
        .unwrap();
        std::fs::write(
            layer_path.join("skills/my-skill/reference.md"),
            "# Reference\n",
        )
        .unwrap();
        std::fs::write(layer_path.join("skills/my-skill/notes.txt"), "Some notes\n").unwrap();

        // Ignore the notes file
        std::fs::write(
            layer_path.join(".calvinignore"),
            "skills/my-skill/notes.txt\n",
        )
        .unwrap();

        let loader = FsLayerLoader::default();
        let mut layer = Layer::new(
            "test",
            LayerPath::new(layer_path.clone(), layer_path.clone()),
            LayerType::Project,
        );

        loader.load_layer_assets(&mut layer).unwrap();

        assert_eq!(layer.assets.len(), 1);
        let skill = &layer.assets[0];
        assert!(skill
            .supplementals()
            .contains_key(&std::path::PathBuf::from("reference.md")));
        assert!(!skill
            .supplementals()
            .contains_key(&std::path::PathBuf::from("notes.txt")));
    }

    #[test]
    fn fs_loader_glob_pattern_ignores_files() {
        let dir = tempdir().unwrap();
        let layer_path = dir.path().join(".promptpack");
        std::fs::create_dir_all(&layer_path).unwrap();

        std::fs::write(
            layer_path.join("policy.md"),
            r#"---
description: Policy
scope: project
---
# Policy
"#,
        )
        .unwrap();
        std::fs::write(
            layer_path.join("old-policy.bak"),
            r#"---
description: Old backup
scope: project
---
# Old
"#,
        )
        .unwrap();

        // Ignore all .bak files
        std::fs::write(layer_path.join(".calvinignore"), "*.bak\n").unwrap();

        let loader = FsLayerLoader::default();
        let mut layer = Layer::new(
            "test",
            LayerPath::new(layer_path.clone(), layer_path.clone()),
            LayerType::Project,
        );

        loader.load_layer_assets(&mut layer).unwrap();

        assert_eq!(layer.assets.len(), 1);
        assert_eq!(layer.assets[0].id(), "policy");
    }

    #[test]
    fn fs_loader_no_calvinignore_loads_all() {
        let dir = tempdir().unwrap();
        let layer_path = dir.path().join(".promptpack");
        std::fs::create_dir_all(&layer_path).unwrap();

        std::fs::write(
            layer_path.join("one.md"),
            r#"---
description: One
scope: project
---
# One
"#,
        )
        .unwrap();
        std::fs::write(
            layer_path.join("two.md"),
            r#"---
description: Two
scope: project
---
# Two
"#,
        )
        .unwrap();

        // No .calvinignore file

        let loader = FsLayerLoader::default();
        let mut layer = Layer::new(
            "test",
            LayerPath::new(layer_path.clone(), layer_path.clone()),
            LayerType::Project,
        );

        loader.load_layer_assets(&mut layer).unwrap();

        assert_eq!(layer.assets.len(), 2);
        assert_eq!(layer.ignored_count, 0);
    }
}
