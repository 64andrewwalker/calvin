//! AssetRepository port - abstraction for loading assets
//!
//! This trait allows the domain layer to load assets without
//! knowing about file system details.

use crate::domain::entities::Asset;
use anyhow::Result;
use std::path::Path;

/// Abstract repository for loading assets
///
/// Assets are the source files in `.promptpack/` directory.
/// This trait is implemented by infrastructure layer.
pub trait AssetRepository {
    /// Load all assets from a source directory
    fn load_all(&self, source: &Path) -> Result<Vec<Asset>>;

    /// Load a single asset by path
    fn load_by_path(&self, path: &Path) -> Result<Asset>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_repository_trait_is_object_safe() {
        fn _assert_object_safe(_: &dyn AssetRepository) {}
    }
}
