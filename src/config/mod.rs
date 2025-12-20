//! Configuration module for Calvin
//!
//! Implements configuration hierarchy per TD-7:
//! 1. CLI flags (highest priority)
//! 2. Environment variables (CALVIN_*)
//! 3. Project config (.promptpack/config.toml)
//! 4. User config (~/.config/calvin/config.toml)
//! 5. Built-in defaults (lowest priority)

mod loader;
#[cfg(test)]
mod tests;
mod types;

// Re-export ConfigWarning from domain layer (for backward compatibility)
pub use crate::domain::value_objects::ConfigWarning;
// Re-export DeployTarget from domain layer
pub use crate::domain::value_objects::DeployTarget;

pub use types::{
    AnimationMode, ColorMode, Config, DenyConfig, DeployConfig, FormatConfig, McpConfig,
    McpServerConfig, OutputConfig, SecurityConfig, SecurityMcpConfig, SecurityMode, SyncConfig,
    TargetsConfig, Verbosity,
};

/// Legacy alias for backward compatibility
#[deprecated(since = "0.3.0", note = "Use DeployTarget instead")]
#[allow(dead_code)]
pub type DeployTargetConfig = DeployTarget;
