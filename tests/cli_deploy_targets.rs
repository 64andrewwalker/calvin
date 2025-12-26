//! Integration tests for deploy target selection logic
//!
//! Tests the target selection priority:
//! 1. remote - if specified, deploy to remote
//! 2. home - if true, deploy to home directory
//! 3. explicit_project - if true, deploy to project (overrides config)
//! 4. config.deploy.target - fall back to configuration
//! 5. default - project

mod common;

use std::fs;

use common::*;

fn env_without_project_config() -> TestEnv {
    TestEnv::builder()
        .without_project_config_file()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .build()
}

fn env_with_project_config(toml: &str) -> TestEnv {
    TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .with_project_config(toml)
        .build()
}

// ============================================================================
// Target Selection Priority Tests
// ============================================================================

#[test]
fn deploy_defaults_to_project_without_config() {
    let env = env_without_project_config();
    let result = env.run(&["deploy", "--dry-run"]);

    assert!(
        result.success,
        "Deploy failed:\n{}",
        result.combined_output()
    );
    // Target should show current project path, not ~/
    assert!(
        !result.stdout.contains("Target: ~/"),
        "Expected project target, got:\n{}",
        result.stdout
    );
}

#[test]
fn deploy_uses_home_from_config() {
    let env = env_with_project_config(CONFIG_DEPLOY_HOME);
    let result = env.run(&["deploy", "--dry-run"]);

    assert!(
        result.success,
        "Deploy failed:\n{}",
        result.combined_output()
    );
    // Target should be home
    assert!(
        result.stdout.contains("Target: ~/"),
        "Expected home target in output:\n{}",
        result.stdout
    );
}

#[test]
fn deploy_home_flag_overrides_config() {
    let env = env_with_project_config(CONFIG_DEPLOY_PROJECT);
    let result = env.run(&["deploy", "--home", "--dry-run"]);

    assert!(
        result.success,
        "Deploy failed:\n{}",
        result.combined_output()
    );
    // --home should override project config
    assert!(
        result.stdout.contains("Target: ~/"),
        "Expected --home to override config:\n{}",
        result.stdout
    );
}

#[test]
fn deploy_home_and_remote_conflict() {
    let env = env_without_project_config();
    let result = env.run(&["deploy", "--home", "--remote", "user@host:/path"]);

    assert!(
        !result.success,
        "Expected deploy to fail due to conflicting flags.\nOutput:\n{}",
        result.combined_output()
    );
    // clap emits "cannot be used with" when args conflict
    assert!(
        result.stderr.contains("cannot be used")
            || result.stderr.contains("conflict")
            || result.stdout.contains("cannot be used")
            || result.stdout.contains("conflict"),
        "Expected conflict error.\nOutput:\n{}",
        result.combined_output()
    );
}

#[test]
fn deploy_remote_takes_precedence() {
    let env = env_with_project_config(CONFIG_DEPLOY_HOME);
    // Even with home in config, --remote should take precedence
    let result = env.run(&[
        "deploy",
        "--remote",
        "user@host:/path",
        "--dry-run",
        "--targets",
        "cursor",
        "--yes",
    ]);

    // Should mention remote in output somewhere
    assert!(
        result.stdout.contains("user@host")
            || result.stderr.contains("user@host")
            || result.stderr.contains("rsync"),
        "Expected remote target handling.\nOutput:\n{}",
        result.combined_output()
    );
}

// ============================================================================
// Mode Flags Tests
// ============================================================================

#[test]
fn deploy_dry_run_shows_would_write() {
    let env = env_without_project_config();
    let result = env.run(&["deploy", "--dry-run"]);

    assert!(
        result.success,
        "Deploy dry-run should succeed:\n{}",
        result.combined_output()
    );
}

#[test]
fn deploy_force_mode_accepted() {
    let env = env_without_project_config();
    let result = env.run(&["deploy", "--force", "--dry-run"]);

    assert!(
        result.success,
        "Deploy should succeed:\n{}",
        result.combined_output()
    );
    assert!(
        result.stdout.contains("Force") || result.stdout.contains("force"),
        "Expected force mode in output:\n{}",
        result.stdout
    );
}

#[test]
fn deploy_yes_skips_prompts() {
    let env = env_without_project_config();
    // --yes should skip any interactive prompts
    let result = env.run(&["deploy", "--yes"]);

    assert!(
        result.success,
        "Deploy should succeed:\n{}",
        result.combined_output()
    );
}

// ============================================================================
// Scope Policy Tests
// ============================================================================

#[test]
fn deploy_home_uses_force_user_scope() {
    let env = env_without_project_config();
    // When deploying to home, scope policy should be ForceUser
    // This is hard to test directly, but we can verify home deployment works
    let result = env.run(&["deploy", "--home", "--dry-run"]);

    assert!(
        result.success,
        "Deploy should succeed:\n{}",
        result.combined_output()
    );
}

