//! Layer Operations
//!
//! Shared utilities for loading assets from resolved layers.
//!
//! This module provides the unified entry point for asset loading,
//! ensuring consistent behavior (including `.calvinignore` support)
//! across all use cases: Deploy, Diff, Watch, and Layer Query.

use crate::domain::entities::Layer;
use crate::domain::ports::layer_loader::{ensure_unique_asset_ids, LayerLoadError};
use crate::domain::ports::AssetRepository;
use crate::domain::value_objects::IgnorePatterns;

/// Load assets for a list of resolved layers.
///
/// This function is the **single source of truth** for loading assets from layers.
/// It ensures consistent behavior across all consumers:
///
/// - `DeployUseCase`
/// - `DiffUseCase`
/// - `WatchUseCase`
/// - `FsLayerLoader`
///
/// # What this function does
///
/// 1. Reads `.calvinignore` from each layer
/// 2. Loads assets using `AssetRepository` with ignore filtering
/// 3. Ensures unique asset IDs within each layer
/// 4. Populates `layer.assets` and `layer.ignored_count`
///
/// # Errors
///
/// Returns an error if:
/// - `.calvinignore` cannot be loaded (invalid syntax)
/// - Asset loading fails (I/O error, parse error)
/// - Duplicate asset IDs are found within a layer
pub fn load_resolved_layers<AR: AssetRepository>(
    asset_repo: &AR,
    layers: &mut [Layer],
) -> Result<(), LayerLoadError> {
    for layer in layers {
        let layer_root = layer.path.resolved();

        let ignore = IgnorePatterns::load(layer_root).map_err(|e| LayerLoadError::LoadFailed {
            message: format!("Failed to load .calvinignore for '{}': {}", layer.name, e),
        })?;

        let (assets, ignored_count) = asset_repo
            .load_all_with_ignore(layer_root, &ignore)
            .map_err(|e| LayerLoadError::LoadFailed {
                message: format!("Failed to load layer '{}': {}", layer.name, e),
            })?;

        ensure_unique_asset_ids(&layer.name, &assets)?;

        layer.assets = assets;
        layer.ignored_count = ignored_count;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{Asset, LayerPath, LayerType};
    use crate::domain::value_objects::IgnorePatterns;
    use anyhow::Result;
    use std::cell::RefCell;
    use std::path::Path;
    use tempfile::tempdir;

    struct MockAssetRepository {
        assets: Vec<Asset>,
        load_all_with_ignore_calls: RefCell<Vec<std::path::PathBuf>>,
    }

    impl AssetRepository for MockAssetRepository {
        fn load_all(&self, _source: &Path) -> Result<Vec<Asset>> {
            Ok(self.assets.clone())
        }

        fn load_all_with_ignore(
            &self,
            source: &Path,
            _ignore: &IgnorePatterns,
        ) -> Result<(Vec<Asset>, usize)> {
            self.load_all_with_ignore_calls
                .borrow_mut()
                .push(source.to_path_buf());
            Ok((self.assets.clone(), 0))
        }

        fn load_by_path(&self, _path: &Path) -> Result<Asset> {
            self.assets
                .first()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Asset not found"))
        }
    }

    #[test]
    fn load_resolved_layers_populates_layer_assets() {
        let dir = tempdir().unwrap();
        let layer_path = dir.path().to_path_buf();

        let assets = vec![Asset::new("test", "test.md", "Test asset", "# Content")];
        let repo = MockAssetRepository {
            assets,
            load_all_with_ignore_calls: RefCell::new(Vec::new()),
        };

        let mut layers = vec![Layer::new(
            "test-layer",
            LayerPath::new(layer_path.clone(), layer_path.clone()),
            LayerType::Project,
        )];

        let result = load_resolved_layers(&repo, &mut layers);

        assert!(result.is_ok());
        assert_eq!(layers[0].assets.len(), 1);
        assert_eq!(layers[0].assets[0].id(), "test");
    }

    #[test]
    fn load_resolved_layers_calls_load_all_with_ignore() {
        let dir = tempdir().unwrap();
        let layer_path = dir.path().to_path_buf();

        let repo = MockAssetRepository {
            assets: Vec::new(),
            load_all_with_ignore_calls: RefCell::new(Vec::new()),
        };

        let mut layers = vec![Layer::new(
            "test-layer",
            LayerPath::new(layer_path.clone(), layer_path.clone()),
            LayerType::Project,
        )];

        let _ = load_resolved_layers(&repo, &mut layers);

        let calls = repo.load_all_with_ignore_calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], layer_path);
    }
}
