//! Calvin - PromptOps compiler and synchronization tool
//!
//! Calvin enables teams to maintain AI rules, commands, and workflows in a single
//! source format (PromptPack), then compile and distribute them to multiple
//! AI coding assistant platforms.

pub mod adapters;
pub mod config;
pub mod error;
pub mod fs;
pub mod models;
pub mod parser;
pub mod security;
pub mod security_baseline;
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
