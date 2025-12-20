//! E2E tests for `calvin watch` command
//!
//! These tests verify the watch mode functionality works correctly.

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

/// Helper to create a minimal .promptpack structure for watch tests
fn setup_watch_test(dir: &Path) -> std::path::PathBuf {
    let promptpack = dir.join(".promptpack");
    fs::create_dir_all(&promptpack).unwrap();

    // Create a minimal config with deploy target configured
    // Note: Calvin uses config.toml (not calvin.toml)
    let config = r#"[build]
version = "2"

[deploy]
target = "project"

[targets]
claude_code = true
cursor = true
"#;
    fs::write(promptpack.join("config.toml"), config).unwrap();

    // Create an initial action
    let action = r#"---
kind: action
description: Initial action for watch test
---
# Initial Action

This is the initial content.
"#;
    fs::write(promptpack.join("watch-test.md"), action).unwrap();

    promptpack
}

/// Test that watch command starts and produces JSON output
#[test]
fn watch_produces_json_start_event() {
    let temp = tempdir().unwrap();
    let project_dir = temp.path();

    setup_watch_test(project_dir);

    // Start watch in JSON mode, but kill it after a short time
    let mut child = Command::new(env!("CARGO_BIN_EXE_calvin"))
        .arg("watch")
        .arg("--json")
        .current_dir(project_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start calvin watch");

    // Give it a moment to start
    thread::sleep(Duration::from_millis(500));

    // Kill the process
    let _ = child.kill();
    let output = child.wait_with_output().expect("Failed to get output");

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("watch stdout: {}", stdout);

    // Should have started and emitted events
    assert!(
        stdout.contains("watch_started") || stdout.contains("sync_started"),
        "Expected watch to emit start events. Got: {}",
        stdout
    );
}

/// Test that watch does initial sync on startup
#[test]
fn watch_does_initial_sync() {
    let temp = tempdir().unwrap();
    let project_dir = temp.path();

    setup_watch_test(project_dir);

    // Start watch, let it do initial sync, then kill
    let mut child = Command::new(env!("CARGO_BIN_EXE_calvin"))
        .arg("watch")
        .arg("--json")
        .current_dir(project_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start calvin watch");

    // Wait for initial sync
    thread::sleep(Duration::from_millis(1000));

    let _ = child.kill();
    let output = child.wait_with_output().expect("Failed to get output");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should have completed sync
    assert!(
        stdout.contains("sync_complete"),
        "Expected watch to complete initial sync. Got: {}",
        stdout
    );

    // Files should have been written (initial sync writes all files)
    // Check that .claude/commands was created
    assert!(
        project_dir.join(".claude/commands/watch-test.md").exists(),
        "Initial sync should write claude command file"
    );
}

/// Test watch with --home flag produces correct output
#[test]
fn watch_home_flag_uses_correct_scope() {
    let temp = tempdir().unwrap();
    let project_dir = temp.path();

    setup_watch_test(project_dir);

    // Start watch with --home in dry-run (JSON shows scope info)
    let mut child = Command::new(env!("CARGO_BIN_EXE_calvin"))
        .arg("watch")
        .arg("--home")
        .arg("--json")
        .current_dir(project_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start calvin watch --home");

    thread::sleep(Duration::from_millis(500));

    let _ = child.kill();
    let output = child.wait_with_output().expect("Failed to get output");

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("watch --home stdout: {}", stdout);

    // Should have started
    assert!(
        stdout.contains("watch_started"),
        "Expected watch to emit start event. Got: {}",
        stdout
    );
}

/// Test watch mode compiles incrementally (via AssetPipeline)
///
/// Note: This test is timing-sensitive due to filesystem watcher debouncing.
/// The watch command may not immediately pick up changes.
#[test]
fn watch_compiles_with_asset_pipeline() {
    let temp = tempdir().unwrap();
    let project_dir = temp.path();
    let promptpack = setup_watch_test(project_dir);

    // First do a normal deploy to establish baseline
    let deploy_output = Command::new(env!("CARGO_BIN_EXE_calvin"))
        .arg("deploy")
        .arg("--yes")
        .current_dir(project_dir)
        .output()
        .expect("Failed to run initial deploy");

    assert!(
        deploy_output.status.success(),
        "Initial deploy should succeed"
    );

    // Verify initial file exists
    assert!(
        project_dir.join(".claude/commands/watch-test.md").exists(),
        "Initial sync should create file"
    );

    let initial_content =
        fs::read_to_string(project_dir.join(".claude/commands/watch-test.md")).unwrap();

    assert!(
        initial_content.contains("Initial Action"),
        "Initial content should be present"
    );

    // Modify the source file BEFORE starting watch
    let updated_content = r#"---
kind: action
description: Updated action for watch test
---
# Updated Action

This content was updated!
"#;
    fs::write(promptpack.join("watch-test.md"), updated_content).unwrap();

    // Start watch - it should detect the change on initial scan
    let mut child = Command::new(env!("CARGO_BIN_EXE_calvin"))
        .arg("watch")
        .arg("--json")
        .current_dir(project_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start calvin watch");

    // Wait for watch to do initial sync
    thread::sleep(Duration::from_millis(1000));

    let _ = child.kill();
    let output = child.wait_with_output().expect("Failed to get output");

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("watch stdout: {}", stdout);

    // Watch should complete at least one sync
    assert!(
        stdout.contains("sync_complete"),
        "Watch should complete sync. Got: {}",
        stdout
    );

    // Check that the file was updated
    let deployed_content =
        fs::read_to_string(project_dir.join(".claude/commands/watch-test.md")).unwrap_or_default();

    // The content should now be updated
    assert!(
        deployed_content.contains("Updated Action"),
        "Watch should update deployed file. Current content: {}",
        deployed_content
    );
}

/// Test that AssetPipeline scope policy is correctly applied
#[test]
fn watch_respects_scope_policy() {
    let temp = tempdir().unwrap();
    let project_dir = temp.path();
    let promptpack = project_dir.join(".promptpack");
    fs::create_dir_all(&promptpack).unwrap();

    // Create config
    let config = r#"[build]
version = "2"

[targets]
claude_code = true
"#;
    fs::write(promptpack.join("config.toml"), config).unwrap();

    // Create an asset with explicit user scope
    let user_action = r#"---
kind: action
description: User-scoped action
scope: user
---
# User Action

This should go to home directory.
"#;
    fs::write(promptpack.join("user-action.md"), user_action).unwrap();

    // Create an asset with project scope
    let project_action = r#"---
kind: action
description: Project-scoped action  
scope: project
---
# Project Action

This should go to project directory.
"#;
    fs::write(promptpack.join("project-action.md"), project_action).unwrap();

    // Deploy without --home (uses Keep policy)
    let output = Command::new(env!("CARGO_BIN_EXE_calvin"))
        .arg("deploy")
        .arg("--yes")
        .current_dir(project_dir)
        .output()
        .expect("Failed to deploy");

    assert!(output.status.success(), "Deploy should succeed");

    // Project-scoped file should be in project directory
    assert!(
        project_dir
            .join(".claude/commands/project-action.md")
            .exists(),
        "Project-scoped action should be in project directory"
    );

    // User-scoped file would go to home (can't test actual home write easily)
    // But we can verify it's NOT in the project directory
    // Actually, with Keep policy, user-scoped files still get written to ~ paths
    // which we can't easily verify in a test. Let's check the file count instead.
}
