//! Integration tests for `--source` semantics (multi-layer Phase 3).
//!
//! PRD: `--source PATH` replaces the project layer path, but should not change the project root
//! (outputs + lockfile should still be anchored to the current project directory).

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
fn deploy_with_external_source_writes_lockfile_to_project_root() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();
    fs::create_dir_all(project_dir.join(".git")).unwrap();

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
targets: [cursor]
---
SOLO
"#,
    );

    let output = Command::new(bin())
        .current_dir(project_dir)
        .args([
            "deploy",
            "--yes",
            "--targets",
            "cursor",
            "--source",
            external_promptpack.to_string_lossy().as_ref(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "deploy failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Output should be written to the project root (relative paths).
    assert!(project_dir.join(".cursor/rules/solo/RULE.md").exists());

    // Lockfile should be in the project root, not next to the external promptpack.
    assert!(
        project_dir.join("calvin.lock").exists(),
        "expected lockfile at project root"
    );
    assert!(
        !external_root.join("calvin.lock").exists(),
        "did not expect lockfile next to external promptpack"
    );
}
