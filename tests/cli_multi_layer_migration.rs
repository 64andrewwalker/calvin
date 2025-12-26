//! Integration tests for asset provenance when an asset migrates between layers.

mod common;

use std::fs;
use std::process::Command;

use common::WindowsCompatExt;
use tempfile::tempdir;

fn write_asset(promptpack: &std::path::Path, filename: &str, body: &str) {
    fs::create_dir_all(promptpack).unwrap();
    fs::write(promptpack.join(filename), body).unwrap();
}

fn lockfile_files_len(lockfile_toml: &str) -> usize {
    let value: toml::Value = toml::from_str(lockfile_toml).unwrap();
    value
        .get("files")
        .and_then(|v| v.as_table())
        .map(|t| t.len())
        .unwrap_or_default()
}

#[test]
fn asset_migrates_from_project_to_user_layer_updates_lockfile_provenance() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    fs::create_dir_all(project_dir.join(".git")).unwrap();

    let fake_home = project_dir.join("fake_home");
    fs::create_dir_all(&fake_home).unwrap();
    let user_layer = fake_home.join(".calvin/.promptpack");

    let project_layer = project_dir.join(".promptpack");
    write_asset(
        &project_layer,
        "review.md",
        r#"---
kind: policy
description: Review (project)
scope: project
targets: [cursor]
---
PROJECT REVIEW
"#,
    );

    let bin = env!("CARGO_BIN_EXE_calvin");
    let output = Command::new(bin)
        .current_dir(project_dir)
        .with_test_home(&fake_home)
        .args(["deploy", "--yes", "--targets", "cursor"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "deploy failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let lockfile_path = project_dir.join("calvin.lock");
    let lockfile_1 = fs::read_to_string(&lockfile_path).unwrap();
    assert_eq!(lockfile_files_len(&lockfile_1), 1);
    assert!(
        lockfile_1.contains("source_layer = \"project\""),
        "{}",
        lockfile_1
    );

    let out_path = project_dir.join(".cursor/rules/review/RULE.md");
    let out_1 = fs::read_to_string(&out_path).unwrap();
    assert!(out_1.contains("PROJECT REVIEW"), "{}", out_1);

    // Move asset to user layer (same ID/output path)
    fs::remove_file(project_layer.join("review.md")).unwrap();
    write_asset(
        &user_layer,
        "review.md",
        r#"---
kind: policy
description: Review (user)
scope: project
targets: [cursor]
---
USER REVIEW
"#,
    );

    let output = Command::new(bin)
        .current_dir(project_dir)
        .with_test_home(&fake_home)
        .args(["deploy", "--yes", "--targets", "cursor"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "deploy failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let lockfile_2 = fs::read_to_string(&lockfile_path).unwrap();
    assert_eq!(lockfile_files_len(&lockfile_2), 1);
    assert!(
        lockfile_2.contains("source_layer = \"user\""),
        "expected provenance to update to user, got:\n{}",
        lockfile_2
    );
    assert!(
        !lockfile_2.contains("source_layer = \"project\""),
        "expected old provenance to be replaced, got:\n{}",
        lockfile_2
    );

    let out_2 = fs::read_to_string(&out_path).unwrap();
    assert!(out_2.contains("USER REVIEW"), "{}", out_2);
}

#[test]
fn asset_migrates_from_user_to_project_layer_updates_lockfile_provenance() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    fs::create_dir_all(project_dir.join(".git")).unwrap();

    let fake_home = project_dir.join("fake_home");
    fs::create_dir_all(&fake_home).unwrap();
    let user_layer = fake_home.join(".calvin/.promptpack");

    write_asset(
        &user_layer,
        "review.md",
        r#"---
kind: policy
description: Review (user)
scope: project
targets: [cursor]
---
USER REVIEW
"#,
    );

    let bin = env!("CARGO_BIN_EXE_calvin");
    let output = Command::new(bin)
        .current_dir(project_dir)
        .with_test_home(&fake_home)
        .args(["deploy", "--yes", "--targets", "cursor"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "deploy failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let lockfile_path = project_dir.join("calvin.lock");
    let lockfile_1 = fs::read_to_string(&lockfile_path).unwrap();
    assert_eq!(lockfile_files_len(&lockfile_1), 1);
    assert!(
        lockfile_1.contains("source_layer = \"user\""),
        "{}",
        lockfile_1
    );

    let out_path = project_dir.join(".cursor/rules/review/RULE.md");
    let out_1 = fs::read_to_string(&out_path).unwrap();
    assert!(out_1.contains("USER REVIEW"), "{}", out_1);

    // Move asset to project layer (same ID/output path)
    fs::remove_file(user_layer.join("review.md")).unwrap();
    let project_layer = project_dir.join(".promptpack");
    write_asset(
        &project_layer,
        "review.md",
        r#"---
kind: policy
description: Review (project)
scope: project
targets: [cursor]
---
PROJECT REVIEW
"#,
    );

    let output = Command::new(bin)
        .current_dir(project_dir)
        .with_test_home(&fake_home)
        .args(["deploy", "--yes", "--targets", "cursor"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "deploy failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let lockfile_2 = fs::read_to_string(&lockfile_path).unwrap();
    assert_eq!(lockfile_files_len(&lockfile_2), 1);
    assert!(
        lockfile_2.contains("source_layer = \"project\""),
        "expected provenance to update to project, got:\n{}",
        lockfile_2
    );
    assert!(
        !lockfile_2.contains("source_layer = \"user\""),
        "expected old provenance to be replaced, got:\n{}",
        lockfile_2
    );

    let out_2 = fs::read_to_string(&out_path).unwrap();
    assert!(out_2.contains("PROJECT REVIEW"), "{}", out_2);
}
