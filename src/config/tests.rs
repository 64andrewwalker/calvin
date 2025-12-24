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
    assert_eq!(config.enabled_targets().len(), 2);
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
    config.targets.enabled = Some(vec![Target::ClaudeCode, Target::Cursor]);

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
    let targets = config.enabled_targets();
    assert_eq!(targets.len(), 2);
    assert!(targets.contains(&Target::ClaudeCode));
    assert!(targets.contains(&Target::Cursor));
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

// === TDD: Fix targets-config-bug-2025-12-24 ===

/// Test that empty list `enabled = []` means "no targets"
/// This is different from field missing (which means "all targets")
#[test]
fn test_enabled_targets_empty_list_means_none() {
    let toml = r#"
[targets]
enabled = []
"#;

    let config: Config = toml::from_str(toml).unwrap();
    let targets = config.enabled_targets();
    // Empty list should mean "no targets", not "all targets"
    assert!(
        targets.is_empty(),
        "enabled = [] should return empty vec, not all targets"
    );
}

/// Test that missing `enabled` field means "all targets" (default behavior)
#[test]
fn test_enabled_targets_field_missing_means_all() {
    let toml = r#"
[targets]
# enabled field is missing
"#;

    let config: Config = toml::from_str(toml).unwrap();
    let targets = config.enabled_targets();
    // Missing field should mean "all targets"
    assert_eq!(
        targets.len(),
        5,
        "missing enabled field should return all 5 targets"
    );
}

/// Test that missing [targets] section means "all targets"
#[test]
fn test_enabled_targets_section_missing_means_all() {
    let toml = r#"
[format]
version = "1.0"
"#;

    let config: Config = toml::from_str(toml).unwrap();
    let targets = config.enabled_targets();
    assert_eq!(
        targets.len(),
        5,
        "missing [targets] section should return all 5 targets"
    );
}

/// Test that explicit list `enabled = ["claude-code"]` returns only specified targets
#[test]
fn test_enabled_targets_explicit_list() {
    let toml = r#"
[targets]
enabled = ["claude-code", "cursor"]
"#;

    let config: Config = toml::from_str(toml).unwrap();
    let targets = config.enabled_targets();
    assert_eq!(targets.len(), 2);
    assert!(targets.contains(&Target::ClaudeCode));
    assert!(targets.contains(&Target::Cursor));
}

/// Test that invalid target name returns a helpful error
#[test]
fn test_invalid_target_name_error() {
    let toml = r#"
[targets]
enabled = ["invalid-target"]
"#;

    let result: Result<Config, _> = toml::from_str(toml);
    assert!(
        result.is_err(),
        "invalid target 'invalid-target' should fail"
    );
    let err = result.unwrap_err().to_string();
    // Should contain the invalid value for debugging
    assert!(
        err.contains("invalid-target") || err.contains("unknown variant"),
        "error should mention the invalid value: {}",
        err
    );
}

/// Test that 'claude' is a valid alias for 'claude-code'
#[test]
fn test_claude_alias_valid() {
    let toml = r#"
[targets]
enabled = ["claude"]
"#;

    let config: Config = toml::from_str(toml).unwrap();
    let targets = config.enabled_targets();
    assert_eq!(targets.len(), 1);
    assert!(
        targets.contains(&Target::ClaudeCode),
        "claude should map to ClaudeCode"
    );
}

/// Test that load_or_default returns Result with warnings for invalid config
#[test]
fn test_load_or_default_with_invalid_config_returns_warning() {
    let dir = tempdir().unwrap();
    let promptpack_dir = dir.path().join(".promptpack");
    fs::create_dir_all(&promptpack_dir).unwrap();

    // Write invalid config with invalid target
    let config_path = promptpack_dir.join("config.toml");
    fs::write(
        &config_path,
        r#"
[targets]
enabled = ["invalid-target"]
"#,
    )
    .unwrap();

    // load_or_default_with_warnings should return the error
    let result = crate::config::loader::load_or_default_with_warnings(Some(dir.path()));

    // Should return an error or warning about invalid config
    assert!(
        result.is_err() || !result.as_ref().unwrap().1.is_empty(),
        "should report error or warning for invalid config"
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

// === TDD: Env Var Validation Refactor - Verbosity ===

#[test]
fn test_verbosity_valid_values_contains_all_variants() {
    assert!(Verbosity::VALID_VALUES.contains(&"quiet"));
    assert!(Verbosity::VALID_VALUES.contains(&"normal"));
    assert!(Verbosity::VALID_VALUES.contains(&"verbose"));
    assert!(Verbosity::VALID_VALUES.contains(&"debug"));
    assert_eq!(Verbosity::VALID_VALUES.len(), 4);
}

#[test]
fn test_verbosity_from_str_valid_values() {
    assert_eq!(Verbosity::parse_str("quiet"), Some(Verbosity::Quiet));
    assert_eq!(Verbosity::parse_str("normal"), Some(Verbosity::Normal));
    assert_eq!(Verbosity::parse_str("verbose"), Some(Verbosity::Verbose));
    assert_eq!(Verbosity::parse_str("debug"), Some(Verbosity::Debug));
    // Case insensitive
    assert_eq!(Verbosity::parse_str("QUIET"), Some(Verbosity::Quiet));
    assert_eq!(Verbosity::parse_str("Debug"), Some(Verbosity::Debug));
}

#[test]
fn test_verbosity_from_str_invalid_values() {
    assert_eq!(Verbosity::parse_str("invalid"), None);
    assert_eq!(Verbosity::parse_str(""), None);
    assert_eq!(Verbosity::parse_str("quite"), None); // typo
}
