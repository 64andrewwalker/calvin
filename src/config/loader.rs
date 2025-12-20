//! Configuration loading and persistence

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::CalvinResult;
use crate::models::Target;

use super::types::{Config, DeployTargetConfig, SecurityMode, Verbosity};

/// Non-fatal configuration warning surfaced to CLI users.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigWarning {
    pub key: String,
    pub file: PathBuf,
    pub line: Option<usize>,
    pub suggestion: Option<String>,
}

/// Load configuration and collect non-fatal warnings (e.g. unknown keys).
pub fn load_with_warnings(path: &Path) -> CalvinResult<(Config, Vec<ConfigWarning>)> {
    let content = fs::read_to_string(path)?;

    let mut unknown_paths: Vec<String> = Vec::new();
    let deserializer = toml::de::Deserializer::new(&content);

    let config: Config = serde_ignored::deserialize(deserializer, |p| {
        unknown_paths.push(p.to_string());
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
                .next_back()
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
pub fn load_or_default(project_root: Option<&Path>) -> Config {
    // Try project config first
    if let Some(root) = project_root {
        let project_config = root.join(".promptpack/config.toml");
        if project_config.exists() {
            if let Ok(config) = Config::load(&project_config) {
                return with_env_overrides(config);
            }
        }
    }

    // Try user config
    if let Some(user_config_dir) = dirs_config_dir() {
        let user_config = user_config_dir.join("calvin/config.toml");
        if user_config.exists() {
            if let Ok(config) = Config::load(&user_config) {
                return with_env_overrides(config);
            }
        }
    }

    // Return defaults with env overrides
    with_env_overrides(Config::default())
}

/// Apply environment variable overrides (CALVIN_* prefix)
pub fn with_env_overrides(mut config: Config) -> Config {
    // CALVIN_SECURITY_MODE
    if let Ok(mode) = std::env::var("CALVIN_SECURITY_MODE") {
        config.security.mode = match mode.to_lowercase().as_str() {
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
            config.targets.enabled = parsed;
        }
    }

    // CALVIN_VERBOSITY
    if let Ok(verbosity) = std::env::var("CALVIN_VERBOSITY") {
        config.output.verbosity = match verbosity.to_lowercase().as_str() {
            "quiet" => Verbosity::Quiet,
            "verbose" => Verbosity::Verbose,
            "debug" => Verbosity::Debug,
            _ => Verbosity::Normal,
        };
    }

    // CALVIN_ATOMIC_WRITES
    if let Ok(val) = std::env::var("CALVIN_ATOMIC_WRITES") {
        config.sync.atomic_writes = val.to_lowercase() != "false" && val != "0";
    }

    config
}

/// Save deploy target to config file
pub fn save_deploy_target(config_path: &Path, target: DeployTargetConfig) -> CalvinResult<()> {
    use std::io::Write;

    // Don't save Unset
    if target == DeployTargetConfig::Unset {
        return Ok(());
    }

    let target_str = match target {
        DeployTargetConfig::Home => "home",
        DeployTargetConfig::Project => "project",
        DeployTargetConfig::Unset => return Ok(()),
    };

    // Read existing config or start fresh
    let existing = fs::read_to_string(config_path).unwrap_or_default();

    // Check if [deploy] section exists
    if existing.contains("[deploy]") {
        // Update existing deploy.target
        let mut lines: Vec<String> = existing.lines().map(|s| s.to_string()).collect();
        let mut in_deploy_section = false;
        let mut target_updated = false;

        for line in &mut lines {
            if line.trim() == "[deploy]" {
                in_deploy_section = true;
            } else if line.starts_with('[') && in_deploy_section {
                // New section started, insert target before this
                if !target_updated {
                    // This shouldn't happen if target exists, but handle it
                }
                in_deploy_section = false;
            } else if in_deploy_section && line.trim().starts_with("target") {
                *line = format!("target = \"{}\"", target_str);
                target_updated = true;
            }
        }

        // If [deploy] section exists but no target line, add it
        if !target_updated {
            for (i, line) in lines.iter().enumerate() {
                if line.trim() == "[deploy]" {
                    lines.insert(i + 1, format!("target = \"{}\"", target_str));
                    break;
                }
            }
        }

        let new_content = lines.join("\n");
        let mut file = fs::File::create(config_path)?;
        file.write_all(new_content.as_bytes())?;
    } else {
        // Append [deploy] section
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(config_path)?;

        // Add newline if file doesn't end with one
        if !existing.is_empty() && !existing.ends_with('\n') {
            writeln!(file)?;
        }
        writeln!(file)?;
        writeln!(file, "[deploy]")?;
        writeln!(file, "target = \"{}\"", target_str)?;
    }

    Ok(())
}

/// Get XDG config directory
fn dirs_config_dir() -> Option<PathBuf> {
    std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".config"))
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
        "color",
        "animation",
        "unicode",
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
            curr[j + 1] =
                std::cmp::min(std::cmp::min(prev[j + 1] + 1, curr[j] + 1), prev[j] + cost);
        }
        prev.clone_from_slice(&curr);
    }

    prev[b_bytes.len()]
}
