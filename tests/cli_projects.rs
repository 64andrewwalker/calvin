//! Integration tests for `calvin projects` (Phase 2: Global Registry)

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

fn deploy_project(project_dir: &std::path::Path, home: &std::path::Path) {
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
}

#[test]
fn projects_lists_registered_projects() {
    let dir = tempdir().unwrap();
    let home = dir.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let project_a = dir.path().join("project-a");
    let project_b = dir.path().join("project-b");
    fs::create_dir_all(&project_a).unwrap();
    fs::create_dir_all(&project_b).unwrap();

    deploy_project(&project_a, &home);
    deploy_project(&project_b, &home);

    let output = Command::new(bin())
        .current_dir(dir.path())
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .args(["--json", "projects"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "projects failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("expected JSON output");

    let projects = json
        .get("projects")
        .and_then(|v| v.as_array())
        .expect("expected projects array");

    let paths: Vec<&str> = projects
        .iter()
        .filter_map(|p| p.get("path").and_then(|v| v.as_str()))
        .collect();

    assert!(
        paths.iter().any(|p| p.ends_with("project-a")),
        "expected JSON to include project-a, got:\n{}",
        stdout
    );
    assert!(
        paths.iter().any(|p| p.ends_with("project-b")),
        "expected JSON to include project-b, got:\n{}",
        stdout
    );
}

#[test]
fn projects_prune_removes_entries_with_missing_lockfile() {
    let dir = tempdir().unwrap();
    let home = dir.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let project_a = dir.path().join("project-a");
    let project_b = dir.path().join("project-b");
    fs::create_dir_all(&project_a).unwrap();
    fs::create_dir_all(&project_b).unwrap();

    deploy_project(&project_a, &home);
    deploy_project(&project_b, &home);

    // Remove one lockfile so it becomes invalid.
    fs::remove_file(project_b.join("calvin.lock")).unwrap();

    let output = Command::new(bin())
        .current_dir(dir.path())
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .args(["--json", "projects", "--prune"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "projects --prune failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("expected JSON output");

    let projects = json
        .get("projects")
        .and_then(|v| v.as_array())
        .expect("expected projects array");

    let paths: Vec<&str> = projects
        .iter()
        .filter_map(|p| p.get("path").and_then(|v| v.as_str()))
        .collect();

    assert!(
        paths.iter().any(|p| p.ends_with("project-a")),
        "expected JSON to include project-a, got:\n{}",
        stdout
    );
    assert!(
        !paths.iter().any(|p| p.ends_with("project-b")),
        "expected JSON to NOT include project-b, got:\n{}",
        stdout
    );

    let pruned = json
        .get("pruned")
        .and_then(|v| v.as_array())
        .expect("expected pruned array");
    let pruned_paths: Vec<&str> = pruned.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        pruned_paths.iter().any(|p| p.ends_with("project-b")),
        "expected JSON pruned list to include project-b, got:\n{}",
        stdout
    );
}
