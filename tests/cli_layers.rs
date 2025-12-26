//! Integration tests for `calvin layers` (Phase 4)

mod common;

use std::fs;
use std::process::Command;

use common::WindowsCompatExt;
use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

fn write_asset(promptpack: &std::path::Path, filename: &str, body: &str) {
    fs::create_dir_all(promptpack).unwrap();
    fs::write(promptpack.join(filename), body).unwrap();
}

#[test]
fn layers_lists_project_user_and_additional_layers() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();
    fs::create_dir_all(project_dir.join(".git")).unwrap();

    let home = project_dir.join("home");
    fs::create_dir_all(&home).unwrap();

    // User layer
    let user_layer = home.join(".calvin/.promptpack");
    write_asset(
        &user_layer,
        "user.md",
        r#"---
kind: policy
description: User
scope: project
targets: [cursor]
---
USER
"#,
    );

    // Additional layer via user config
    let team_layer_root = project_dir.join("team_layer");
    let team_layer = team_layer_root.join(".promptpack");
    write_asset(
        &team_layer,
        "team.md",
        r#"---
kind: policy
description: Team
scope: project
targets: [cursor]
---
TEAM
"#,
    );
    fs::create_dir_all(home.join(".config/calvin")).unwrap();
    // Use forward slashes for cross-platform TOML compatibility (backslashes are escape chars)
    let team_layer_str = team_layer.to_string_lossy().replace('\\', "/");
    fs::write(
        home.join(".config/calvin/config.toml"),
        format!(
            r#"
[sources]
additional_layers = ["{}"]
"#,
            team_layer_str
        ),
    )
    .unwrap();

    // Project layer
    let project_layer = project_dir.join(".promptpack");
    write_asset(
        &project_layer,
        "project.md",
        r#"---
kind: policy
description: Project
scope: project
targets: [cursor]
---
PROJECT
"#,
    );

    let output = Command::new(bin())
        .current_dir(project_dir)
        .with_test_home(&home)
        .args(["--json", "layers"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "layers failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let layers = json.get("layers").and_then(|v| v.as_array()).unwrap();
    let names: Vec<&str> = layers
        .iter()
        .filter_map(|l| l.get("name").and_then(|v| v.as_str()))
        .collect();
    assert!(names.contains(&"user"), "expected user layer in {names:?}");
    assert!(
        names.iter().any(|n| n.starts_with("custom-")),
        "expected custom layer in {names:?}"
    );
    assert!(
        names.contains(&"project"),
        "expected project layer in {names:?}"
    );
}
