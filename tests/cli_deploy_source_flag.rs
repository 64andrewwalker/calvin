//! Integration tests for Phase 3 `deploy --source`

use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

#[test]
fn deploy_source_flag_overrides_project_layer_directory() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();
    fs::create_dir_all(project_dir.join(".git")).unwrap();

    let source = project_dir.join("custom-pack");
    fs::create_dir_all(&source).unwrap();

    fs::write(
        source.join("policy.md"),
        r#"---
kind: policy
description: Policy
scope: project
targets: [cursor]
---
POLICY
"#,
    )
    .unwrap();

    let output = Command::new(bin())
        .current_dir(project_dir)
        .args([
            "deploy",
            "--source",
            source.to_string_lossy().as_ref(),
            "--yes",
            "--targets",
            "cursor",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "deploy --source failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(project_dir.join(".cursor/rules/policy/RULE.md").exists());
}
