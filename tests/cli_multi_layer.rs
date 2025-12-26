//! Integration tests for multi-layer promptpack behavior (Phase 1)

mod common;

use std::fs;
use std::process::Command;

use common::WindowsCompatExt;
use tempfile::tempdir;

fn write_asset(promptpack: &std::path::Path, filename: &str, body: &str) {
    fs::create_dir_all(promptpack).unwrap();
    fs::write(promptpack.join(filename), body).unwrap();
}

#[test]
fn deploy_merges_user_and_project_layers_with_project_override() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    // Avoid the "not a git repository" interactive confirmation prompt.
    fs::create_dir_all(project_dir.join(".git")).unwrap();

    // Fake HOME with a user layer at ~/.calvin/.promptpack
    let fake_home = project_dir.join("fake_home");
    fs::create_dir_all(&fake_home).unwrap();
    let user_layer = fake_home.join(".calvin/.promptpack");

    // User layer defines a shared asset and a user-only asset
    write_asset(
        &user_layer,
        "shared.md",
        r#"---
kind: policy
description: Shared (user)
scope: project
targets: [cursor]
---
USER SHARED
"#,
    );
    write_asset(
        &user_layer,
        "user-only.md",
        r#"---
kind: policy
description: User only
scope: project
targets: [cursor]
---
USER ONLY
"#,
    );

    // Project layer overrides shared.md
    let project_layer = project_dir.join(".promptpack");
    write_asset(
        &project_layer,
        "shared.md",
        r#"---
kind: policy
description: Shared (project)
scope: project
targets: [cursor]
---
PROJECT SHARED
"#,
    );

    // Deploy
    let bin = env!("CARGO_BIN_EXE_calvin");
    let output = Command::new(bin)
        .current_dir(project_dir)
        .with_test_home(&fake_home)
        .args(["deploy", "--yes", "--targets", "cursor"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "deploy failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Both assets should deploy to Cursor.
    let shared_output = project_dir.join(".cursor/rules/shared/RULE.md");
    let user_only_output = project_dir.join(".cursor/rules/user-only/RULE.md");
    assert!(shared_output.exists(), "shared output missing");
    assert!(user_only_output.exists(), "user-only output missing");

    // Project layer should override shared asset content.
    let shared_content = fs::read_to_string(shared_output).unwrap();
    assert!(
        shared_content.contains("PROJECT SHARED"),
        "expected project override content, got:\n{}",
        shared_content
    );

    // Lockfile should record provenance (Phase 0 + Phase 1 wiring).
    let lockfile = fs::read_to_string(project_dir.join("calvin.lock")).unwrap();
    assert!(
        lockfile.contains("source_layer = \"project\""),
        "expected lockfile to contain project provenance, got:\n{}",
        lockfile
    );
    assert!(
        lockfile.contains("source_layer = \"user\""),
        "expected lockfile to contain user provenance, got:\n{}",
        lockfile
    );
}

#[test]
fn deploy_succeeds_with_user_layer_only_when_project_layer_missing() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    fs::create_dir_all(project_dir.join(".git")).unwrap();

    let fake_home = project_dir.join("fake_home");
    fs::create_dir_all(&fake_home).unwrap();
    let user_layer = fake_home.join(".calvin/.promptpack");

    write_asset(
        &user_layer,
        "solo.md",
        r#"---
kind: policy
description: Solo (user only)
scope: project
targets: [cursor]
---
SOLO USER
"#,
    );

    let bin = env!("CARGO_BIN_EXE_calvin");
    let output = Command::new(bin)
        .current_dir(project_dir)
        .with_test_home(&fake_home)
        .args(["deploy", "--yes", "--targets", "cursor"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "deploy failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(project_dir.join("calvin.lock").exists());
    assert!(project_dir.join(".cursor/rules/solo/RULE.md").exists());
}
