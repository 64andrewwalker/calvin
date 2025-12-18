use std::fs;
use std::process::Command;

use tempfile::tempdir;

#[test]
fn test_deploy_yes_overwrites_conflicts() {
    let dir = tempdir().unwrap();

    // Avoid the "not a git repository" interactive confirmation prompt.
    fs::create_dir_all(dir.path().join(".git")).unwrap();

    let source = dir.path().join(".promptpack");
    fs::create_dir_all(source.join("actions")).unwrap();
    fs::write(
        source.join("actions/test.md"),
        r#"---
description: Test action
kind: action
targets: [claude-code]
---
# Test Action

Hello world.
"#,
    )
    .unwrap();

    let bin = env!("CARGO_BIN_EXE_calvin");

    // First deploy: creates lockfile + output.
    let status = Command::new(bin)
        .current_dir(dir.path())
        .args([
            "deploy",
            "--source",
            source.to_str().unwrap(),
            "--targets",
            "claude-code",
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let output_path = dir.path().join(".claude/commands/test.md");
    let original = fs::read_to_string(&output_path).unwrap();

    // Simulate a user modification.
    fs::write(&output_path, "USER MODIFIED").unwrap();

    // Second deploy with --yes should overwrite the conflict (no prompt).
    let status = Command::new(bin)
        .current_dir(dir.path())
        .args([
            "deploy",
            "--source",
            source.to_str().unwrap(),
            "--targets",
            "claude-code",
            "--yes",
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let after = fs::read_to_string(&output_path).unwrap();
    assert_eq!(after, original);
}
