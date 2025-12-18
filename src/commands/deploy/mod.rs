//! Deploy module using two-stage sync
//!
//! Architecture:
//! - DeployRunner for the core logic
//! - Separates concerns into targets, options, runner
//! - Uses plan -> resolve -> execute two-stage sync

pub mod cmd;
pub mod options;
pub mod runner;
pub mod targets;

pub use cmd::{cmd_deploy, cmd_install, cmd_sync};
