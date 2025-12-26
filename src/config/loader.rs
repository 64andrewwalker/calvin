//! Configuration loading and persistence

use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::value_objects::{ConfigWarning, DeployTarget, Target};
use crate::error::CalvinResult;

use super::env_validator::EnvVarValidator;
use super::types::{Config, SecurityMode, Verbosity};

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
    match load_or_default_with_warnings(project_root) {
        Ok((config, warnings)) => {
            // Print warnings if any
            for warning in warnings {
                eprintln!(
                    "Warning: Unknown config key '{}' in {}{}",
                    warning.key,
                    warning.file.display(),
                    warning
                        .suggestion
                        .as_ref()
                        .map(|s| format!(". Did you mean '{}'?", s))
                        .unwrap_or_default()
                );
            }
            config
        }
        Err(e) => {
            // Print warning and fall back to defaults
            eprintln!("Warning: Failed to load config: {}", e);
            eprintln!("Using default configuration");
            with_env_overrides(Config::default())
        }
    }
}

/// Load from project config, user config, or defaults with detailed error/warning info
///
/// Returns Ok((config, warnings)) on success, Err on parse failure.
/// This allows callers to handle errors programmatically rather than just printing.
pub fn load_or_default_with_warnings(
    project_root: Option<&Path>,
) -> CalvinResult<(Config, Vec<ConfigWarning>)> {
    // Prefer XDG config (`~/.config/calvin/config.toml`), but support legacy
    // `~/.calvin/config.toml` as an alternative (PRD note).
    let xdg_user_config_path = dirs_config_dir().map(|d| d.join("calvin/config.toml"));

    // Allow override for testing (especially on Windows where dirs::home_dir
    // uses system API and cannot be overridden via environment variables).
    // Fallback to legacy path if the override is not set.
    let legacy_user_config_path = std::env::var("CALVIN_USER_CONFIG_PATH")
        .ok()
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".calvin/config.toml")));

    let user_config_path = xdg_user_config_path
        .as_ref()
        .filter(|p| p.exists())
        .cloned()
        .or_else(|| {
            legacy_user_config_path
                .as_ref()
                .filter(|p| p.exists())
                .cloned()
        });
    let project_config_path = project_root.map(|r| r.join(".promptpack/config.toml"));

    let mut warnings: Vec<ConfigWarning> = Vec::new();

    let mut merged: toml::Value = toml::Value::Table(toml::map::Map::new());

    // User config (lower priority than project config)
    if let Some(user_config) = user_config_path.as_ref().filter(|p| p.exists()) {
        let content = fs::read_to_string(user_config)?;
        let user_value: toml::Value = toml::from_str(&content).map_err(|e| {
            crate::error::CalvinError::InvalidFrontmatter {
                file: user_config.to_path_buf(),
                message: e.to_string(),
            }
        })?;

        let (_parsed, w) = load_with_warnings(user_config)?;
        warnings.extend(w);

        merge_toml(&mut merged, user_value);
    }

    // Project config (highest priority config file)
    if let Some(project_config) = project_config_path.as_ref().filter(|p| p.exists()) {
        let content = fs::read_to_string(project_config)?;
        let project_value: toml::Value = toml::from_str(&content).map_err(|e| {
            crate::error::CalvinError::InvalidFrontmatter {
                file: project_config.to_path_buf(),
                message: e.to_string(),
            }
        })?;

        validate_project_sources_config(project_config, &project_value)?;

        let (_parsed, w) = load_with_warnings(project_config)?;
        warnings.extend(w);

        merge_toml(&mut merged, project_value);
    }

    // Deserialize merged config. If no configs present, this yields defaults.
    let config: Config = match merged {
        toml::Value::Table(ref t) if t.is_empty() => Config::default(),
        _ => merged
            .try_into()
            .map_err(|e| crate::error::CalvinError::InvalidFrontmatter {
                file: project_config_path
                    .and_then(|p| p.exists().then_some(p))
                    .or_else(|| user_config_path.and_then(|p| p.exists().then_some(p)))
                    .unwrap_or_else(|| PathBuf::from("config.toml")),
                message: e.to_string(),
            })?,
    };

    Ok((with_env_overrides(config), warnings))
}

/// Apply environment variable overrides (CALVIN_* prefix)
pub fn with_env_overrides(mut config: Config) -> Config {
    // CALVIN_SECURITY_MODE - with validation and helpful warnings
    if let Ok(mode) = std::env::var("CALVIN_SECURITY_MODE") {
        config.security.mode = EnvVarValidator::new(
            "CALVIN_SECURITY_MODE",
            SecurityMode::VALID_VALUES,
        )
        .parse(&mode, SecurityMode::parse_str, SecurityMode::Balanced);
    }

    // CALVIN_TARGETS (comma-separated) - with validation and helpful warnings
    if let Ok(targets) = std::env::var("CALVIN_TARGETS") {
        let mut parsed: Vec<Target> = Vec::new();
        let mut had_invalid = false;

        for s in targets.split(',') {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                continue;
            }
            match Target::from_str_with_suggestion(trimmed) {
                Ok(target) if target != Target::All => parsed.push(target),
                Ok(_) => {} // Ignore 'all' - it's meta
                Err(e) => {
                    eprintln!("Warning: {}", e);
                    had_invalid = true;
                }
            }
        }

        // Only apply if we got some valid targets, or if there were no invalid ones
        if !parsed.is_empty() || !had_invalid {
            config.targets.enabled = Some(parsed);
        }
    }

    // CALVIN_VERBOSITY - with validation and helpful warnings
    if let Ok(verbosity) = std::env::var("CALVIN_VERBOSITY") {
        config.output.verbosity = EnvVarValidator::new("CALVIN_VERBOSITY", Verbosity::VALID_VALUES)
            .parse(&verbosity, Verbosity::parse_str, Verbosity::Normal);
    }

    // CALVIN_ATOMIC_WRITES
    if let Ok(val) = std::env::var("CALVIN_ATOMIC_WRITES") {
        config.sync.atomic_writes = val.to_lowercase() != "false" && val != "0";
    }

    // CALVIN_SOURCES_USE_USER_LAYER
    if let Ok(val) = std::env::var("CALVIN_SOURCES_USE_USER_LAYER") {
        config.sources.use_user_layer = val.to_lowercase() != "false" && val != "0";
    }

    // CALVIN_SOURCES_USER_LAYER_PATH
    if let Ok(val) = std::env::var("CALVIN_SOURCES_USER_LAYER_PATH") {
        config.sources.user_layer_path = Some(PathBuf::from(val));
    }

    config
}

