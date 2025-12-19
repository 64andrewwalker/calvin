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

pub mod deploy;

pub use deploy::{DeployOptions, DeployResult, DeployUseCase};
