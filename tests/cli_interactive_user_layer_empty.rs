//! Integration tests for interactive mode state detection.
//!
//! These tests focus on multi-layer behavior described in
//! `docs/archive/multi-layer-implementation/multi-layer-promptpack-prd.md`:
//! an empty but existing user layer is still a valid layer.

use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

#[test]
fn interactive_reports_user_layer_even_when_empty() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path().join("project");
    fs::create_dir_all(&project_dir).unwrap();

    let fake_home = dir.path().join("home");
    fs::create_dir_all(&fake_home).unwrap();
    fs::create_dir_all(fake_home.join(".config")).unwrap();

    // Create an empty user layer directory with only config.toml.
    // PRD: "Layer directory exists but is empty → valid layer, 0 assets".
    let user_layer = fake_home.join(".calvin/.promptpack");
    fs::create_dir_all(&user_layer).unwrap();
    fs::write(
        user_layer.join("config.toml"),
        r#"
[targets]
enabled = ["cursor"]
"#,
    )
    .unwrap();

    let output = Command::new(bin())
        .current_dir(&project_dir)
        .env("HOME", &fake_home)
        .env("XDG_CONFIG_HOME", fake_home.join(".config"))
        .args(["--json"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "calvin --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["event"], "interactive");
    assert_eq!(v["state"], "global_layers_only");
    assert!(v["state_aliases"]
        .as_array()
        .is_some_and(|a| a.iter().any(|v| v.as_str() == Some("user_layer_only"))));
    assert_eq!(v["assets"]["total"], 0);
}
