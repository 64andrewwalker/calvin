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

    #[error("Duplicate asset id '{asset_id}' in layer '{layer_name}'")]
    DuplicateAssetId {
        layer_name: String,
        asset_id: String,
    },

    #[error("Failed to load assets: {message}")]
    LoadFailed { message: String },
}

pub(crate) fn ensure_unique_asset_ids(
    layer_name: &str,
    assets: &[crate::domain::entities::Asset],
) -> Result<(), LayerLoadError> {
    let mut seen: HashSet<&str> = HashSet::new();
    for asset in assets {
        if !seen.insert(asset.id()) {
            return Err(LayerLoadError::DuplicateAssetId {
                layer_name: layer_name.to_string(),
                asset_id: asset.id().to_string(),
            });
        }
    }
    Ok(())
}