// ============================================================================
// Config Saving Tests
// ============================================================================

#[test]
fn deploy_saves_target_to_config_on_success() {
    let env = env_without_project_config();

    // Deploy to project (default)
    let result = env.run(&["deploy", "--yes"]);
    assert!(
        result.success,
        "Deploy should succeed:\n{}",
        result.combined_output()
    );

    // Check that config was created/updated
    let config_path = env.project_path(".promptpack/config.toml");
    assert!(
        config_path.exists(),
        "Expected config.toml to be created at {:?}",
        config_path
    );
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(
        content.contains("[deploy]") && content.contains("target = \"project\""),
        "Expected deploy target to be saved in config.\nconfig.toml:\n{}",
        content
    );
}

#[test]
fn deploy_dry_run_does_not_save_config() {
    let env = env_without_project_config();

    // Note: we intentionally start with no config.toml
    let config_path = env.project_path(".promptpack/config.toml");
    assert!(
        !config_path.exists(),
        "Precondition: config.toml should not exist, but found {:?}",
        config_path
    );

    // Deploy with dry-run
    let result = env.run(&["deploy", "--dry-run"]);
    assert!(
        result.success,
        "Deploy dry-run should succeed:\n{}",
        result.combined_output()
    );

    // Config should NOT be created by dry-run
    assert!(
        !config_path.exists(),
        "Dry-run should not create config.toml, but found {:?}",
        config_path
    );
}

// ============================================================================
// Target Filter Tests
// ============================================================================

#[test]
fn deploy_respects_targets_flag() {
    let env = env_without_project_config();
    let result = env.run(&["deploy", "--targets", "cursor", "--dry-run"]);

    assert!(
        result.success,
        "Deploy should succeed:\n{}",
        result.combined_output()
    );
}

#[test]
fn deploy_multiple_targets() {
    let env = env_without_project_config();
    let result = env.run(&["deploy", "--targets", "cursor,claude-code", "--dry-run"]);

    assert!(
        result.success,
        "Deploy should succeed:\n{}",
        result.combined_output()
    );
}

// ============================================================================
// Cursor Commands Generation Tests (Bug Fix: cursor-commands-missing)
// ============================================================================

fn env_with_action_asset() -> TestEnv {
    TestEnv::builder()
        .with_project_asset("test-action.md", SIMPLE_ACTION)
        .build()
}

/// Bug fix test: Cursor-only deploy should generate .cursor/commands/
///
/// When deploying only to Cursor (without Claude Code), actions should
/// generate command files to .cursor/commands/ since Claude Code's
/// commands at ~/.claude/commands/ won't be available.
#[test]
fn deploy_cursor_only_generates_commands() {
    let env = env_with_action_asset();

    // Deploy with Cursor only (no Claude Code)
    let result = env.run(&["deploy", "--targets", "cursor", "--yes"]);

    assert!(
        result.success,
        "Deploy should succeed.\nOutput:\n{}",
        result.combined_output()
    );

    // Check that .cursor/commands/test-action.md was created
    let command_path = env.project_path(".cursor/commands/test-action.md");
    assert!(
        command_path.exists(),
        "Cursor-only deploy should create .cursor/commands/test-action.md. \
         Path checked: {:?}",
        command_path
    );

    // Verify the content
    let content = fs::read_to_string(&command_path).unwrap();
    assert!(
        content.contains("Test Action"),
        "Command file should contain the action content"
    );
    assert!(
        content.contains("Generated by Calvin"),
        "Command file should have Calvin footer"
    );
}

/// Bug fix test: Cursor + Claude Code should NOT generate .cursor/commands/
///
/// When both Cursor and Claude Code are targets, Cursor can read commands
/// from ~/.claude/commands/, so we should NOT create duplicate .cursor/commands/
#[test]
fn deploy_cursor_with_claude_code_no_duplicate_commands() {
    let env = env_with_action_asset();

    // Deploy with both Cursor and Claude Code
    let result = env.run(&["deploy", "--targets", "cursor,claude-code", "--yes"]);

    assert!(
        result.success,
        "Deploy should succeed.\nOutput:\n{}",
        result.combined_output()
    );

    // Check that .cursor/commands/ was NOT created (Claude Code provides commands)
    let cursor_commands_dir = env.project_path(".cursor/commands");
    assert!(
        !cursor_commands_dir.exists(),
        "Cursor should NOT create .cursor/commands when Claude Code is present. \
         Claude Code provides commands at .claude/commands/"
    );

    // But .claude/commands/ should exist
    let claude_command_path = env.project_path(".claude/commands/test-action.md");
    assert!(
        claude_command_path.exists(),
        "Claude Code should create .claude/commands/test-action.md"
    );
}

// ============================================================================
// Config Targets Enabled Tests (Bug Fix: targets-config-bug-2025-12-24)
// ============================================================================

