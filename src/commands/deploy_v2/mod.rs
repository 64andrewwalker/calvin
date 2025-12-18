//! Refactored deploy module using two-stage sync
//!
//! This is a clean reimplementation that:
//! - Uses DeployRunner for the core logic
//! - Separates concerns into targets, options, runner
//! - Uses plan -> resolve -> execute two-stage sync

pub mod targets;
pub mod options;
pub mod runner;

pub use targets::{DeployTarget, ScopePolicy};
pub use options::DeployOptions;
pub use runner::DeployRunner;
