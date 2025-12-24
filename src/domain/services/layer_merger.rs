//! Layer merger
//!
//! Merges assets from multiple layers according to priority rules:
//! - Same asset ID: higher priority layer wins (replaces entirely)
//! - Different asset IDs: all are kept

use std::collections::HashMap;
use std::path::PathBuf;

use crate::domain::entities::{Asset, Layer};

#[derive(Debug, Clone, PartialEq)]
pub struct MergedAsset {
    pub asset: Asset,
    pub source_layer: String,
    pub source_layer_path: PathBuf,
    pub source_file: PathBuf,
    pub overrides: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverrideInfo {
    pub asset_id: String,
    pub from_layer: String,
    pub by_layer: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MergeResult {
    pub assets: HashMap<String, MergedAsset>,
    pub overrides: Vec<OverrideInfo>,
}

pub fn merge_layers(layers: &[Layer]) -> MergeResult {
    let mut merged: HashMap<String, MergedAsset> = HashMap::new();
    let mut overrides: Vec<OverrideInfo> = Vec::new();

    for layer in layers {
        for asset in &layer.assets {
            let id = asset.id().to_string();
            let previous_source_layer = merged.get(&id).map(|e| e.source_layer.clone());
            if let Some(from_layer) = previous_source_layer.clone() {
                overrides.push(OverrideInfo {
                    asset_id: id.clone(),
                    from_layer,
                    by_layer: layer.name.clone(),
                });
            }

            let source_layer_path = layer.path.resolved().clone();
            let source_file = source_layer_path.join(asset.source_path());

            merged.insert(
                id.clone(),
                MergedAsset {
                    asset: asset.clone(),
                    source_layer: layer.name.clone(),
                    source_layer_path,
                    source_file,
                    overrides: previous_source_layer,
                },
            );
        }
    }

    MergeResult {
        assets: merged,
        overrides,
    }
}

#[cfg(test)]
mod tests;
