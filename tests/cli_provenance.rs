//! Integration tests for `calvin provenance` (Phase 4)

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
fn provenance_includes_source_layer_and_asset() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();
    fs::create_dir_all(project_dir.join(".git")).unwrap();

    let home = project_dir.join("home");
    fs::create_dir_all(&home).unwrap();

    // User layer defines an asset.
    let user_layer = home.join(".calvin/.promptpack");
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

    // Project layer overrides same id.
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

    let deploy = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .args(["deploy", "--yes", "--targets", "cursor"])
        .output()
        .unwrap();

    assert!(
        deploy.status.success(),
        "deploy failed: {}",
        String::from_utf8_lossy(&deploy.stderr)
    );

    let output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .args(["--json", "provenance"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "provenance failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let entries = json.get("entries").and_then(|v| v.as_array()).unwrap();
    assert!(!entries.is_empty(), "expected provenance entries");

    let has_project_source = entries.iter().any(|e| {
        e.get("source_layer")
            .and_then(|v| v.as_str())
            .is_some_and(|s| s == "project")
    });
    assert!(
        has_project_source,
        "expected project provenance, got {json}"
    );
}

#[test]
fn provenance_filter_limits_output() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();
    fs::create_dir_all(project_dir.join(".git")).unwrap();

    let promptpack = project_dir.join(".promptpack");
    write_asset(
        &promptpack,
        "one.md",
        r#"---
kind: policy
description: One
scope: project
targets: [cursor]
---
ONE
"#,
    );
    write_asset(
        &promptpack,
        "two.md",
        r#"---
kind: policy
description: Two
scope: project
targets: [cursor]
---
TWO
"#,
    );

    let deploy = Command::new(bin())
        .current_dir(project_dir)
        .args(["deploy", "--yes", "--targets", "cursor"])
        .output()
        .unwrap();
    assert!(deploy.status.success());

    let output = Command::new(bin())
        .current_dir(project_dir)
        .args(["--json", "provenance", "--filter", "one"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let entries = json.get("entries").and_then(|v| v.as_array()).unwrap();
    assert!(
        entries.iter().all(|e| {
            e.get("path")
                .and_then(|v| v.as_str())
                .is_some_and(|p| p.contains("one"))
        }),
        "expected filter to limit results, got {json}"
    );
}
