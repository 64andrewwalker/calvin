//! Variant tests for verbose deploy output
//!
//! These tests cover edge cases and boundary conditions for the verbose
//! deploy output feature (PRD §10.4, §12.2).

mod common;

use common::*;

/// Variant: Single layer (no overrides possible)
#[test]
fn verbose_deploy_single_layer_shows_no_overrides() {
    let env = TestEnv::builder()
        .with_project_asset(
            "single.md",
            r#"---
kind: policy
description: Single asset
scope: project
targets: [cursor]
---
Single content
"#,
        )
        .build();

    let result = env.run(&["deploy", "-v", "--yes", "--targets", "cursor"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    // Should show layer stack with 1 layer
    assert!(
        result.stdout.contains("1. [project]"),
        "should show single project layer:\n{}",
        result.stdout
    );

    // Should NOT show "Overrides:" section when there are none
    assert!(
        !result.stdout.contains("Overrides:"),
        "should not show Overrides section when there are no overrides:\n{}",
        result.stdout
    );

    // Should show "0 overridden" in summary
    assert!(
        result.stdout.contains("0 overridden"),
        "should show 0 overridden in summary:\n{}",
        result.stdout
    );
}

/// Variant: Empty project layer with user layer
#[test]
fn verbose_deploy_empty_project_layer_uses_user_assets() {
    let env = TestEnv::builder()
        .with_user_asset(
            "user-asset.md",
            r#"---
kind: policy
description: User asset
scope: project
targets: [cursor]
---
User content
"#,
        )
        .build();

    let result = env.run(&["deploy", "-v", "--yes", "--targets", "cursor"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    // Should show both layers
    assert!(
        result.stdout.contains("[project]") && result.stdout.contains("[user]"),
        "should show both layers even if project is empty:\n{}",
        result.stdout
    );

    // Project layer should show 0 assets
    assert!(
        result.stdout.contains("(0 assets)"),
        "project layer should show 0 assets:\n{}",
        result.stdout
    );

    // User layer should show 1 asset
    assert!(
        result.stdout.contains("(1 assets)"),
        "user layer should show 1 asset:\n{}",
        result.stdout
    );
}

/// Variant: Multiple overrides
#[test]
fn verbose_deploy_multiple_overrides() {
    let env = TestEnv::builder()
        .with_user_asset(
            "policy-a.md",
            "---\nid: policy-a\nkind: policy\ndescription: A\nscope: project\ntargets: [cursor]\n---\n",
        )
        .with_user_asset(
            "policy-b.md",
            "---\nid: policy-b\nkind: policy\ndescription: B\nscope: project\ntargets: [cursor]\n---\n",
        )
        .with_user_asset(
            "policy-c.md",
            "---\nid: policy-c\nkind: policy\ndescription: C\nscope: project\ntargets: [cursor]\n---\n",
        )
        .with_project_asset(
            "policy-a.md",
            "---\nid: policy-a\nkind: policy\ndescription: A override\nscope: project\ntargets: [cursor]\n---\n",
        )
        .with_project_asset(
            "policy-b.md",
            "---\nid: policy-b\nkind: policy\ndescription: B override\nscope: project\ntargets: [cursor]\n---\n",
        )
        .build();

    let result = env.run(&["deploy", "-v", "--yes", "--targets", "cursor"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    // Should show "2 overridden" in summary
    assert!(
        result.stdout.contains("2 overridden"),
        "should show 2 overridden:\n{}",
        result.stdout
    );

    // Should have Overrides section with both
    assert!(
        result.stdout.contains("Overrides:"),
        "should show Overrides section:\n{}",
        result.stdout
    );
    assert!(
        result.stdout.contains("policy-a") && result.stdout.contains("policy-b"),
        "should list both overridden assets:\n{}",
        result.stdout
    );
}

/// Variant: Layer stack format consistency with `calvin layers`
#[test]
fn verbose_deploy_layer_format_matches_layers_command() {
    let env = TestEnv::builder()
        .with_user_asset(
            "test.md",
            "---\nkind: policy\ndescription: T\nscope: project\ntargets: [cursor]\n---\n",
        )
        .with_project_asset(
            "local.md",
            "---\nkind: policy\ndescription: L\nscope: project\ntargets: [cursor]\n---\n",
        )
        .build();

    // Run deploy -v
    let deploy_result = env.run(&["deploy", "-v", "--yes", "--targets", "cursor"]);
    assert!(
        deploy_result.success,
        "deploy failed:\n{}",
        deploy_result.combined_output()
    );

    // Run layers
    let layers_result = env.run(&["layers"]);
    assert!(
        layers_result.success,
        "layers failed:\n{}",
        layers_result.combined_output()
    );

    // Both should use "highest priority first" wording
    assert!(
        deploy_result.stdout.contains("highest priority first"),
        "deploy -v should say 'highest priority first':\n{}",
        deploy_result.stdout
    );
    assert!(
        layers_result.stdout.contains("highest priority first"),
        "layers should say 'highest priority first':\n{}",
        layers_result.stdout
    );

    // Both should use numbered format
    assert!(
        deploy_result.stdout.contains("2. [project]") && deploy_result.stdout.contains("1. [user]"),
        "deploy -v should use numbered format:\n{}",
        deploy_result.stdout
    );
    assert!(
        layers_result.stdout.contains("2. [project]") && layers_result.stdout.contains("1. [user]"),
        "layers should use numbered format:\n{}",
        layers_result.stdout
    );
}
