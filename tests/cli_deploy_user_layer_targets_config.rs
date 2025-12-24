//! Integration tests for multi-layer config.toml merge behavior.
//!
//! PRD: `~/.calvin/.promptpack/config.toml` participates in config merge across layers.
//! In particular, `[targets] enabled = [...]` from the user layer should affect deploy
//! when the project does not explicitly set targets.

use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

#[test]
fn deploy_respects_targets_enabled_from_user_layer_promptpack_config() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    // Avoid interactive "not a git repository" prompt.
    fs::create_dir_all(project_dir.join(".git")).unwrap();

    // Fake HOME with a user layer at ~/.calvin/.promptpack/config.toml
    let home = project_dir.join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(home.join(".config")).unwrap();
    fs::create_dir_all(home.join(".calvin/.promptpack")).unwrap();
    fs::write(
        home.join(".calvin/.promptpack/config.toml"),
        r#"
[targets]
enabled = ["antigravity"]
"#,
    )
    .unwrap();

    // Project layer asset targets both cursor + antigravity; target selection should decide.
    fs::create_dir_all(project_dir.join(".promptpack")).unwrap();
    fs::write(
        project_dir.join(".promptpack/test.md"),
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

    // Deploy without `--targets` (should use merged layer config).
    let output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .args(["deploy", "--yes"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "deploy failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Antigravity outputs should exist...
    assert!(project_dir.join(".agent/rules/test.md").exists());
    // ...but Cursor outputs should not (targets limited to antigravity).
    assert!(!project_dir.join(".cursor/rules/test/RULE.md").exists());
}
