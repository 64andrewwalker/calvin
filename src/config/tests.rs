//! Tests for the config module

use super::types::*;
use crate::models::Target;
use std::fs;
use tempfile::tempdir;

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
    let mode: SecurityMode = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(mode, SecurityMode::Yolo);

    let yaml = "balanced";
    let mode: SecurityMode = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(mode, SecurityMode::Balanced);

    let yaml = "strict";
    let mode: SecurityMode = serde_yaml_ng::from_str(yaml).unwrap();
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
    let v: Verbosity = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(v, Verbosity::Quiet);

    let yaml = "verbose";
    let v: Verbosity = serde_yaml_ng::from_str(yaml).unwrap();
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

// === TDD: CLI animation output config (v0.3.0 / Phase 0) ===

#[test]
fn test_output_config_defaults() {
    let config = Config::default();
    assert_eq!(config.output.color, ColorMode::Auto);
    assert_eq!(config.output.animation, AnimationMode::Auto);
    assert!(config.output.unicode);
}

#[test]
fn test_config_parse_output_color_animation_unicode() {
    let toml = r#"
[output]
color = "never"
animation = "minimal"
unicode = false
"#;

    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.output.color, ColorMode::Never);
    assert_eq!(config.output.animation, AnimationMode::Minimal);
    assert!(!config.output.unicode);
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
    assert_eq!(
        config.security.deny.patterns,
        vec!["secrets/**".to_string()]
    );
    assert_eq!(
        config.security.deny.exclude,
        vec![".env.example".to_string()]
    );
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
