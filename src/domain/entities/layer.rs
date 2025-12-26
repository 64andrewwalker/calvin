//! Layer entity
//!
//! A layer represents a promptpack source (user/custom/project) along with its loaded assets.

use std::path::PathBuf;

use crate::domain::entities::Asset;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerType {
    User,
    Custom,
    Project,
}

/// A layer path, optionally tracking the original (symlink) path for display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerPath {
    original: PathBuf,
    resolved: PathBuf,
}

impl LayerPath {
    pub fn new(original: PathBuf, resolved: PathBuf) -> Self {
        Self { original, resolved }
    }

    pub fn original(&self) -> &PathBuf {
        &self.original
    }

    pub fn resolved(&self) -> &PathBuf {
        &self.resolved
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub name: String,
    pub path: LayerPath,
    pub layer_type: LayerType,
    pub assets: Vec<Asset>,
}

impl Layer {
    pub fn new(name: impl Into<String>, path: LayerPath, layer_type: LayerType) -> Self {
        Self {
            name: name.into(),
            path,
            layer_type,
            assets: Vec::new(),
        }
    }

    pub fn with_assets(mut self, assets: Vec<Asset>) -> Self {
        self.assets = assets;
        self
    }
}

#[cfg(test)]
mod tests;
