//! Scope Policy for sync operations
//!
//! **Migration Note**: `ScopePolicy`, `DeploymentTarget`, and `ScopePolicyExt` are now defined in
//! `domain::policies` and re-exported here for backward compatibility.
//!
//! For new code, import directly from `crate::domain::policies`.

// Re-export domain types for backward compatibility
pub use crate::domain::policies::{DeploymentTarget, ScopePolicy, ScopePolicyExt};

// Tests moved to domain::policies::scope_policy
