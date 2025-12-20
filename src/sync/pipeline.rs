//! Asset compilation pipeline (legacy location)
//!
//! **Migration Note**: `AssetPipeline` has been migrated to `application::pipeline`.
//! This module re-exports it for backward compatibility.
//!
//! For new code, import directly from `crate::application::AssetPipeline`.

// Re-export from new location
pub use crate::application::pipeline::AssetPipeline;
