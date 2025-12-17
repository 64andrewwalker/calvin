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
use std::collections::HashMap;

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
}

/// Non-fatal configuration warning surfaced to CLI users.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigWarning {
    pub key: String,
    pub file: PathBuf,
    pub line: Option<usize>,
    pub suggestion: Option<String>,
}

impl Config {
    /// Load configuration from a TOML file
    pub fn load(path: &Path) -> CalvinResult<Self> {
        let (config, _warnings) = Self::load_with_warnings(path)?;
        Ok(config)
    }

    /// Load configuration and collect non-fatal warnings (e.g. unknown keys).
    pub fn load_with_warnings(path: &Path) -> CalvinResult<(Self, Vec<ConfigWarning>)> {
        let content = fs::read_to_string(path)?;

        let mut unknown_paths: Vec<String> = Vec::new();
        let deserializer = toml::de::Deserializer::new(&content);

        let config: Self = serde_ignored::deserialize(deserializer, |path| {
            unknown_paths.push(path.to_string());
        })
        .map_err(|e| crate::error::CalvinError::InvalidFrontmatter {
            file: path.to_path_buf(),
            message: e.to_string(),
        })?;

        let warnings = unknown_paths
            .into_iter()
            .map(|path_str| {
                let key = path_str
                    .split('.')
                    .last()
                    .unwrap_or(path_str.as_str())
                    .to_string();
                ConfigWarning {
                    key: key.clone(),
                    file: path.to_path_buf(),
                    line: find_line_number(&content, &key),
                    suggestion: suggest_key(&key),
                }
            })
            .collect();

        Ok((config, warnings))
    }
    
    /// Load from project config, user config, or defaults
    pub fn load_or_default(project_root: Option<&Path>) -> Self {
        // Try project config first
        if let Some(root) = project_root {
            let project_config = root.join(".promptpack/config.toml");
            if project_config.exists() {
                if let Ok(config) = Self::load(&project_config) {
                    return config.with_env_overrides();
                }
            }
        }
        
        // Try user config
        if let Some(user_config_dir) = dirs_config_dir() {
            let user_config = user_config_dir.join("calvin/config.toml");
            if user_config.exists() {
                if let Ok(config) = Self::load(&user_config) {
                    return config.with_env_overrides();
                }
            }
        }
        
        // Return defaults with env overrides
        Self::default().with_env_overrides()
    }
    
