//! LayerLoader port
//!
//! Loads assets for a resolved layer path.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::domain::entities::Layer;

pub trait LayerLoader: Send + Sync {
    fn load_layer_assets(&self, layer: &mut Layer) -> Result<(), LayerLoadError>;
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LayerLoadError {
    #[error("Layer path not found: {path}")]
    PathNotFound { path: PathBuf },

    #[error("Layer path is not a directory: {path}")]
    InvalidLayerPath { path: PathBuf },

    #[error("Permission denied reading layer: {path}\n  → Fix: Check file permissions\n  → Run: chmod -R +r {path}")]
    PermissionDenied { path: PathBuf },

    #[error("duplicate asset ID '{asset_id}' in layer '{layer_name}'\n  → Files:\n    1. {file1}\n    2. {file2}\n  → Fix: Rename one of the assets to have a unique ID")]
    DuplicateAssetInLayer {
        layer_name: String,
        asset_id: String,
        file1: PathBuf,
        file2: PathBuf,
    },

    #[error("Failed to load assets: {message}")]
    LoadFailed { message: String },
}

pub(crate) fn ensure_unique_asset_ids(
    layer_name: &str,
    assets: &[crate::domain::entities::Asset],
) -> Result<(), LayerLoadError> {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut first_path_by_id: std::collections::HashMap<&str, &std::path::PathBuf> =
        std::collections::HashMap::new();
    for asset in assets {
        let id = asset.id();
        if !seen.insert(id) {
            let file1 = first_path_by_id
                .get(id)
                .map(|p| (*p).clone())
                .unwrap_or_else(|| asset.source_path().clone());
            let file2 = asset.source_path().clone();
            return Err(LayerLoadError::DuplicateAssetInLayer {
                layer_name: layer_name.to_string(),
                asset_id: id.to_string(),
                file1,
                file2,
            });
        }
        first_path_by_id.insert(id, asset.source_path());
    }
    Ok(())
}
