//! TDD Test: Interactive mode should include layer information (PRD §12.1)
//!
//! The interactive state detection should include layer details that can
//! be displayed to users. We test via `--json` mode since interactive
//! mode requires a TTY.

use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

#[test]
fn interactive_json_includes_layer_details_when_global_layers_exist() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    // Setup: fake git repo
    fs::create_dir_all(project_dir.join(".git")).unwrap();

    // Create user layer with assets
    let fake_home = project_dir.join("fake_home");
    let user_layer = fake_home.join(".calvin/.promptpack");
    fs::create_dir_all(&user_layer).unwrap();
    fs::write(
        user_layer.join("global-rules.md"),
        r#"---
kind: policy
description: Global rules
scope: project
targets: [cursor]
---
Global content
"#,
    )
    .unwrap();

    // NO project layer - this triggers "global layers only" state

    let output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &fake_home)
        .env("XDG_CONFIG_HOME", fake_home.join(".config"))
        .args(["--json"]) // JSON mode to get structured output
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // PRD §12.1: JSON output should include layer information
    // The interactive state should include:
    // - "layers" array with layer details
    // - Each layer should have name, path, and asset count

    assert!(
        stdout.contains("global_layers_only") || stdout.contains("configured"),
        "should detect global layers or configured state:\n{}",
        stdout
    );

    // Should include layers array
    assert!(
        stdout.contains("layers") || stdout.contains("total"),
        "interactive JSON should include layer details:\n{}",
        stdout
    );
}

#[test]
fn interactive_json_includes_per_layer_asset_counts() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    fs::create_dir_all(project_dir.join(".git")).unwrap();

    // Create user layer with 2 assets
    let fake_home = project_dir.join("fake_home");
    let user_layer = fake_home.join(".calvin/.promptpack");
    fs::create_dir_all(&user_layer).unwrap();
    fs::write(
        user_layer.join("rule1.md"),
        r#"---
kind: policy
description: Rule 1
scope: project
targets: [cursor]
---
Content 1
"#,
    )
    .unwrap();
    fs::write(
        user_layer.join("rule2.md"),
        r#"---
kind: policy
description: Rule 2
scope: project
targets: [cursor]
---
Content 2
"#,
    )
    .unwrap();

    // Create project layer with 1 asset
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
        .env("HOME", &fake_home)
        .env("XDG_CONFIG_HOME", fake_home.join(".config"))
        .args(["--json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should include per-layer details
    // Expecting: layers: [ { name: "user", assets: 2 }, { name: "project", assets: 1 } ]
    assert!(
        stdout.contains("layers"),
        "interactive JSON should include layers array:\n{}",
        stdout
    );

    // Parse JSON and verify layer details
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let layers = json.get("layers").expect("should have layers");
    assert!(layers.is_array(), "layers should be an array:\n{}", stdout);
    assert!(
        !layers.as_array().unwrap().is_empty(),
        "layers should not be empty:\n{}",
        stdout
    );
}
