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
//! - `application/` - Use cases and orchestration
//! - `infrastructure/` - Adapters and external integrations
//!
//! See `docs/architecture/` for the full design.

// Architecture layers
pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod presentation;

// Core modules
pub mod config;
pub mod error;
pub mod models;
pub mod parser;
pub mod security;

// Re-exports for convenience
pub use application::{compile_assets, DeployResult};
pub use config::{Config, SecurityMode};
pub use domain::entities::OutputFile;
pub use domain::ports::TargetAdapter;
pub use error::{CalvinError, CalvinResult};
pub use infrastructure::adapters::{all_adapters, get_adapter};
pub use models::{AssetKind, Frontmatter, PromptAsset, Scope, Target};
pub use parser::{extract_frontmatter, parse_frontmatter};
pub use security::{run_doctor, DoctorReport, DoctorSink};

// Re-export for fuzz testing
pub use domain::value_objects::parse_lockfile_key;
pub use serde_yaml_ng;

// Watch module re-exports (from application layer)
pub use application::watch::{
    parse_incremental, IncrementalCache, WatchEvent, WatchOptions, WatchUseCase,
};