    /// Apply environment variable overrides (CALVIN_* prefix)
    pub fn with_env_overrides(mut self) -> Self {
        // CALVIN_SECURITY_MODE
        if let Ok(mode) = std::env::var("CALVIN_SECURITY_MODE") {
            self.security.mode = match mode.to_lowercase().as_str() {
                "yolo" => SecurityMode::Yolo,
                "strict" => SecurityMode::Strict,
                _ => SecurityMode::Balanced,
            };
        }

        // CALVIN_TARGETS (comma-separated)
        if let Ok(targets) = std::env::var("CALVIN_TARGETS") {
            let parsed: Vec<Target> = targets
                .split(',')
                .filter_map(|s| match s.trim().to_lowercase().as_str() {
                    "claude-code" | "claudecode" => Some(Target::ClaudeCode),
                    "cursor" => Some(Target::Cursor),
                    "vscode" | "vs-code" => Some(Target::VSCode),
                    "antigravity" => Some(Target::Antigravity),
                    "codex" => Some(Target::Codex),
                    _ => None,
                })
                .collect();
            if !parsed.is_empty() {
                self.targets.enabled = parsed;
            }
        }

        // CALVIN_VERBOSITY
        if let Ok(verbosity) = std::env::var("CALVIN_VERBOSITY") {
            self.output.verbosity = match verbosity.to_lowercase().as_str() {
                "quiet" => Verbosity::Quiet,
                "verbose" => Verbosity::Verbose,
                "debug" => Verbosity::Debug,
                _ => Verbosity::Normal,
            };
        }

        // CALVIN_ATOMIC_WRITES
        if let Ok(val) = std::env::var("CALVIN_ATOMIC_WRITES") {
            self.sync.atomic_writes = val.to_lowercase() != "false" && val != "0";
        }

        self
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

fn find_line_number(content: &str, needle: &str) -> Option<usize> {
    for (i, line) in content.lines().enumerate() {
        if line.contains(needle) {
            return Some(i + 1);
        }
    }
    None
}

fn suggest_key(unknown: &str) -> Option<String> {
    const CANDIDATES: &[&str] = &[
        "format",
        "version",
        "security",
        "mode",
        "allow_naked",
        "deny",
        "patterns",
        "exclude",
        "mcp",
        "allowlist",
        "additional_allowlist",
        "targets",
        "enabled",
        "sync",
        "atomic_writes",
        "respect_lockfile",
        "output",
        "verbosity",
        "servers",
        "command",
        "args",
    ];

    let mut best: Option<(&str, usize)> = None;
    for candidate in CANDIDATES {
        let dist = levenshtein(unknown, candidate);
        best = match best {
            None => Some((candidate, dist)),
            Some((_, best_dist)) if dist < best_dist => Some((candidate, dist)),
            Some(current) => Some(current),
        };
    }

    match best {
        Some((candidate, dist)) if dist <= 2 => Some(candidate.to_string()),
        _ => None,
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    if a == b {
        return 0;
    }

    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();

    let mut prev: Vec<usize> = (0..=b_bytes.len()).collect();
    let mut curr = vec![0usize; b_bytes.len() + 1];

    for (i, &ac) in a_bytes.iter().enumerate() {
        curr[0] = i + 1;
        for (j, &bc) in b_bytes.iter().enumerate() {
            let cost = if ac == bc { 0 } else { 1 };
            curr[j + 1] = std::cmp::min(
                std::cmp::min(prev[j + 1] + 1, curr[j] + 1),
                prev[j] + cost,
            );
        }
        prev.clone_from_slice(&curr);
    }

    prev[b_bytes.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

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

    #[test]
    fn test_env_override_security_mode() {
        // SAFETY: Single-threaded test, no concurrent access to env vars
        unsafe { std::env::set_var("CALVIN_SECURITY_MODE", "strict") };
        let config = Config::default().with_env_overrides();
        assert_eq!(config.security.mode, SecurityMode::Strict);
        unsafe { std::env::remove_var("CALVIN_SECURITY_MODE") };
    }

    #[test]
    fn test_env_override_targets() {
        // SAFETY: Single-threaded test, no concurrent access to env vars
        unsafe { std::env::set_var("CALVIN_TARGETS", "claude-code,cursor") };
        let config = Config::default().with_env_overrides();
        assert_eq!(config.targets.enabled.len(), 2);
        assert!(config.targets.enabled.contains(&Target::ClaudeCode));
        assert!(config.targets.enabled.contains(&Target::Cursor));
        unsafe { std::env::remove_var("CALVIN_TARGETS") };
    }

    #[test]
    fn test_env_override_verbosity() {
        // SAFETY: Single-threaded test, no concurrent access to env vars
        unsafe { std::env::set_var("CALVIN_VERBOSITY", "debug") };
        let config = Config::default().with_env_overrides();
        assert_eq!(config.output.verbosity, Verbosity::Debug);
        unsafe { std::env::remove_var("CALVIN_VERBOSITY") };
    }

    #[test]
    fn test_env_override_atomic_writes() {
        // SAFETY: Single-threaded test, no concurrent access to env vars
        unsafe { std::env::set_var("CALVIN_ATOMIC_WRITES", "false") };
        let config = Config::default().with_env_overrides();
        assert!(!config.sync.atomic_writes);
        unsafe { std::env::remove_var("CALVIN_ATOMIC_WRITES") };
    }

    // === TDD: US-1 Configurable deny list (Sprint 1 / P0) ===

    #[test]
    fn test_config_parse_security_deny_table_with_exclude() {
        let toml = r#"
[security]
mode = "balanced"
allow_naked = false

[security.deny]
patterns = ["secrets/**"]
exclude = [".env.example"]
"#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.security.mode, SecurityMode::Balanced);
        assert!(!config.security.allow_naked);
        assert_eq!(config.security.deny.patterns, vec!["secrets/**".to_string()]);
        assert_eq!(config.security.deny.exclude, vec![".env.example".to_string()]);
    }

    #[test]
    fn test_config_parse_security_deny_array_compat() {
        let toml = r#"
[security]
deny = ["*.secret"]
"#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.security.deny.patterns, vec!["*.secret".to_string()]);
        assert!(config.security.deny.exclude.is_empty());
    }

    // === TDD: US-2 MCP allowlist config (Sprint 2 / P1) ===

    #[test]
    fn test_config_parse_security_mcp_additional_allowlist() {
        let toml = r#"
[security.mcp]
additional_allowlist = ["internal-code-server"]
"#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(
            config.security.mcp.additional_allowlist,
            vec!["internal-code-server".to_string()]
        );
    }

    // === TDD: US-9 Config unknown key warnings (Sprint 2 / P1) ===

    #[test]
    fn test_config_load_with_warnings_reports_unknown_key_with_suggestion() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");

        fs::write(&path, "securty = 1\n").unwrap();

        let (_config, warnings) = Config::load_with_warnings(&path).unwrap();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].key, "securty");
        assert_eq!(warnings[0].line, Some(1));
        assert_eq!(warnings[0].suggestion, Some("security".to_string()));
    }
}
