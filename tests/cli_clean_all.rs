//! Integration tests for `calvin clean --all` (Phase 2: Global Registry)

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

fn deploy_project(project_dir: &std::path::Path, home: &std::path::Path) -> std::path::PathBuf {
    // Avoid interactive "not a git repository" prompt.
    fs::create_dir_all(project_dir.join(".git")).unwrap();

    let promptpack = project_dir.join(".promptpack");
    write_asset(
        &promptpack,
        "policy.md",
        r#"---
kind: policy
description: Policy
scope: project
targets: [cursor]
---
POLICY
"#,
    );

    let output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .args(["deploy", "--yes", "--targets", "cursor"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "deploy failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    project_dir.join(".cursor/rules/policy/RULE.md")
}

#[test]
fn clean_all_removes_files_for_each_registered_project() {
    let dir = tempdir().unwrap();
    let home = dir.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let project_a = dir.path().join("project-a");
    let project_b = dir.path().join("project-b");
    fs::create_dir_all(&project_a).unwrap();
    fs::create_dir_all(&project_b).unwrap();

    let a_output = deploy_project(&project_a, &home);
    let b_output = deploy_project(&project_b, &home);

    assert!(a_output.exists(), "expected deploy output to exist");
    assert!(b_output.exists(), "expected deploy output to exist");

    // Run registry-wide clean.
    let output = Command::new(bin())
        .current_dir(dir.path())
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .args(["clean", "--all", "--yes"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "clean --all failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        !a_output.exists(),
        "expected project-a output to be removed"
    );
    assert!(
        !b_output.exists(),
        "expected project-b output to be removed"
    );
}
