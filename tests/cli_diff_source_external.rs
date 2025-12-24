//! Integration tests for `calvin diff --source` semantics (multi-layer Phase 3).
//!
//! PRD: `--source PATH` replaces the project layer path, but should not change the project root.
//! Diff must be anchored to the current working directory (project root), not `source.parent()`.

use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn write_asset(promptpack: &std::path::Path, filename: &str, body: &str) {
    fs::create_dir_all(promptpack).unwrap();
    fs::write(promptpack.join(filename), body).unwrap();
}

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

#[test]
fn diff_with_external_source_anchors_to_project_root() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();
    fs::create_dir_all(project_dir.join(".git")).unwrap();
    let fake_home = project_dir.join("fake_home");
    fs::create_dir_all(&fake_home).unwrap();

    // "External" promptpack (not at ./ .promptpack).
    let external_root = project_dir.join("shared");
    let external_promptpack = external_root.join(".promptpack");
    write_asset(
        &external_promptpack,
        "solo.md",
        r#"---
kind: policy
description: Solo
scope: project
---
SOLO
"#,
    );

    // First deploy (to establish outputs + lockfile at the *project root*).
    let deploy = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &fake_home)
        .env("XDG_CONFIG_HOME", fake_home.join(".config"))
        .args([
            "deploy",
            "--yes",
            "--no-user-layer",
            "--no-additional-layers",
            "--source",
            external_promptpack.to_string_lossy().as_ref(),
        ])
        .output()
        .unwrap();

    assert!(
        deploy.status.success(),
        "deploy failed: {}",
        String::from_utf8_lossy(&deploy.stderr)
    );
    assert!(
        project_dir.join("calvin.lock").exists(),
        "expected lockfile at project root"
    );

    // Now diff the same external source. If diff incorrectly anchors to `source.parent()`,
    // it will look for outputs + lockfile next to the external promptpack and report "new".
    let diff = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &fake_home)
        .env("XDG_CONFIG_HOME", fake_home.join(".config"))
        .args([
            "diff",
            "--json",
            "--source",
            external_promptpack.to_string_lossy().as_ref(),
        ])
        .output()
        .unwrap();

    assert!(
        diff.status.success(),
        "diff failed: {}",
        String::from_utf8_lossy(&diff.stderr)
    );

    let stdout = String::from_utf8_lossy(&diff.stdout);
    assert!(
        !stdout.contains("\"status\":\"new\""),
        "expected no new files when diffing immediately after deploy; got:\n{stdout}"
    );
    assert!(
        stdout.contains("\"status\":\"unchanged\""),
        "expected at least one unchanged file; got:\n{stdout}"
    );
}
