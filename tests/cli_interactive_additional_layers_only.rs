//! Integration test for interactive state detection with additional layers configured.

use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

#[test]
fn interactive_detects_additional_layers_without_project_promptpack() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    let home = project_dir.join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(home.join(".config/calvin")).unwrap();

    // Create an additional layer promptpack (team).
    let team_layer = project_dir.join("team/.promptpack");
    fs::create_dir_all(team_layer.join("policies")).unwrap();
    fs::write(
        team_layer.join("policies/rule.md"),
        r#"---
kind: policy
description: Team Rule
scope: project
targets: [cursor]
---
TEAM
"#,
    )
    .unwrap();

    // Configure additional layer via user config.
    fs::write(
        home.join(".config/calvin/config.toml"),
        format!(
            r#"
[sources]
additional_layers = ["{}"]
"#,
            team_layer.display()
        ),
    )
    .unwrap();

    let output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .args(["--json"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "calvin --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(
        json.get("state").and_then(|v| v.as_str()),
        Some("global_layers_only")
    );
    assert!(json["state_aliases"]
        .as_array()
        .is_some_and(|a| a.iter().any(|v| v.as_str() == Some("user_layer_only"))));
    assert_eq!(json["assets"]["total"], 1);
}
