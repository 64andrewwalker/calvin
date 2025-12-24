use std::fs;
use std::process::Command;

use serde_json::Value;
use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

fn write_asset(promptpack: &std::path::Path, filename: &str) {
    fs::create_dir_all(promptpack).unwrap();
    fs::write(
        promptpack.join(filename),
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
}

#[test]
fn check_all_layers_emits_layer_stack_check_event() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();
    fs::create_dir_all(project_dir.join(".git")).unwrap();

    let home = project_dir.join("home");
    fs::create_dir_all(&home).unwrap();

    let user_layer = home.join(".calvin/.promptpack");
    write_asset(&user_layer, "user.md");

    let project_layer = project_dir.join(".promptpack");
    write_asset(&project_layer, "project.md");

    let output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .args(["check", "--all-layers", "--json"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "check --all-layers failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut saw_layer_check = false;
    for line in stdout.lines().filter(|l| !l.trim().is_empty()) {
        let v: Value = serde_json::from_str(line).unwrap();
        if v.get("event").and_then(|v| v.as_str()) == Some("check")
            && v.get("platform").and_then(|v| v.as_str()) == Some("layers")
            && v.get("name").and_then(|v| v.as_str()) == Some("layer_stack")
        {
            saw_layer_check = true;
        }
    }

    assert!(
        saw_layer_check,
        "expected layer_stack check event\n{stdout}"
    );
}
