//! TDD Test: calvin layers output format should match PRD §4.4
//!
//! Expected format:
//! - Numbered list (3, 2, 1) from highest to lowest priority
//! - Single-line merged summary
//! - Title: "Layer Stack (highest priority first)" or similar

mod common;

use std::fs;
use std::process::Command;

use common::WindowsCompatExt;
use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

#[test]
fn layers_output_uses_numbered_list() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    fs::create_dir_all(project_dir.join(".git")).unwrap();

    // Create user layer
    let fake_home = project_dir.join("fake_home");
    let user_layer = fake_home.join(".calvin/.promptpack");
    fs::create_dir_all(&user_layer).unwrap();
    fs::write(
        user_layer.join("test.md"),
        r#"---
kind: policy
description: Test
scope: project
targets: [cursor]
---
Content
"#,
    )
    .unwrap();

    // Create project layer
    let project_layer = project_dir.join(".promptpack");
    fs::create_dir_all(&project_layer).unwrap();
    fs::write(
        project_layer.join("local.md"),
        r#"---
kind: policy
description: Local
scope: project
targets: [cursor]
---
Local content
"#,
    )
    .unwrap();

    let output = Command::new(bin())
        .current_dir(project_dir)
        .with_test_home(&fake_home)
        .args(["layers"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should use numbered list format (PRD §4.4)
    // Looking for patterns like "1." or "2."
    assert!(
        stdout.contains("1.")
            || stdout.contains("2.")
            || stdout.contains("1)")
            || stdout.contains("2)"),
        "should use numbered list format:\n{}",
        stdout
    );
}

#[test]
fn layers_output_shows_highest_priority_first() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    fs::create_dir_all(project_dir.join(".git")).unwrap();

    let fake_home = project_dir.join("fake_home");
    let user_layer = fake_home.join(".calvin/.promptpack");
    fs::create_dir_all(&user_layer).unwrap();
    fs::write(
        user_layer.join("test.md"),
        r#"---
kind: policy
description: Test
scope: project
targets: [cursor]
---
"#,
    )
    .unwrap();

    let project_layer = project_dir.join(".promptpack");
    fs::create_dir_all(&project_layer).unwrap();
    fs::write(
        project_layer.join("local.md"),
        r#"---
kind: policy
description: Local
scope: project
targets: [cursor]
---
"#,
    )
    .unwrap();

    let output = Command::new(bin())
        .current_dir(project_dir)
        .with_test_home(&fake_home)
        .args(["layers"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Project should appear before user (highest priority first)
    let project_pos = stdout.find("project");
    let user_pos = stdout.find("user");

    assert!(
        project_pos.is_some() && user_pos.is_some(),
        "should show both project and user layers:\n{}",
        stdout
    );

    // In "highest priority first" ordering, project comes first
    assert!(
        project_pos.unwrap() < user_pos.unwrap(),
        "project (highest) should appear before user (lowest):\n{}",
        stdout
    );
}

#[test]
fn layers_output_shows_merged_summary() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    fs::create_dir_all(project_dir.join(".git")).unwrap();

    let fake_home = project_dir.join("fake_home");
    let user_layer = fake_home.join(".calvin/.promptpack");
    fs::create_dir_all(&user_layer).unwrap();
    fs::write(
        user_layer.join("test.md"),
        r#"---
kind: policy
description: Test
scope: project
targets: [cursor]
---
"#,
    )
    .unwrap();

    let project_layer = project_dir.join(".promptpack");
    fs::create_dir_all(&project_layer).unwrap();
    fs::write(
        project_layer.join("local.md"),
        r#"---
kind: policy
description: Local
scope: project
targets: [cursor]
---
"#,
    )
    .unwrap();

    let output = Command::new(bin())
        .current_dir(project_dir)
        .with_test_home(&fake_home)
        .args(["layers"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show merged count
    assert!(
        stdout.contains("merged") || stdout.contains("Merged"),
        "should show merged asset count:\n{}",
        stdout
    );

    // Should show override count (even if 0)
    assert!(
        stdout.contains("overridden") || stdout.contains("Overridden"),
        "should show override count:\n{}",
        stdout
    );
}
