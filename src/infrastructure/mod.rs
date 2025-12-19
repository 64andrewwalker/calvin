//! Infrastructure Layer
//!
//! Concrete implementations of domain ports.
//! This layer handles all I/O operations.
//!
//! ## Structure
//!
//! - `fs/` - File system implementations (Local, Remote)
//! - `repositories/` - Repository implementations (Lockfile, Asset)
//! - `adapters/` - Target adapters (ClaudeCode, Cursor, VSCode, etc.)

pub mod fs;
pub mod repositories;

// Re-export for convenience
pub use fs::{LocalFs, RemoteFs};
pub use repositories::{FsAssetRepository, TomlLockfileRepository};
