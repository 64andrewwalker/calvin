//! Configuration type definitions

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::ports::DomainConfig;
use crate::domain::value_objects::{ConfigWarning, Target};
use crate::error::CalvinResult;

use super::loader;

// Re-export domain types for backward compatibility
pub use crate::domain::value_objects::DeployTarget;
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
///
/// The `enabled` field has three distinct states:
/// - `None` (field missing): Use default behavior (all targets)
/// - `Some([])` (empty list): Explicitly disable all targets (deploy nothing)
/// - `Some([...])` (list with items): Deploy only to specified targets
#[derive(Debug, Clone, Serialize, Default)]
pub struct TargetsConfig {
    /// Enabled targets. None = all targets, Some([]) = no targets, Some([...]) = specified targets
    #[serde(default)]
    pub enabled: Option<Vec<Target>>,
}

// Custom deserialize to distinguish between missing field and empty list
impl<'de> Deserialize<'de> for TargetsConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct TargetsConfigHelper {
            enabled: Option<Vec<Target>>,
        }

        let helper = TargetsConfigHelper::deserialize(deserializer)?;
        Ok(TargetsConfig {
            enabled: helper.enabled,
        })
    }
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

/// Sources configuration for multi-layer promptpacks.
///
/// Security note (PRD §8 / design D8):
/// - User config may define additional layer paths and user layer path.
/// - Project config may only disable layers via ignore flags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcesConfig {
    /// Whether to use the user layer (~/.calvin/.promptpack by default).
    #[serde(default = "default_true")]
    pub use_user_layer: bool,

    /// Project-level switch: disable user layer regardless of user config.
    #[serde(default)]
    pub ignore_user_layer: bool,

    /// User layer path override (may be configured by the user config or env var).
    #[serde(default)]
    pub user_layer_path: Option<PathBuf>,

    /// Additional layer paths (lowest → highest, before project layer).
    #[serde(default)]
    pub additional_layers: Vec<PathBuf>,

    /// Project-level switch: disable additional layers regardless of user config.
    #[serde(default)]
    pub ignore_additional_layers: bool,
}

impl Default for SourcesConfig {
    fn default() -> Self {
        Self {
            use_user_layer: true,
            ignore_user_layer: false,
            user_layer_path: None,
            additional_layers: Vec::new(),
            ignore_additional_layers: false,
        }
    }
}

pub fn default_user_layer_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".calvin/.promptpack"))
        .unwrap_or_else(|| PathBuf::from("~/.calvin/.promptpack"))
}

/// Deploy configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeployConfig {
    /// Default deploy target (project or home)
    #[serde(default)]
    pub target: DeployTarget,
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

impl Verbosity {
    /// All valid string representations for this enum
    pub const VALID_VALUES: &'static [&'static str] = &["quiet", "normal", "verbose", "debug"];

    /// Parse a string into Verbosity (case-insensitive)
    /// Returns None for invalid values
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "quiet" => Some(Self::Quiet),
            "normal" => Some(Self::Normal),
            "verbose" => Some(Self::Verbose),
            "debug" => Some(Self::Debug),
            _ => None,
        }
    }
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

    #[serde(default)]
    pub sources: SourcesConfig,
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

    /// Get enabled targets based on configuration.
    ///
    /// Semantics:
    /// - `None` (field missing): All targets (default behavior)
    /// - `Some([])` (empty list): No targets (explicitly disabled)
    /// - `Some([...])` (list with items): Only specified targets
    pub fn enabled_targets(&self) -> Vec<Target> {
        match &self.targets.enabled {
            None => Target::ALL_CONCRETE.to_vec(),
            Some(targets) => targets.clone(),
        }
    }

    /// Save deploy target to config file
    ///
    /// Updates or creates the [deploy] section in config.toml with the target.
    /// Preserves other settings in the file.
    pub fn save_deploy_target(config_path: &Path, target: DeployTarget) -> CalvinResult<()> {
        loader::save_deploy_target(config_path, target)
    }
}

/// Implementation of DomainConfig for Config
///
/// This allows the domain layer to work with Config through the DomainConfig trait
/// without directly depending on the Config type.
impl DomainConfig for Config {
    fn security_mode(&self) -> SecurityMode {
        self.security.mode
    }

    fn enabled_targets(&self) -> Vec<Target> {
        self.enabled_targets()
    }

    fn deploy_target(&self) -> DeployTarget {
        self.deploy.target
    }

    fn format_version(&self) -> &str {
        &self.format.version
    }

    fn atomic_writes(&self) -> bool {
        self.sync.atomic_writes
    }

    fn respect_lockfile(&self) -> bool {
        self.sync.respect_lockfile
    }
}
