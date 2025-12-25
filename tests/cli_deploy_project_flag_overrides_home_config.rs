//! Regression test: `calvin deploy --project` must override `[deploy].target = "home"`.
//!
//! This fixes the UX trap where a project config defaults deploys to home,
//! but the user wants a one-off deploy to the project without editing config.

use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

#[test]
fn deploy_project_flag_overrides_home_config_without_modifying_config() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    // Avoid interactive "not a git repository" prompt.
    fs::create_dir_all(project_dir.join(".git")).unwrap();

    // Minimal promptpack + config that defaults to home deploys.
    fs::create_dir_all(project_dir.join(".promptpack")).unwrap();
    fs::write(
        project_dir.join(".promptpack/test.md"),
        r#"---
kind: policy
description: Test
scope: project
targets: [cursor]
---
TEST
"#,
    )
    .unwrap();
    fs::write(
        project_dir.join(".promptpack/config.toml"),
        r#"
[deploy]
target = "home"

[targets]
enabled = ["cursor"]
"#,
    )
    .unwrap();

    // Fake HOME so a mistaken home deploy would be observable and isolated.
    let home = project_dir.join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(home.join(".config")).unwrap();

    let output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .args(["deploy", "--project", "--yes"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "deploy failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Must deploy to the project, not HOME.
    assert!(
        project_dir.join(".cursor").exists(),
        "expected project .cursor/"
    );
    assert!(
        !home.join(".cursor").exists(),
        "expected no home .cursor/ when using --project override"
    );

    // CLI override must not rewrite the user's persisted preference.
    let config = fs::read_to_string(project_dir.join(".promptpack/config.toml")).unwrap();
    assert!(
        config.contains("target = \"home\""),
        "expected config to remain home; got:\n{config}"
    );
}
