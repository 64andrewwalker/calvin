//! Deploy Module
//!
//! Orchestrates the deployment flow for Calvin.
//!
//! ## Structure
//!
//! - `options` - Configuration types (`DeployOptions`, `DeployOutputOptions`)
//! - `result` - Result types (`DeployResult`)
//! - `use_case` - Core use case logic (`DeployUseCase`)
//!
//! ## Usage
//!
//! ```ignore
//! use calvin::application::deploy::{DeployOptions, DeployResult, DeployUseCase};
//!
//! let use_case = DeployUseCase::new(asset_repo, lockfile_repo, fs, adapters);
//! let result = use_case.execute(&DeployOptions::new(source));
//! ```

mod options;
mod result;
mod use_case;

pub use options::{DeployOptions, DeployOutputOptions};
pub use result::DeployResult;
pub use use_case::DeployUseCase;

#[cfg(test)]
mod tests;
