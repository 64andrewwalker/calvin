//! Infrastructure Layer
//!
//! Concrete implementations of domain ports.
//! This layer handles all I/O operations.
//!
//! ## Structure
//!
//! - `adapters/` - Target adapters (ClaudeCode, Cursor, VSCode, etc.)
//! - `config/` - Configuration loading implementations
//! - `events/` - Event sink implementations (JSON, Console)
//! - `fs/` - File system implementations (Local, Remote)
//! - `repositories/` - Repository implementations (Lockfile, Asset)
//! - `sync/` - Sync destination implementations (Local, Remote)

pub mod adapters;
pub mod config;
pub mod conflict;
pub mod events;
pub mod fs;
pub mod repositories;
pub mod sync;

// Re-export for convenience
pub use adapters::{all_adapters, get_adapter, ClaudeCodeAdapter, CursorAdapter};
pub use config::TomlConfigRepository;
pub use conflict::InteractiveResolver;
pub use events::JsonEventSink;
pub use fs::{LocalFs, RemoteFs};
pub use repositories::{FsAssetRepository, TomlLockfileRepository};
pub use sync::{LocalHomeDestination, LocalProjectDestination, RemoteDestination};
