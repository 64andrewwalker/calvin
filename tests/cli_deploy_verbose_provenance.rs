//! TDD Test: Verbose deploy should show asset provenance (PRD §10.4)
//!
//! Verbosity levels:
//! - `-v`: Layer stack + asset summary + overrides
//! - `-vv`: Layer stack + full asset provenance + overrides

use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

#[test]
fn verbose_deploy_shows_layer_stack_with_asset_counts() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    // Setup: fake git repo to avoid interactive prompt
    fs::create_dir_all(project_dir.join(".git")).unwrap();

    // Create user layer with one asset
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

    // Create project layer with one asset
    let project_layer = project_dir.join(".promptpack");
    fs::create_dir_all(&project_layer).unwrap();
    fs::write(
        project_layer.join("local-rules.md"),
        r#"---
kind: policy
description: Local rules
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
        .args([
            "deploy",
            "-v", // verbose
            "--yes",
            "--targets",
            "cursor",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "deploy failed:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    // PRD §10.4: Layer stack should show asset counts
    // Expected: "Layer Stack" or "Layers:" with "(N assets)" format
    assert!(
        stdout.contains("assets") && (stdout.contains("Layer") || stdout.contains("layer")),
        "verbose output should show layer stack with asset counts:\n{}",
        stdout
    );
}

#[test]
fn verbose_deploy_shows_asset_provenance() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    fs::create_dir_all(project_dir.join(".git")).unwrap();

    // Create user layer
    let fake_home = project_dir.join("fake_home");
    let user_layer = fake_home.join(".calvin/.promptpack");
    fs::create_dir_all(&user_layer).unwrap();
    fs::write(
        user_layer.join("shared-style.md"),
        r#"---
kind: policy
description: Shared style
scope: project
targets: [cursor]
---
Shared style content
"#,
    )
    .unwrap();

    // Create project layer
    let project_layer = project_dir.join(".promptpack");
    fs::create_dir_all(&project_layer).unwrap();
    fs::write(
        project_layer.join("project-action.md"),
        r#"---
kind: action
description: Project action
scope: project
targets: [cursor]
---
Project action content
"#,
    )
    .unwrap();

    let output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &fake_home)
        .env("XDG_CONFIG_HOME", fake_home.join(".config"))
        .args([
            "deploy",
            "-v", // verbose
            "--yes",
            "--targets",
            "cursor",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "deploy failed:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    // PRD §10.4: At -v level, should show asset summary
    // Full provenance list is shown at -vv
    assert!(
        stdout.contains("Assets:") || stdout.contains("merged"),
        "verbose output should show asset summary:\n{}",
        stdout
    );
}

#[test]
fn verbose_deploy_shows_override_info() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    fs::create_dir_all(project_dir.join(".git")).unwrap();

    // Create user layer with an asset
    let fake_home = project_dir.join("fake_home");
    let user_layer = fake_home.join(".calvin/.promptpack");
    fs::create_dir_all(&user_layer).unwrap();
    fs::write(
        user_layer.join("code-style.md"),
        r#"---
id: code-style
kind: policy
description: Code style (user)
scope: project
targets: [cursor]
---
User code style
"#,
    )
    .unwrap();

    // Create project layer with SAME ID (should override)
    let project_layer = project_dir.join(".promptpack");
    fs::create_dir_all(&project_layer).unwrap();
    fs::write(
        project_layer.join("code-style.md"),
        r#"---
id: code-style
kind: policy
description: Code style (project)
scope: project
targets: [cursor]
---
Project code style
"#,
    )
    .unwrap();

    let output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &fake_home)
        .env("XDG_CONFIG_HOME", fake_home.join(".config"))
        .args([
            "deploy",
            "-v", // verbose
            "--yes",
            "--targets",
            "cursor",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "deploy failed:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    // PRD §12.2: Override Confirmation should be shown
    // Should indicate that project overrides user
    assert!(
        stdout.contains("override") || stdout.contains("Override"),
        "verbose output should show override information when assets are overridden:\n{}",
        stdout
    );

    // Should mention the asset that was overridden
    assert!(
        stdout.contains("code-style"),
        "verbose output should mention the overridden asset name:\n{}",
        stdout
    );
}

#[test]
fn very_verbose_deploy_shows_full_provenance_list() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    fs::create_dir_all(project_dir.join(".git")).unwrap();

    // Create user layer
    let fake_home = project_dir.join("fake_home");
    let user_layer = fake_home.join(".calvin/.promptpack");
    fs::create_dir_all(&user_layer).unwrap();
    fs::write(
        user_layer.join("user-rule.md"),
        r#"---
kind: policy
description: User rule
scope: project
targets: [cursor]
---
User content
"#,
    )
    .unwrap();

    // Create project layer
    let project_layer = project_dir.join(".promptpack");
    fs::create_dir_all(&project_layer).unwrap();
    fs::write(
        project_layer.join("project-rule.md"),
        r#"---
kind: policy
description: Project rule
scope: project
targets: [cursor]
---
Project content
"#,
    )
    .unwrap();

    let output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &fake_home)
        .env("XDG_CONFIG_HOME", fake_home.join(".config"))
        .args([
            "deploy",
            "-vv", // very verbose
            "--yes",
            "--targets",
            "cursor",
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "deploy failed:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    // At -vv, should show "Asset Provenance" with full list
    assert!(
        stdout.contains("Asset Provenance") || stdout.contains("Provenance"),
        "very verbose output should show Asset Provenance section:\n{}",
        stdout
    );

    // Should list individual assets with their source
    assert!(
        stdout.contains("user-rule") && stdout.contains("project-rule"),
        "very verbose output should list individual asset names:\n{}",
        stdout
    );

    // Should show source info (← notation)
    assert!(
        stdout.contains("←") || stdout.contains("<-"),
        "very verbose output should show source arrow notation:\n{}",
        stdout
    );
}