fn env_with_cursor_only_config() -> TestEnv {
    env_with_project_config(CONFIG_CURSOR_ONLY)
}

fn env_with_empty_targets_config() -> TestEnv {
    env_with_project_config(CONFIG_EMPTY_TARGETS)
}

/// Bug fix test: enabled = ["cursor"] should only deploy to Cursor
///
/// Previously, config was ignored and all targets were deployed.
#[test]
fn deploy_respects_config_targets_cursor_only() {
    let env = env_with_cursor_only_config();
    let result = env.run(&["deploy", "--yes"]);

    assert!(
        result.success,
        "Deploy should succeed.\nOutput:\n{}",
        result.combined_output()
    );

    // .cursor should exist
    assert_deployed!(env, ".cursor");

    // .claude should NOT exist (not in enabled list)
    assert_not_deployed!(env, ".claude");
}

/// Bug fix test: enabled = [] should not deploy any targets
///
/// Empty list means "explicitly disable all targets", not "use defaults".
#[test]
fn deploy_respects_config_empty_targets() {
    let env = env_with_empty_targets_config();
    let result = env.run(&["deploy", "--yes"]);

    assert!(
        result.success,
        "Deploy should succeed (no targets = nothing to do).\nOutput:\n{}",
        result.combined_output()
    );

    // No target directories should be created
    assert_not_deployed!(env, ".cursor");
    assert_not_deployed!(env, ".claude");
    assert_not_deployed!(env, ".github");
}

fn env_with_all_targets_asset_and_cursor_only_config() -> TestEnv {
    TestEnv::builder()
        .with_project_asset("test.md", ALL_TARGETS_POLICY)
        .with_project_config(CONFIG_CURSOR_ONLY)
        .build()
}

/// Test: CLI --targets flag overrides config
#[test]
fn deploy_cli_targets_overrides_config() {
    // Use a project with an all-targets asset (no asset-level restriction)
    let env = env_with_all_targets_asset_and_cursor_only_config();
    // Config says cursor only, but CLI says claude-code
    // Use -t short form which is equivalent to --targets
    let result = env.run(&["deploy", "-t", "claude-code", "--yes"]);

    assert!(
        result.success,
        "Deploy should succeed.\nOutput:\n{}",
        result.combined_output()
    );

    // .claude should exist (CLI override)
    assert_deployed!(env, ".claude");
}

// ============================================================================
// Environment Variable Validation Tests
// ============================================================================

#[test]
fn deploy_warns_on_invalid_security_mode_env() {
    // Set invalid security mode with typo
    let env = env_without_project_config();
    let result = env.run_with_env(
        &["deploy", "--dry-run"],
        &[("CALVIN_SECURITY_MODE", "strct")],
    );
    assert!(
        result.success,
        "Deploy failed:\n{}",
        result.combined_output()
    );

    // Should contain warning about invalid value
    assert!(
        result.stderr.contains("Warning:") && result.stderr.contains("CALVIN_SECURITY_MODE"),
        "Should warn about invalid CALVIN_SECURITY_MODE.\nstderr:\n{}",
        result.stderr
    );

    // Should suggest the correct value
    assert!(
        result.stderr.contains("strict") || result.stderr.contains("Did you mean"),
        "Should suggest 'strict'. stderr:\n{}",
        result.stderr
    );
}

#[test]
fn deploy_warns_on_invalid_verbosity_env() {
    // Set invalid verbosity with typo
    let env = env_without_project_config();
    let result = env.run_with_env(&["deploy", "--dry-run"], &[("CALVIN_VERBOSITY", "quite")]);
    assert!(
        result.success,
        "Deploy failed:\n{}",
        result.combined_output()
    );

    // Should contain warning about invalid value
    assert!(
        result.stderr.contains("Warning:") && result.stderr.contains("CALVIN_VERBOSITY"),
        "Should warn about invalid CALVIN_VERBOSITY.\nstderr:\n{}",
        result.stderr
    );

    // Should list valid values
    assert!(
        result.stderr.contains("quiet") || result.stderr.contains("Valid values"),
        "Should mention valid values.\nstderr:\n{}",
        result.stderr
    );
}

#[test]
fn deploy_warns_on_invalid_target_env() {
    // Set invalid target with typo
    let env = env_without_project_config();
    let result = env.run_with_env(
        &["deploy", "--dry-run"],
        &[("CALVIN_TARGETS", "cluade-code")],
    );
    assert!(
        result.success,
        "Deploy failed:\n{}",
        result.combined_output()
    );

    // Should contain warning about invalid target
    assert!(
        result.stderr.contains("Warning:") || result.stderr.contains("Unknown target"),
        "Should warn about invalid target.\nstderr:\n{}",
        result.stderr
    );

    // Should suggest the correct value
    assert!(
        result.stderr.contains("claude-code") || result.stderr.contains("Did you mean"),
        "Should suggest 'claude-code'. stderr:\n{}",
        result.stderr
    );
}
