//! Regression tests: `calvin diff` must use the resolved multi-layer stack.
//!
//! In multi-layer mode, deploy/watch compile assets from user/custom/project layers.
//! Diff must preview the same outputs, otherwise it diverges from what deploy will do.

mod common;

use std::fs;
use std::process::Command;

use common::WindowsCompatExt;
use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

#[test]
fn diff_includes_user_layer_assets_and_respects_layer_targets_config() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    // Avoid interactive "not a git repository" prompt.
    fs::create_dir_all(project_dir.join(".git")).unwrap();

    // Fake HOME with a user layer promptpack.
    let home = project_dir.join("home");
    fs::create_dir_all(home.join(".config")).unwrap();
    fs::create_dir_all(home.join(".calvin/.promptpack")).unwrap();

    // User layer config restricts targets to antigravity only.
    fs::write(
        home.join(".calvin/.promptpack/config.toml"),
        r#"
[targets]
enabled = ["antigravity"]
"#,
    )
    .unwrap();

    // User layer asset targets both antigravity + cursor; the config must decide.
    fs::write(
        home.join(".calvin/.promptpack/test.md"),
        r#"---
kind: policy
description: Test
scope: project
targets: [antigravity, cursor]
---
TEST
"#,
    )
    .unwrap();

    // Diff in the project directory with NO project .promptpack/ present.
    let output = Command::new(bin())
        .current_dir(project_dir)
        .with_test_home(&home)
        .args(["diff", "--json"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "diff failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Antigravity output must be present (status should be "new" in a fresh project dir).
    assert!(
        stdout.contains(".agent/rules/test.md") && stdout.contains("\"status\":\"new\""),
        "expected diff to include antigravity output; got:\n{stdout}"
    );

    // Cursor output must NOT be present because targets are limited to antigravity.
    assert!(
        !stdout.contains(".cursor/"),
        "expected diff to exclude cursor outputs; got:\n{stdout}"
    );
}
