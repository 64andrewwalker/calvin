use std::fs;
use std::process::Command;

use serde_json::Value;
use tempfile::tempdir;

#[test]
fn test_deploy_json_emits_ndjson_event_stream() {
    let dir = tempdir().unwrap();
    let fake_home = dir.path().join("fake_home");
    fs::create_dir_all(&fake_home).unwrap();

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
    let output = Command::new(bin)
        .current_dir(dir.path())
        .env("HOME", &fake_home)
        .env("XDG_CONFIG_HOME", fake_home.join(".config"))
        .args([
            "deploy",
            "--json",
            "--source",
            source.to_str().unwrap(),
            "--targets",
            "claude-code",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(
        lines.len() > 1,
        "expected NDJSON (multiple lines), got:\n{stdout}"
    );

    let first: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["event"], "start");
    assert_eq!(first["command"], "deploy");

    let last: Value = serde_json::from_str(lines[lines.len() - 1]).unwrap();
    assert_eq!(last["event"], "complete");
    assert_eq!(last["command"], "deploy");

    assert!(
        lines.iter().any(|l| {
            serde_json::from_str::<Value>(l)
                .ok()
                .is_some_and(|v| v["event"] == "item_written")
        }),
        "expected at least one item_written event, got:\n{stdout}"
    );
}
