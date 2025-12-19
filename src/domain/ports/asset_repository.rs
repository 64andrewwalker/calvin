//! AssetRepository port - abstraction for loading assets
//!
//! This trait allows the domain layer to load assets without
//! knowing about file system details.

use std::path::Path;

/// Result type for asset operations
pub type AssetResult<T> = Result<T, AssetError>;

/// Asset loading errors
#[derive(Debug)]
pub enum AssetError {
    /// Asset not found
    NotFound(String),
    /// Invalid asset format
    InvalidFormat(String),
    /// I/O error during loading
    Io(std::io::Error),
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetError::NotFound(path) => write!(f, "Asset not found: {}", path),
            AssetError::InvalidFormat(msg) => write!(f, "Invalid asset format: {}", msg),
            AssetError::Io(err) => write!(f, "I/O error: {}", err),
        }
    }
}

impl std::error::Error for AssetError {}

/// Abstract repository for loading assets
///
/// Assets are the source files in `.promptpack/` directory.
/// This trait is implemented by infrastructure layer.
pub trait AssetRepository {
    /// Asset type (to be defined in entities)
    type Asset;

    /// Load all assets from a source directory
    fn load_all(&self, source: &Path) -> AssetResult<Vec<Self::Asset>>;

    /// Load a single asset by path
    fn load(&self, path: &Path) -> AssetResult<Self::Asset>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_error_display() {
        let err = AssetError::NotFound("test.md".to_string());
        assert!(err.to_string().contains("test.md"));
    }
}
