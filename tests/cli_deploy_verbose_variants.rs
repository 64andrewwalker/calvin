//! Variant tests for verbose deploy output
//!
//! These tests cover edge cases and boundary conditions for the verbose
//! deploy output feature (PRD §10.4, §12.2).

use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

/// Variant: Single layer (no overrides possible)
#[test]
fn verbose_deploy_single_layer_shows_no_overrides() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    fs::create_dir_all(project_dir.join(".git")).unwrap();

    // Only project layer, no user layer
    let fake_home = project_dir.join("fake_home");
    fs::create_dir_all(&fake_home).unwrap();

    let project_layer = project_dir.join(".promptpack");
    fs::create_dir_all(&project_layer).unwrap();
    fs::write(
        project_layer.join("single.md"),
        r#"---
kind: policy
description: Single asset
scope: project
targets: [cursor]
---
Single content
"#,
    )
    .unwrap();

    let output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &fake_home)
        .env("XDG_CONFIG_HOME", fake_home.join(".config"))
        .args(["deploy", "-v", "--yes", "--targets", "cursor"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "deploy failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Should show layer stack with 1 layer
    assert!(
        stdout.contains("1. [project]"),
        "should show single project layer:\n{}",
        stdout
    );

    // Should NOT show "Overrides:" section when there are none
    assert!(
        !stdout.contains("Overrides:"),
        "should not show Overrides section when there are no overrides:\n{}",
        stdout
    );

    // Should show "0 overridden" in summary
    assert!(
        stdout.contains("0 overridden"),
        "should show 0 overridden in summary:\n{}",
        stdout
    );
}

/// Variant: Empty project layer with user layer
#[test]
fn verbose_deploy_empty_project_layer_uses_user_assets() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    fs::create_dir_all(project_dir.join(".git")).unwrap();

    // User layer with asset
    let fake_home = project_dir.join("fake_home");
    let user_layer = fake_home.join(".calvin/.promptpack");
    fs::create_dir_all(&user_layer).unwrap();
    fs::write(
        user_layer.join("user-asset.md"),
        r#"---
kind: policy
description: User asset
scope: project
targets: [cursor]
---
User content
"#,
    )
    .unwrap();

    // Empty project layer
    let project_layer = project_dir.join(".promptpack");
    fs::create_dir_all(&project_layer).unwrap();
    // No files in project layer

    let output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &fake_home)
        .env("XDG_CONFIG_HOME", fake_home.join(".config"))
        .args(["deploy", "-v", "--yes", "--targets", "cursor"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "deploy failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Should show both layers
    assert!(
        stdout.contains("[project]") && stdout.contains("[user]"),
        "should show both layers even if project is empty:\n{}",
        stdout
    );

    // Project layer should show 0 assets
    assert!(
        stdout.contains("(0 assets)"),
        "project layer should show 0 assets:\n{}",
        stdout
    );

    // User layer should show 1 asset
    assert!(
        stdout.contains("(1 assets)"),
        "user layer should show 1 asset:\n{}",
        stdout
    );
}

/// Variant: Multiple overrides
#[test]
fn verbose_deploy_multiple_overrides() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    fs::create_dir_all(project_dir.join(".git")).unwrap();

    // User layer with multiple assets
    let fake_home = project_dir.join("fake_home");
    let user_layer = fake_home.join(".calvin/.promptpack");
    fs::create_dir_all(&user_layer).unwrap();
    fs::write(
        user_layer.join("policy-a.md"),
        "---\nid: policy-a\nkind: policy\ndescription: A\nscope: project\ntargets: [cursor]\n---\n",
    )
    .unwrap();
    fs::write(
        user_layer.join("policy-b.md"),
        "---\nid: policy-b\nkind: policy\ndescription: B\nscope: project\ntargets: [cursor]\n---\n",
    )
    .unwrap();
    fs::write(
        user_layer.join("policy-c.md"),
        "---\nid: policy-c\nkind: policy\ndescription: C\nscope: project\ntargets: [cursor]\n---\n",
    )
    .unwrap();

    // Project layer overrides 2 of them
    let project_layer = project_dir.join(".promptpack");
    fs::create_dir_all(&project_layer).unwrap();
    fs::write(
        project_layer.join("policy-a.md"),
        "---\nid: policy-a\nkind: policy\ndescription: A override\nscope: project\ntargets: [cursor]\n---\n",
    )
    .unwrap();
    fs::write(
        project_layer.join("policy-b.md"),
        "---\nid: policy-b\nkind: policy\ndescription: B override\nscope: project\ntargets: [cursor]\n---\n",
    )
    .unwrap();

    let output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &fake_home)
        .env("XDG_CONFIG_HOME", fake_home.join(".config"))
        .args(["deploy", "-v", "--yes", "--targets", "cursor"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());

    // Should show "2 overridden" in summary
    assert!(
        stdout.contains("2 overridden"),
        "should show 2 overridden:\n{}",
        stdout
    );

    // Should have Overrides section with both
    assert!(
        stdout.contains("Overrides:"),
        "should show Overrides section:\n{}",
        stdout
    );
    assert!(
        stdout.contains("policy-a") && stdout.contains("policy-b"),
        "should list both overridden assets:\n{}",
        stdout
    );
}

/// Variant: Layer stack format consistency with `calvin layers`
#[test]
fn verbose_deploy_layer_format_matches_layers_command() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    fs::create_dir_all(project_dir.join(".git")).unwrap();

    let fake_home = project_dir.join("fake_home");
    let user_layer = fake_home.join(".calvin/.promptpack");
    fs::create_dir_all(&user_layer).unwrap();
    fs::write(
        user_layer.join("test.md"),
        "---\nkind: policy\ndescription: T\nscope: project\ntargets: [cursor]\n---\n",
    )
    .unwrap();

    let project_layer = project_dir.join(".promptpack");
    fs::create_dir_all(&project_layer).unwrap();
    fs::write(
        project_layer.join("local.md"),
        "---\nkind: policy\ndescription: L\nscope: project\ntargets: [cursor]\n---\n",
    )
    .unwrap();

    // Run deploy -v
    let deploy_output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &fake_home)
        .env("XDG_CONFIG_HOME", fake_home.join(".config"))
        .args(["deploy", "-v", "--yes", "--targets", "cursor"])
        .output()
        .unwrap();

    // Run layers
    let layers_output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &fake_home)
        .env("XDG_CONFIG_HOME", fake_home.join(".config"))
        .args(["layers"])
        .output()
        .unwrap();

    let deploy_stdout = String::from_utf8_lossy(&deploy_output.stdout);
    let layers_stdout = String::from_utf8_lossy(&layers_output.stdout);

    // Both should use "highest priority first" wording
    assert!(
        deploy_stdout.contains("highest priority first"),
        "deploy -v should say 'highest priority first':\n{}",
        deploy_stdout
    );
    assert!(
        layers_stdout.contains("highest priority first"),
        "layers should say 'highest priority first':\n{}",
        layers_stdout
    );

    // Both should use numbered format
    assert!(
        deploy_stdout.contains("2. [project]") && deploy_stdout.contains("1. [user]"),
        "deploy -v should use numbered format:\n{}",
        deploy_stdout
    );
    assert!(
        layers_stdout.contains("2. [project]") && layers_stdout.contains("1. [user]"),
        "layers should use numbered format:\n{}",
        layers_stdout
    );
}
