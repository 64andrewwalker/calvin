//! Configuration type definitions

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::CalvinResult;
use crate::models::Target;

use super::loader::{self, ConfigWarning};

// Re-export SecurityMode from domain layer
pub use crate::domain::value_objects::SecurityMode;

/// Format version configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatConfig {
    #[serde(default = "default_format_version")]
    pub version: String,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            version: default_format_version(),
        }
    }
}

fn default_format_version() -> String {
    "1.0".to_string()
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    #[serde(default)]
    pub mode: SecurityMode,

    #[serde(default)]
    pub deny: DenyConfig,

    #[serde(default)]
    pub allow_naked: bool,

    #[serde(default)]
    pub mcp: SecurityMcpConfig,
}

/// MCP allowlist configuration for `doctor` checks.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityMcpConfig {
    #[serde(default)]
    pub allowlist: Vec<String>,

    #[serde(default)]
    pub additional_allowlist: Vec<String>,
}

/// Deny list configuration for security baselines.
///
/// Supports both legacy array form:
///   [security]
///   deny = ["*.secret"]
///
/// And structured table form:
///   [security.deny]
///   patterns = ["*.secret"]
///   exclude = [".env.example"]
#[derive(Debug, Clone, Serialize, Default)]
pub struct DenyConfig {
    pub patterns: Vec<String>,
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum DenyConfigDe {
    List(Vec<String>),
    Table {
        #[serde(default)]
        patterns: Vec<String>,
        #[serde(default)]
        exclude: Vec<String>,
    },
}

impl<'de> Deserialize<'de> for DenyConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match DenyConfigDe::deserialize(deserializer)? {
            DenyConfigDe::List(patterns) => Ok(Self {
                patterns,
                exclude: Vec::new(),
            }),
            DenyConfigDe::Table { patterns, exclude } => Ok(Self { patterns, exclude }),
        }
    }
}

/// Target configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TargetsConfig {
    #[serde(default)]
    pub enabled: Vec<Target>,
}

/// Sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    #[serde(default = "default_true")]
    pub atomic_writes: bool,

    #[serde(default = "default_true")]
    pub respect_lockfile: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            atomic_writes: true,
            respect_lockfile: true,
        }
    }
}

fn default_true() -> bool {
    true
}

/// Deploy target configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DeployTargetConfig {
    /// Not configured (user needs to choose)
    #[default]
    Unset,
    /// Deploy to project directory
    Project,
    /// Deploy to user home directory
    Home,
}

/// Deploy configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeployConfig {
    /// Default deploy target (project or home)
    #[serde(default)]
    pub target: DeployTargetConfig,
}

/// Output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default)]
    pub verbosity: Verbosity,

    #[serde(default)]
    pub color: ColorMode,

    #[serde(default)]
    pub animation: AnimationMode,

    #[serde(default = "default_true")]
    pub unicode: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            verbosity: Verbosity::default(),
            color: ColorMode::default(),
            animation: AnimationMode::default(),
            unicode: true,
        }
    }
}

/// Color output mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}

/// Animation output mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AnimationMode {
    #[default]
    Auto,
    Always,
    Never,
    Minimal,
}

/// Verbosity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Verbosity {
    Quiet,
    #[default]
    Normal,
    Verbose,
    Debug,
}

/// MCP Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

/// MCP configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpConfig {
    #[serde(default)]
    pub servers: HashMap<String, McpServerConfig>,
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub format: FormatConfig,

    #[serde(default)]
    pub security: SecurityConfig,

    #[serde(default)]
    pub targets: TargetsConfig,

    #[serde(default)]
    pub sync: SyncConfig,

    #[serde(default)]
    pub output: OutputConfig,

    #[serde(default)]
    pub mcp: McpConfig,

    #[serde(default)]
    pub deploy: DeployConfig,
}

impl Config {
    /// Load configuration from a TOML file
    pub fn load(path: &Path) -> CalvinResult<Self> {
        let (config, _warnings) = loader::load_with_warnings(path)?;
        Ok(config)
    }

    /// Load configuration and collect non-fatal warnings (e.g. unknown keys).
    pub fn load_with_warnings(path: &Path) -> CalvinResult<(Self, Vec<ConfigWarning>)> {
        loader::load_with_warnings(path)
    }

    /// Load from project config, user config, or defaults
    pub fn load_or_default(project_root: Option<&Path>) -> Self {
        loader::load_or_default(project_root)
    }

    /// Apply environment variable overrides (CALVIN_* prefix)
    pub fn with_env_overrides(self) -> Self {
        loader::with_env_overrides(self)
    }

    /// Get enabled targets (all if empty)
    pub fn enabled_targets(&self) -> Vec<Target> {
        if self.targets.enabled.is_empty() {
            vec![
                Target::ClaudeCode,
                Target::Cursor,
                Target::VSCode,
                Target::Antigravity,
                Target::Codex,
            ]
        } else {
            self.targets.enabled.clone()
        }
    }

    /// Save deploy target to config file
    ///
    /// Updates or creates the [deploy] section in config.toml with the target.
    /// Preserves other settings in the file.
    pub fn save_deploy_target(config_path: &Path, target: DeployTargetConfig) -> CalvinResult<()> {
        loader::save_deploy_target(config_path, target)
    }
}
