//! Calvin - PromptOps compiler and synchronization tool
//!
//! Calvin enables teams to maintain AI rules, commands, and workflows in a single
//! source format (PromptPack), then compile and distribute them to multiple
//! AI coding assistant platforms.

pub mod parser;
pub mod models;
pub mod error;
pub mod adapters;
pub mod sync;
pub mod config;
pub mod security;
pub mod watcher;

// Re-exports for convenience
pub use error::{CalvinError, CalvinResult};
pub use models::{Frontmatter, PromptAsset, AssetKind, Scope, Target};
pub use parser::parse_frontmatter;
pub use adapters::{TargetAdapter, OutputFile, all_adapters, get_adapter};
pub use sync::{SyncOptions, SyncResult, compile_assets, sync_outputs};
pub use config::{Config, SecurityMode};
pub use security::{DoctorReport, run_doctor};
pub use watcher::{WatchOptions, WatchEvent, watch};
