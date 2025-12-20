//! Deploy module using DeployUseCase
//!
//! Architecture:
//! - DeployUseCase from application layer handles deployment logic
//! - bridge.rs provides conversion functions for CLI options
//! - targets.rs defines deployment target types
//! - options.rs defines CLI options

pub mod bridge;
pub mod cmd;
pub mod options;
pub mod targets;

pub use cmd::{cmd_deploy, cmd_deploy_with_explicit_target, cmd_install, cmd_sync};
