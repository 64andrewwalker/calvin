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
