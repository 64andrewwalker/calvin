//! Calvin - PromptOps compiler and synchronization tool
//!
//! Calvin enables teams to maintain AI rules, commands, and workflows in a single
//! source format (PromptPack), then compile and distribute them to multiple
//! AI coding assistant platforms.
//!
//! ## Architecture (v2)
//!
//! Calvin follows a layered architecture:
//! - `domain/` - Pure business logic (no I/O dependencies)
//! - `adapters/` - Target platform adapters (will move to infrastructure/)
//! - `sync/` - Sync engine (will be split across layers)
//!
//! See `docs/architecture/` for the full design.

// New architecture layers (v2)
pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod presentation;

// Legacy modules (will be refactored)
pub mod adapters;
pub mod config;
pub mod error;
pub mod fs;
pub mod models;
pub mod parser;
pub mod security;
pub(crate) mod security_baseline;
pub mod sync;
pub mod watcher;

// Re-exports for convenience
pub use adapters::{all_adapters, get_adapter, OutputFile, TargetAdapter};
pub use config::{Config, SecurityMode};
pub use error::{CalvinError, CalvinResult};
pub use models::{AssetKind, Frontmatter, PromptAsset, Scope, Target};
pub use parser::parse_frontmatter;
pub use security::{run_doctor, DoctorReport, DoctorSink};
pub use sync::{compile_assets, SyncEngine, SyncEngineOptions, SyncOptions, SyncResult};
pub use watcher::{parse_incremental, watch, IncrementalCache, WatchEvent, WatchOptions};
