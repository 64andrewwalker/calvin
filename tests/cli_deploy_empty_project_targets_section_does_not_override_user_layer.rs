//! Regression test: an *empty* `[targets]` section in the project layer config.toml
//! must not override a valid user-layer `[targets] enabled = [...]`.
//!
//! Real-world trigger: generated templates often include `[targets]` with only commented
//! examples, which produces an empty TOML table and previously caused Calvin to fall back
//! to "all targets" (mis-deploying).

use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

#[test]
fn deploy_does_not_let_empty_project_targets_section_override_user_layer_targets() {
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

    // Project layer: include an *empty* [targets] section (comment-only examples).
    fs::create_dir_all(project_dir.join(".promptpack")).unwrap();
    fs::write(
        project_dir.join(".promptpack/config.toml"),
        r#"
[targets]
# enabled = ["cursor"]
"#,
    )
    .unwrap();

    // Project layer asset targets both cursor + antigravity; target selection should decide.
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
