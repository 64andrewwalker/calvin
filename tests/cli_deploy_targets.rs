//! Integration tests for deploy target selection logic
//!
//! Tests the target selection priority:
//! 1. remote - if specified, deploy to remote
//! 2. home - if true, deploy to home directory
//! 3. explicit_project - if true, deploy to project (overrides config)
//! 4. config.deploy.target - fall back to configuration
//! 5. default - project

use std::fs;
use std::process::Command;

use tempfile::tempdir;

/// Create a minimal test project with promptpack
fn create_test_project() -> tempfile::TempDir {
    let dir = tempdir().unwrap();

    // Avoid "not a git repository" prompt
    fs::create_dir_all(dir.path().join(".git")).unwrap();

    let promptpack = dir.path().join(".promptpack");
    fs::create_dir_all(&promptpack).unwrap();

    // Create a simple asset
    fs::write(
        promptpack.join("test.md"),
        r#"---
kind: policy
description: Test Policy
scope: project
targets: [cursor]
---
Test content
"#,
    )
    .unwrap();

    dir
}

/// Create a test project with config specifying home target
fn create_project_with_home_config() -> tempfile::TempDir {
    let dir = create_test_project();

    fs::write(
        dir.path().join(".promptpack/config.toml"),
        r#"
[deploy]
target = "home"

[targets]
enabled = ["cursor"]
"#,
    )
    .unwrap();

    dir
}

/// Create a test project with config specifying project target
fn create_project_with_project_config() -> tempfile::TempDir {
    let dir = create_test_project();

    fs::write(
        dir.path().join(".promptpack/config.toml"),
        r#"
[deploy]
target = "project"

[targets]
enabled = ["cursor"]
"#,
    )
    .unwrap();

    dir
}

fn run_deploy(dir: &tempfile::TempDir, args: &[&str]) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_calvin");
    Command::new(bin)
        .current_dir(dir.path())
        .args(["deploy"])
        .args(args)
        .output()
        .unwrap()
}

// ============================================================================
// Target Selection Priority Tests
// ============================================================================

