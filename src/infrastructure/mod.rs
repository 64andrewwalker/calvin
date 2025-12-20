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
//! - `config/` - Configuration loading implementations
//! - `sync/` - Sync destination implementations (Local, Remote)

pub mod adapters;
pub mod config;
pub mod fs;
pub mod repositories;
pub mod sync;

// Re-export for convenience
pub use adapters::{all_adapters, get_adapter, ClaudeCodeAdapter, CursorAdapter};
pub use config::TomlConfigRepository;
pub use fs::{LocalFs, RemoteFs};
pub use repositories::{FsAssetRepository, TomlLockfileRepository};
pub use sync::{LocalHomeDestination, LocalProjectDestination, RemoteDestination};
