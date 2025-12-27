//! LayerLoader port
//!
//! Loads assets for a resolved layer path.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::domain::entities::Layer;
use crate::domain::entities::{Asset, AssetKind};

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
    assets: &[Asset],
) -> Result<(), LayerLoadError> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut first_path_by_key: std::collections::HashMap<String, &std::path::PathBuf> =
        std::collections::HashMap::new();
    for asset in assets {
        let key = asset_key_for_uniqueness(asset);
        if !seen.insert(key.clone()) {
            let file1 = first_path_by_key
                .get(&key)
                .map(|p| (*p).clone())
                .unwrap_or_else(|| asset.source_path().clone());
            let file2 = asset.source_path().clone();
            return Err(LayerLoadError::DuplicateAssetInLayer {
                layer_name: layer_name.to_string(),
                asset_id: asset.id().to_string(),
                file1,
                file2,
            });
        }
        first_path_by_key.insert(key, asset.source_path());
    }
    Ok(())
}

fn asset_key_for_uniqueness(asset: &Asset) -> String {
    match asset.kind() {
        AssetKind::Skill => format!("skill:{}", asset.id()),
        _ => asset.id().to_string(),
    }
}
