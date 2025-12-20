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
//! - `AssetPipeline` - Parse, filter, and compile assets
//!
//! ## Services
//!
//! - `compile_assets` - Compile PromptAssets to OutputFiles using adapters

pub mod check;
pub mod compiler;
pub mod deploy;
pub mod diff;
pub mod pipeline;
pub mod watch;

pub use check::{CheckItem, CheckOptions, CheckResult, CheckStatus, CheckUseCase};
pub use compiler::compile_assets;
pub use deploy::{DeployOptions, DeployOutputOptions, DeployResult, DeployUseCase};
pub use diff::{ChangeType, DiffEntry, DiffOptions, DiffResult, DiffUseCase};
pub use pipeline::{AssetPipeline, ScopePolicyExt};
pub use watch::{WatchEvent, WatchOptions, WatchUseCase};
