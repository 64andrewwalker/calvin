//! Configuration module for Calvin
//!
//! Implements configuration hierarchy per TD-7:
//! 1. CLI flags (highest priority)
//! 2. Environment variables (CALVIN_*)
//! 3. Project config (.promptpack/config.toml)
//! 4. User config (~/.config/calvin/config.toml)
//! 5. Built-in defaults (lowest priority)

use std::path::{Path, PathBuf};
use std::fs;

use serde::{Deserialize, Serialize};

use crate::error::CalvinResult;
use crate::models::Target;

/// Security mode for Calvin operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SecurityMode {
    /// No enforcement, INFO logs only
    Yolo,
    /// Generate protections, WARN on issues (default)
    #[default]
    Balanced,
    /// Block on security violations
    Strict,
}

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
    pub deny: Vec<String>,
    
    #[serde(default)]
    pub allow_naked: bool,
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

/// Output configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutputConfig {
    #[serde(default)]
    pub verbosity: Verbosity,
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
}

impl Config {
    /// Load configuration from a TOML file
    pub fn load(path: &Path) -> CalvinResult<Self> {
        let content = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)
            .map_err(|e| crate::error::CalvinError::InvalidFrontmatter {
                file: path.to_path_buf(),
                message: e.to_string(),
            })?;
        Ok(config)
    }
    
    /// Load from project config, user config, or defaults
    pub fn load_or_default(project_root: Option<&Path>) -> Self {
        // Try project config first
        if let Some(root) = project_root {
            let project_config = root.join(".promptpack/config.toml");
            if project_config.exists() {
                if let Ok(config) = Self::load(&project_config) {
                    return config;
                }
            }
        }
        
        // Try user config
        if let Some(user_config_dir) = dirs_config_dir() {
            let user_config = user_config_dir.join("calvin/config.toml");
            if user_config.exists() {
                if let Ok(config) = Self::load(&user_config) {
                    return config;
                }
            }
        }
        
        // Return defaults
        Self::default()
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
}

/// Get XDG config directory
fn dirs_config_dir() -> Option<PathBuf> {
    std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        
        assert_eq!(config.format.version, "1.0");
        assert_eq!(config.security.mode, SecurityMode::Balanced);
        assert!(config.sync.atomic_writes);
        assert!(config.sync.respect_lockfile);
    }

    #[test]
    fn test_security_mode_serde() {
        let yaml = "yolo";
        let mode: SecurityMode = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(mode, SecurityMode::Yolo);

        let yaml = "balanced";
        let mode: SecurityMode = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(mode, SecurityMode::Balanced);

        let yaml = "strict";
        let mode: SecurityMode = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(mode, SecurityMode::Strict);
    }

    #[test]
    fn test_config_parse_toml() {
        let toml = r#"
[format]
version = "1.0"

[security]
mode = "balanced"

[targets]
enabled = ["claude-code", "cursor"]

[sync]
atomic_writes = true
respect_lockfile = true

[output]
verbosity = "normal"
"#;

        let config: Config = toml::from_str(toml).unwrap();
        
        assert_eq!(config.format.version, "1.0");
        assert_eq!(config.security.mode, SecurityMode::Balanced);
        assert_eq!(config.targets.enabled.len(), 2);
        assert!(config.sync.atomic_writes);
    }

    #[test]
    fn test_enabled_targets_default() {
        let config = Config::default();
        let targets = config.enabled_targets();
        
        assert_eq!(targets.len(), 5);
    }

    #[test]
    fn test_enabled_targets_filtered() {
        let mut config = Config::default();
        config.targets.enabled = vec![Target::ClaudeCode, Target::Cursor];
        
        let targets = config.enabled_targets();
        assert_eq!(targets.len(), 2);
    }

    #[test]
    fn test_verbosity_serde() {
        let yaml = "quiet";
        let v: Verbosity = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(v, Verbosity::Quiet);

        let yaml = "verbose";
        let v: Verbosity = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(v, Verbosity::Verbose);
    }
}
