//! Integration tests for Phase 3 CLI layer flags (`--layer`, `--no-user-layer`, `--no-additional-layers`)

use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn write_asset(promptpack: &std::path::Path, filename: &str, body: &str) {
    fs::create_dir_all(promptpack).unwrap();
    fs::write(promptpack.join(filename), body).unwrap();
}

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

#[test]
fn deploy_uses_additional_layers_from_user_config_and_layer_flag() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();
    fs::create_dir_all(project_dir.join(".git")).unwrap();

    let home = project_dir.join("home");
    fs::create_dir_all(&home).unwrap();

    // User layer
    let user_layer = home.join(".calvin/.promptpack");
    write_asset(
        &user_layer,
        "user-only.md",
        r#"---
kind: policy
description: User only
scope: project
targets: [cursor]
---
USER ONLY
"#,
    );

    // Additional layer configured via user config
    let team_layer_root = project_dir.join("team_layer");
    let team_layer = team_layer_root.join(".promptpack");
    write_asset(
        &team_layer,
        "team-only.md",
        r#"---
kind: policy
description: Team only
scope: project
targets: [cursor]
---
TEAM ONLY
"#,
    );

    fs::create_dir_all(home.join(".config/calvin")).unwrap();
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

    // Project layer
    let project_layer = project_dir.join(".promptpack");
    write_asset(
        &project_layer,
        "project-only.md",
        r#"---
kind: policy
description: Project only
scope: project
targets: [cursor]
---
PROJECT ONLY
"#,
    );

    // Extra CLI-provided layer
    let cli_layer_root = project_dir.join("cli_layer");
    let cli_layer = cli_layer_root.join(".promptpack");
    write_asset(
        &cli_layer,
        "cli-only.md",
        r#"---
kind: policy
description: CLI only
scope: project
targets: [cursor]
---
CLI ONLY
"#,
    );

    let output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .args([
            "deploy",
            "--yes",
            "--targets",
            "cursor",
            "--layer",
            cli_layer.to_string_lossy().as_ref(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "deploy failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(project_dir.join(".cursor/rules/user-only/RULE.md").exists());
    assert!(project_dir.join(".cursor/rules/team-only/RULE.md").exists());
    assert!(project_dir
        .join(".cursor/rules/project-only/RULE.md")
        .exists());
    assert!(project_dir.join(".cursor/rules/cli-only/RULE.md").exists());
}

#[test]
fn deploy_can_disable_user_and_additional_layers_with_flags() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();
    fs::create_dir_all(project_dir.join(".git")).unwrap();

    let home = project_dir.join("home");
    fs::create_dir_all(&home).unwrap();

    // User layer
    let user_layer = home.join(".calvin/.promptpack");
    write_asset(
        &user_layer,
        "user-only.md",
        r#"---
kind: policy
description: User only
scope: project
targets: [cursor]
---
USER ONLY
"#,
    );

    // Additional layer configured via user config
    let team_layer_root = project_dir.join("team_layer");
    let team_layer = team_layer_root.join(".promptpack");
    write_asset(
        &team_layer,
        "team-only.md",
        r#"---
kind: policy
description: Team only
scope: project
targets: [cursor]
---
TEAM ONLY
"#,
    );

    fs::create_dir_all(home.join(".config/calvin")).unwrap();
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

    // Project layer
    let project_layer = project_dir.join(".promptpack");
    write_asset(
        &project_layer,
        "project-only.md",
        r#"---
kind: policy
description: Project only
scope: project
targets: [cursor]
---
PROJECT ONLY
"#,
    );

    let output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .args([
            "deploy",
            "--yes",
            "--targets",
            "cursor",
            "--no-user-layer",
            "--no-additional-layers",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "deploy failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        !project_dir.join(".cursor/rules/user-only/RULE.md").exists(),
        "expected user layer to be disabled"
    );
    assert!(
        !project_dir.join(".cursor/rules/team-only/RULE.md").exists(),
        "expected additional layers to be disabled"
    );
    assert!(project_dir
        .join(".cursor/rules/project-only/RULE.md")
        .exists());
}
