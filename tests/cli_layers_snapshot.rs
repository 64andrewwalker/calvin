//! Snapshot tests for `calvin layers` command output
//!
//! These tests verify that the layers command output remains consistent.

use std::fs;
use std::process::Command;

fn calvin() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_calvin"));
    // Disable color for consistent output
    cmd.env("NO_COLOR", "1");
    cmd.env("TERM", "dumb");
    cmd
}

/// Create a test project with user and project layers
fn setup_test_project(temp_dir: &std::path::Path, user_layer: &std::path::Path) {
    // Create project .promptpack
    let project_promptpack = temp_dir.join(".promptpack");
    fs::create_dir_all(project_promptpack.join("policies")).unwrap();
    fs::write(
        project_promptpack.join("policies/project-rule.md"),
        r#"---
id: project-rule
title: Project Rule
description: Project level rule for testing
kind: policy
targets: [cursor]
---
This is a project-level rule.
"#,
    )
    .unwrap();

    // Create user layer
    fs::create_dir_all(user_layer.join("policies")).unwrap();
    fs::write(
        user_layer.join("policies/user-rule.md"),
        r#"---
id: user-rule
title: User Rule
description: User level rule for testing
kind: policy
targets: [cursor]
---
This is a user-level rule.
"#,
    )
    .unwrap();
}

#[test]
fn layers_output_snapshot_with_both_layers() {
    let temp_dir = tempfile::tempdir().unwrap();
    let user_layer = temp_dir.path().join(".calvin/.promptpack");
    setup_test_project(temp_dir.path(), &user_layer);

    let output = calvin()
        .arg("layers")
        .current_dir(temp_dir.path())
        .env("CALVIN_SOURCES_USER_LAYER_PATH", &user_layer)
        .output()
        .expect("Failed to run calvin layers");

    let stdout = String::from_utf8_lossy(&output.stdout);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "Command should succeed. stderr: {}",
        stderr
    );

    // Verify key elements are present
    assert!(
        stdout.contains("Layer Stack"),
        "Should show layer stack header"
    );
    assert!(stdout.contains("[project]"), "Should show project layer");
    assert!(stdout.contains("[user]"), "Should show user layer");
    assert!(
        stdout.contains("2 merged assets"),
        "Should show merged count"
    );
    assert!(
        stdout.contains("0 overridden"),
        "Should show override count"
    );
}

#[test]
fn layers_output_snapshot_with_override() {
    let temp_dir = tempfile::tempdir().unwrap();
    let user_layer = temp_dir.path().join(".calvin/.promptpack");

    // Create project .promptpack
    let project_promptpack = temp_dir.path().join(".promptpack");
    fs::create_dir_all(project_promptpack.join("policies")).unwrap();

    // Create conflicting asset with same ID in both layers
    fs::write(
        project_promptpack.join("policies/shared.md"),
        r#"---
id: shared-rule
title: Project Shared
description: Project version of shared rule
kind: policy
targets: [cursor]
---
Project version of shared rule.
"#,
    )
    .unwrap();

    fs::create_dir_all(user_layer.join("policies")).unwrap();
    fs::write(
        user_layer.join("policies/shared.md"),
        r#"---
id: shared-rule
title: User Shared
description: User version of shared rule
kind: policy
targets: [cursor]
---
User version of shared rule.
"#,
    )
    .unwrap();

    let output = calvin()
        .arg("layers")
        .current_dir(temp_dir.path())
        .env("CALVIN_SOURCES_USER_LAYER_PATH", &user_layer)
        .output()
        .expect("Failed to run calvin layers");

    let stdout = String::from_utf8_lossy(&output.stdout);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "Command should succeed. stderr: {}",
        stderr
    );

    // Verify override is reported
    assert!(
        stdout.contains("1 overridden") || stdout.contains("1 override"),
        "Should show 1 overridden asset, got:\n{}",
        stdout
    );
}

#[test]
fn layers_output_snapshot_user_only() {
    let temp_dir = tempfile::tempdir().unwrap();
    let user_layer = temp_dir.path().join(".calvin/.promptpack");

    // Only create user layer, no project .promptpack
    fs::create_dir_all(user_layer.join("policies")).unwrap();
    fs::write(
        user_layer.join("policies/user-rule.md"),
        r#"---
id: user-rule
title: User Rule
description: User-only rule for testing
kind: policy
targets: [cursor]
---
User-only rule.
"#,
    )
    .unwrap();

    let output = calvin()
        .arg("layers")
        .current_dir(temp_dir.path())
        .env("CALVIN_SOURCES_USER_LAYER_PATH", &user_layer)
        .output()
        .expect("Failed to run calvin layers");

    let stdout = String::from_utf8_lossy(&output.stdout);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "Command should succeed. stderr: {}",
        stderr
    );

    // Should show only user layer
    assert!(stdout.contains("[user]"), "Should show user layer");
    assert!(
        !stdout.contains("[project]"),
        "Should NOT show project layer"
    );
    assert!(
        stdout.contains("1 merged assets"),
        "Should show 1 merged asset"
    );
}
