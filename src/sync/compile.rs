//! Asset compilation module (legacy location)
//!
//! **Migration Note**: `compile_assets` has been migrated to `application::compiler`.
//! This module re-exports it for backward compatibility.
//!
//! For new code, import directly from `crate::application::compile_assets`.

// Re-export from new location
pub use crate::application::compile_assets;
