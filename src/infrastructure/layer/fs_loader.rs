//! File system LayerLoader implementation

use crate::domain::entities::Layer;
use crate::domain::ports::layer_loader::{ensure_unique_asset_ids, LayerLoadError, LayerLoader};
use crate::domain::ports::AssetRepository;
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

        let assets =
            self.asset_repo
                .load_all(layer_root)
                .map_err(|e| LayerLoadError::LoadFailed {
                    message: e.to_string(),
                })?;

        ensure_unique_asset_ids(&layer.name, &assets)?;
        layer.assets = assets;
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
}
