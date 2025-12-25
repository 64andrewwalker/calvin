//! Tests for `calvin init --user` output format (PRD §12.5)

use std::process::Command;

fn calvin() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_calvin"));
    cmd.env_remove("NO_COLOR");
    cmd.env("TERM", "xterm-256color");
    cmd
}

/// PRD §12.5: init --user should show tree structure of created directories
#[test]
fn init_user_shows_tree_structure() {
    let temp_dir = tempfile::tempdir().unwrap();
    let user_layer = temp_dir.path().join(".calvin/.promptpack");

    let output = calvin()
        .arg("init")
        .arg("--user")
        .env("CALVIN_SOURCES_USER_LAYER_PATH", &user_layer)
        .output()
        .expect("Failed to execute calvin init --user");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "init --user should succeed. stderr: {}",
        stderr
    );

    // Should show directory structure (using indented list format)
    assert!(
        stdout.contains("  - ") || stdout.contains("  - config.toml"),
        "Expected directory structure with '  - ' but got:\n{}",
        stdout
    );

    // Should show created directories
    assert!(
        stdout.contains("config.toml"),
        "Expected to show config.toml in output:\n{}",
        stdout
    );
    assert!(
        stdout.contains("policies"),
        "Expected to show policies/ in output:\n{}",
        stdout
    );
    assert!(
        stdout.contains("actions"),
        "Expected to show actions/ in output:\n{}",
        stdout
    );
    assert!(
        stdout.contains("agents"),
        "Expected to show agents/ in output:\n{}",
        stdout
    );
}

/// PRD §12.5: init --user should show next steps
#[test]
fn init_user_shows_next_steps() {
    let temp_dir = tempfile::tempdir().unwrap();
    let user_layer = temp_dir.path().join(".calvin/.promptpack");

    let output = calvin()
        .arg("init")
        .arg("--user")
        .env("CALVIN_SOURCES_USER_LAYER_PATH", &user_layer)
        .output()
        .expect("Failed to execute calvin init --user");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());

    // Should show "Next steps" section
    assert!(
        stdout.contains("Next") || stdout.contains("next"),
        "Expected next steps section:\n{}",
        stdout
    );

    // Should mention adding prompts
    assert!(
        stdout.contains("Add") || stdout.contains("prompts"),
        "Expected instructions about adding prompts:\n{}",
        stdout
    );
}

/// init --user with --force should work when layer already exists
#[test]
fn init_user_force_overwrites_existing() {
    let temp_dir = tempfile::tempdir().unwrap();
    let user_layer = temp_dir.path().join(".calvin/.promptpack");

    // First init
    let output1 = calvin()
        .arg("init")
        .arg("--user")
        .env("CALVIN_SOURCES_USER_LAYER_PATH", &user_layer)
        .output()
        .expect("Failed to execute first init");
    assert!(output1.status.success());

    // Second init without force should fail
    let output2 = calvin()
        .arg("init")
        .arg("--user")
        .env("CALVIN_SOURCES_USER_LAYER_PATH", &user_layer)
        .output()
        .expect("Failed to execute second init");
    assert!(!output2.status.success());

    // Second init with force should succeed
    let output3 = calvin()
        .arg("init")
        .arg("--user")
        .arg("--force")
        .env("CALVIN_SOURCES_USER_LAYER_PATH", &user_layer)
        .output()
        .expect("Failed to execute init with force");
    assert!(output3.status.success());
}