/// Save deploy target to config file
pub fn save_deploy_target(config_path: &Path, target: DeployTarget) -> CalvinResult<()> {
    use std::io::Write;

    // Don't save Unset
    if target == DeployTarget::Unset {
        return Ok(());
    }

    let target_str = match target {
        DeployTarget::Home => "home",
        DeployTarget::Project => "project",
        DeployTarget::Unset => return Ok(()),
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

fn validate_project_sources_config(path: &Path, value: &toml::Value) -> CalvinResult<()> {
    let Some(sources) = value.get("sources") else {
        return Ok(());
    };
    let Some(table) = sources.as_table() else {
        return Err(crate::error::CalvinError::ConfigSecurityViolation {
            file: path.to_path_buf(),
            message: "[sources] must be a table".to_string(),
        });
    };

    const ALLOWED_KEYS: &[&str] = &["ignore_user_layer", "ignore_additional_layers"];
    for key in table.keys() {
        if !ALLOWED_KEYS.contains(&key.as_str()) {
            return Err(crate::error::CalvinError::ConfigSecurityViolation {
                file: path.to_path_buf(),
                message: format!("project config cannot set sources.{key}"),
            });
        }
    }

    Ok(())
}

fn merge_toml_deep(base: &mut toml::Value, overlay: toml::Value) {
    match (base, overlay) {
        (toml::Value::Table(base_table), toml::Value::Table(overlay_table)) => {
            for (k, v) in overlay_table {
                match base_table.get_mut(&k) {
                    Some(existing) => merge_toml_deep(existing, v),
                    None => {
                        base_table.insert(k, v);
                    }
                }
            }
        }
        (base_slot, overlay_value) => {
            *base_slot = overlay_value;
        }
    }
}

/// Merge config TOML values following PRD §11.2 (section-level overrides).
///
/// Semantics:
/// - Top-level sections are treated as atomic: the overlay replaces the base section entirely.
/// - Exception: `[sources]` is merged deeply so project ignore flags can coexist with user paths.
fn merge_toml(base: &mut toml::Value, overlay: toml::Value) {
    match (base, overlay) {
        (toml::Value::Table(base_table), toml::Value::Table(overlay_table)) => {
            for (k, v) in overlay_table {
                if k == "sources" {
                    match base_table.get_mut(&k) {
                        Some(existing) => merge_toml_deep(existing, v),
                        None => {
                            base_table.insert(k, v);
                        }
                    }
                } else {
                    // Section-level override (no deep merge).
                    base_table.insert(k, v);
                }
            }
        }
        (base_slot, overlay_value) => {
            *base_slot = overlay_value;
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_merge_section_override_drops_missing_keys() {
        let user_toml = r#"
[security]
mode = "balanced"
allow_naked = true
"#;

        let project_toml = r#"
[security]
mode = "strict"
"#;

        let mut merged: toml::Value = toml::Value::Table(toml::map::Map::new());
        let user_value: toml::Value = toml::from_str(user_toml).unwrap();
        let project_value: toml::Value = toml::from_str(project_toml).unwrap();

        merge_toml(&mut merged, user_value);
        merge_toml(&mut merged, project_value);

        let config: Config = merged.try_into().unwrap();

        assert_eq!(config.security.mode, SecurityMode::Strict);
        assert!(
            !config.security.allow_naked,
            "PRD §11.2: higher-level section should fully override lower-level (no deep merge)"
        );
    }

    #[test]
    fn config_merge_sources_preserves_user_settings_when_project_sets_ignore_flags() {
        let user_toml = r#"
[sources]
user_layer_path = "~/.calvin/.promptpack"
additional_layers = ["~/team/.promptpack"]
"#;

        let project_toml = r#"
[sources]
ignore_user_layer = true
"#;

        let mut merged: toml::Value = toml::Value::Table(toml::map::Map::new());
        let user_value: toml::Value = toml::from_str(user_toml).unwrap();
        let project_value: toml::Value = toml::from_str(project_toml).unwrap();

        merge_toml(&mut merged, user_value);
        merge_toml(&mut merged, project_value);

        let config: Config = merged.try_into().unwrap();

        assert!(config.sources.ignore_user_layer);
        assert_eq!(
            config.sources.additional_layers.len(),
            1,
            "project ignore flags should not wipe user-configured additional layers"
        );
        assert_eq!(
            config.sources.user_layer_path.as_deref(),
            Some(Path::new("~/.calvin/.promptpack")),
            "project ignore flags should not wipe user-configured user_layer_path"
        );
    }
}