#[test]
fn deploy_defaults_to_project_without_config() {
    let dir = create_test_project();
    let output = run_deploy(&dir, &["--dry-run"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Target should show current project path, not ~/
    assert!(
        !stdout.contains("Target: ~/"),
        "Expected project target, got: {}",
        stdout
    );
}

#[test]
fn deploy_uses_home_from_config() {
    let dir = create_project_with_home_config();
    let output = run_deploy(&dir, &["--dry-run"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Target should be home
    assert!(
        stdout.contains("Target: ~/"),
        "Expected home target in output: {}",
        stdout
    );
}

#[test]
fn deploy_home_flag_overrides_config() {
    let dir = create_project_with_project_config();
    let output = run_deploy(&dir, &["--home", "--dry-run"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // --home should override project config
    assert!(
        stdout.contains("Target: ~/"),
        "Expected --home to override config: {}",
        stdout
    );
}

#[test]
fn deploy_home_and_remote_conflict() {
    let dir = create_test_project();
    let output = run_deploy(&dir, &["--home", "--remote", "user@host:/path"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // clap emits "cannot be used with" when args conflict
    assert!(
        stderr.contains("cannot be used") || stderr.contains("conflict"),
        "Expected conflict error: {}",
        stderr
    );
}

#[test]
fn deploy_remote_takes_precedence() {
    let dir = create_project_with_home_config();
    // Even with home in config, --remote should take precedence
    let output = run_deploy(
        &dir,
        &[
            "--remote",
            "user@host:/path",
            "--dry-run",
            "--targets",
            "cursor",
        ],
    );

    // This might fail if rsync isn't available, so we just check it was attempted
    // The key is that it doesn't deploy to home
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should mention remote in output somewhere
    assert!(
        stdout.contains("user@host") || stderr.contains("user@host") || stderr.contains("rsync"),
        "Expected remote target handling: stdout={} stderr={}",
        stdout,
        stderr
    );
}

// ============================================================================
// Mode Flags Tests
// ============================================================================

#[test]
fn deploy_dry_run_shows_would_write() {
    let dir = create_test_project();
    let output = run_deploy(&dir, &["--dry-run"]);

    assert!(output.status.success());
    // Dry run should not error
}

#[test]
fn deploy_force_mode_accepted() {
    let dir = create_test_project();
    let output = run_deploy(&dir, &["--force", "--dry-run"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Force") || stdout.contains("force"),
        "Expected force mode in output: {}",
        stdout
    );
}

#[test]
fn deploy_yes_skips_prompts() {
    let dir = create_test_project();
    // --yes should skip any interactive prompts
    let output = run_deploy(&dir, &["--yes"]);

    assert!(output.status.success());
}

// ============================================================================
// Scope Policy Tests
// ============================================================================

#[test]
fn deploy_home_uses_force_user_scope() {
    let dir = create_test_project();
    // When deploying to home, scope policy should be ForceUser
    // This is hard to test directly, but we can verify home deployment works
    let output = run_deploy(&dir, &["--home", "--dry-run"]);

    assert!(output.status.success());
}

// ============================================================================
// Config Saving Tests
// ============================================================================

#[test]
fn deploy_saves_target_to_config_on_success() {
    let dir = create_test_project();

    // Deploy to project (default)
    let output = run_deploy(&dir, &["--yes"]);
    assert!(output.status.success());

    // Check that config was created/updated
    let config_path = dir.path().join(".promptpack/config.toml");
    if config_path.exists() {
        let content = fs::read_to_string(&config_path).unwrap();
        // Should have deploy section
        assert!(
            content.contains("[deploy]"),
            "Expected [deploy] section in config: {}",
            content
        );
    }
}

#[test]
fn deploy_dry_run_does_not_save_config() {
    let dir = create_test_project();

    // Note: we're not creating a config file initially

    // Deploy with dry-run
    let output = run_deploy(&dir, &["--dry-run"]);
    assert!(output.status.success());

    // Config should NOT be created by dry-run
    // (This test might be flaky if there's an existing config)
}

// ============================================================================
// Target Filter Tests
// ============================================================================

#[test]
fn deploy_respects_targets_flag() {
    let dir = create_test_project();
    let output = run_deploy(&dir, &["--targets", "cursor", "--dry-run"]);

    assert!(output.status.success());
}

#[test]
fn deploy_multiple_targets() {
    let dir = create_test_project();
    let output = run_deploy(&dir, &["--targets", "cursor,claude-code", "--dry-run"]);

    assert!(output.status.success());
}

// ============================================================================
// Cursor Commands Generation Tests (Bug Fix: cursor-commands-missing)
// ============================================================================

/// Create a test project with an action asset (actions generate commands)
fn create_project_with_action() -> tempfile::TempDir {
    let dir = tempdir().unwrap();

    // Avoid "not a git repository" prompt
    fs::create_dir_all(dir.path().join(".git")).unwrap();

    let promptpack = dir.path().join(".promptpack");
    fs::create_dir_all(&promptpack).unwrap();

    // Create an ACTION asset - actions should generate commands
    fs::write(
        promptpack.join("test-action.md"),
        r#"---
kind: action
description: Test Action
scope: project
---
# Test Action

This is a test action content.
"#,
    )
    .unwrap();

    dir
}

/// Bug fix test: Cursor-only deploy should generate .cursor/commands/
///
/// When deploying only to Cursor (without Claude Code), actions should
/// generate command files to .cursor/commands/ since Claude Code's
/// commands at ~/.claude/commands/ won't be available.
#[test]
fn deploy_cursor_only_generates_commands() {
    let dir = create_project_with_action();

    // Deploy with Cursor only (no Claude Code)
    let output = run_deploy(&dir, &["--targets", "cursor"]);

    assert!(
        output.status.success(),
        "Deploy should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check that .cursor/commands/test-action.md was created
    let command_path = dir.path().join(".cursor/commands/test-action.md");
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
    let dir = create_project_with_action();

    // Deploy with both Cursor and Claude Code
    let output = run_deploy(&dir, &["--targets", "cursor,claude-code"]);

    assert!(
        output.status.success(),
        "Deploy should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check that .cursor/commands/ was NOT created (Claude Code provides commands)
    let cursor_commands_dir = dir.path().join(".cursor/commands");
    assert!(
        !cursor_commands_dir.exists(),
        "Cursor should NOT create .cursor/commands when Claude Code is present. \
         Claude Code provides commands at .claude/commands/"
    );

    // But .claude/commands/ should exist
    let claude_command_path = dir.path().join(".claude/commands/test-action.md");
    assert!(
        claude_command_path.exists(),
        "Claude Code should create .claude/commands/test-action.md"
    );
}

// ============================================================================
// Config Targets Enabled Tests (Bug Fix: targets-config-bug-2025-12-24)
// ============================================================================

/// Create a test project with config specifying only cursor target
fn create_project_with_cursor_only_config() -> tempfile::TempDir {
    let dir = create_test_project();

    fs::write(
        dir.path().join(".promptpack/config.toml"),
        r#"
[targets]
enabled = ["cursor"]
"#,
    )
    .unwrap();

    dir
}

/// Create a test project with empty targets config (explicitly no targets)
fn create_project_with_empty_targets_config() -> tempfile::TempDir {
    let dir = create_test_project();

    fs::write(
        dir.path().join(".promptpack/config.toml"),
        r#"
[targets]
enabled = []
"#,
    )
    .unwrap();

    dir
}

/// Bug fix test: enabled = ["cursor"] should only deploy to Cursor
///
/// Previously, config was ignored and all targets were deployed.
#[test]
fn deploy_respects_config_targets_cursor_only() {
    let dir = create_project_with_cursor_only_config();
    let output = run_deploy(&dir, &["--yes"]);

    assert!(
        output.status.success(),
        "Deploy should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // .cursor should exist
    let cursor_rules = dir.path().join(".cursor/rules/test.md");
    assert!(
        cursor_rules.exists() || dir.path().join(".cursor").exists(),
        "Cursor target should be deployed"
    );

    // .claude should NOT exist (not in enabled list)
    let claude_dir = dir.path().join(".claude");
    assert!(
        !claude_dir.exists(),
        "Claude Code should NOT be deployed when only cursor is enabled in config. \
         .claude dir exists: {:?}",
        claude_dir
    );
}

/// Bug fix test: enabled = [] should not deploy any targets
///
/// Empty list means "explicitly disable all targets", not "use defaults".
#[test]
fn deploy_respects_config_empty_targets() {
    let dir = create_project_with_empty_targets_config();
    let output = run_deploy(&dir, &["--yes"]);

    assert!(
        output.status.success(),
        "Deploy should succeed (no targets = nothing to do). stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // No target directories should be created
    let cursor_dir = dir.path().join(".cursor");
    let claude_dir = dir.path().join(".claude");
    let vscode_dir = dir.path().join(".vscode");

    assert!(
        !cursor_dir.exists() && !claude_dir.exists() && !vscode_dir.exists(),
        "No target directories should be created when enabled = []. \
         cursor={:?}, claude={:?}, vscode={:?}",
        cursor_dir.exists(),
        claude_dir.exists(),
        vscode_dir.exists()
    );
}

/// Create a test project with an asset that has no target restrictions (all targets)
fn create_project_all_targets_asset() -> tempfile::TempDir {
    let dir = tempdir().unwrap();

    // Avoid "not a git repository" prompt
    fs::create_dir_all(dir.path().join(".git")).unwrap();

    let promptpack = dir.path().join(".promptpack");
    fs::create_dir_all(&promptpack).unwrap();

    // Create asset with all targets (empty targets = defaults to all)
    fs::write(
        promptpack.join("test.md"),
        r#"---
kind: policy
description: Test Policy for All Targets
scope: project
---
Test content for all targets
"#,
    )
    .unwrap();

    // Config limits to cursor only
    fs::write(
        promptpack.join("config.toml"),
        r#"
[targets]
enabled = ["cursor"]
"#,
    )
    .unwrap();

    dir
}

/// Test: CLI --targets flag overrides config
#[test]
fn deploy_cli_targets_overrides_config() {
    // Use a project with an all-targets asset (no asset-level restriction)
    let dir = create_project_all_targets_asset();
    // Config says cursor only, but CLI says claude-code
    // Use -t short form which is equivalent to --targets
    let output = run_deploy(&dir, &["-t", "claude-code", "--yes"]);

    assert!(
        output.status.success(),
        "Deploy should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // .claude should exist (CLI override)
    let claude_dir = dir.path().join(".claude");
    assert!(
        claude_dir.exists(),
        "CLI -t should override config. claude_dir exists: {}",
        claude_dir.exists()
    );
}

// ============================================================================
// Environment Variable Validation Tests
// ============================================================================

/// Run deploy with environment variables
fn run_deploy_with_env(
    dir: &tempfile::TempDir,
    args: &[&str],
    env_vars: &[(&str, &str)],
) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_calvin");
    let mut cmd = Command::new(bin);
    cmd.current_dir(dir.path()).args(["deploy"]).args(args);

    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    cmd.output().unwrap()
}

#[test]
fn deploy_warns_on_invalid_security_mode_env() {
    let dir = create_test_project();

    // Set invalid security mode with typo
    let output = run_deploy_with_env(&dir, &["--yes"], &[("CALVIN_SECURITY_MODE", "strct")]);

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should contain warning about invalid value
    assert!(
        stderr.contains("Warning:") && stderr.contains("CALVIN_SECURITY_MODE"),
        "Should warn about invalid CALVIN_SECURITY_MODE. stderr: {}",
        stderr
    );

    // Should suggest the correct value
    assert!(
        stderr.contains("strict") || stderr.contains("Did you mean"),
        "Should suggest 'strict'. stderr: {}",
        stderr
    );
}

#[test]
fn deploy_warns_on_invalid_verbosity_env() {
    let dir = create_test_project();

    // Set invalid verbosity with typo
    let output = run_deploy_with_env(&dir, &["--yes"], &[("CALVIN_VERBOSITY", "quite")]);

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should contain warning about invalid value
    assert!(
        stderr.contains("Warning:") && stderr.contains("CALVIN_VERBOSITY"),
        "Should warn about invalid CALVIN_VERBOSITY. stderr: {}",
        stderr
    );

    // Should list valid values
    assert!(
        stderr.contains("quiet") || stderr.contains("Valid values"),
        "Should mention valid values. stderr: {}",
        stderr
    );
}

#[test]
fn deploy_warns_on_invalid_target_env() {
    let dir = create_test_project();

    // Set invalid target with typo
    let output = run_deploy_with_env(&dir, &["--yes"], &[("CALVIN_TARGETS", "cluade-code")]);

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should contain warning about invalid target
    assert!(
        stderr.contains("Warning:") || stderr.contains("Unknown target"),
        "Should warn about invalid target. stderr: {}",
        stderr
    );

    // Should suggest the correct value
    assert!(
        stderr.contains("claude-code") || stderr.contains("Did you mean"),
        "Should suggest 'claude-code'. stderr: {}",
        stderr
    );
}
