//! Application Layer
//!
//! Use cases that orchestrate the business flow.
//! This layer:
//! - Depends on Domain layer (entities, services, ports)
//! - Does NOT contain business rules (those are in Domain)
//! - Coordinates between Infrastructure and Domain
//!
//! ## Use Cases
//!
//! - `DeployUseCase` - Orchestrates the deploy flow (load, compile, plan, execute, update lockfile)
//! - `CheckUseCase` - Orchestrates security checks
//! - `WatchUseCase` - Orchestrates file watching with auto-deploy
//! - `DiffUseCase` - Orchestrates diff preview
//!
//! ## Shared Operations
//!
//! - `layer_ops` - Unified asset loading from resolved layers

pub mod check;
pub mod clean;
pub mod deploy;
pub mod diff;
pub mod layer_ops;
pub mod layers;
mod lockfile_migration;
pub mod provenance;
pub mod registry;
pub(crate) mod skills;
pub mod watch;

pub use check::{CheckItem, CheckOptions, CheckResult, CheckStatus, CheckUseCase};
pub use clean::{CleanOptions, CleanResult, CleanUseCase, SkipReason, SkippedFile};
pub use deploy::{DeployOptions, DeployOutputOptions, DeployResult, DeployUseCase};
pub use diff::{ChangeType, DiffEntry, DiffOptions, DiffResult, DiffUseCase};
pub use lockfile_migration::global_lockfile_path;
pub use lockfile_migration::resolve_lockfile_path;
pub use registry::RegistryUseCase;
pub use watch::{
    compute_content_hash, parse_incremental, IncrementalCache, SyncResult, WatchEvent,
    WatchOptions, WatchUseCase, WatcherState, DEBOUNCE_MS,
};
