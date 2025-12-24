//! Integration test for `sources.disable_project_layer`.
//!
//! PRD: user config may set `sources.disable_project_layer = true` for debugging/special cases.
//! When enabled, Calvin should ignore the project layer when resolving the layer stack.

use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

fn write_asset(promptpack: &std::path::Path, filename: &str, body: &str) {
    fs::create_dir_all(promptpack).unwrap();
    fs::write(promptpack.join(filename), body).unwrap();
}

#[test]
fn deploy_can_disable_project_layer_via_user_config() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    let home = project_dir.join("home");
    fs::create_dir_all(home.join(".config/calvin")).unwrap();

    // Project layer asset (should be ignored when disable_project_layer=true).
    let project_promptpack = project_dir.join(".promptpack");
    write_asset(
        &project_promptpack,
        "project-only.md",
        r#"---
kind: policy
description: Project only
scope: project
targets: [cursor]
---
PROJECT
"#,
    );

    // User layer asset (should still be deployed).
    let user_promptpack = home.join(".calvin/.promptpack");
    write_asset(
        &user_promptpack,
        "user-only.md",
        r#"---
kind: policy
description: User only
scope: project
targets: [cursor]
---
USER
"#,
    );

    // User config disables project layer.
    fs::write(
        home.join(".config/calvin/config.toml"),
        r#"
[sources]
disable_project_layer = true
"#,
    )
    .unwrap();

    let output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .args(["deploy", "--yes", "--targets", "cursor"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "deploy failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // User layer asset should be deployed.
    assert!(project_dir.join(".cursor/rules/user-only/RULE.md").exists());

    // Project layer asset should be ignored.
    assert!(!project_dir
        .join(".cursor/rules/project-only/RULE.md")
        .exists());
}
