//! Domain Policies
//!
//! Business rules and policies that govern behavior.
//! These are pure functions that operate on domain entities.

mod scope_policy;

pub use scope_policy::{DeploymentTarget, ScopePolicy};
