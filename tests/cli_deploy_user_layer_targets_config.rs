//! Integration tests for multi-layer config.toml merge behavior.
//!
//! PRD: `~/.calvin/.promptpack/config.toml` participates in config merge across layers.
//! In particular, `[targets] enabled = [...]` from the user layer should affect deploy
//! when the project does not explicitly set targets.

mod common;

use common::*;

#[test]
fn deploy_respects_targets_enabled_from_user_layer_promptpack_config() {
    let env = TestEnv::builder()
        .with_project_asset(
            "test.md",
            r#"---
kind: policy
description: Test
scope: project
targets: [antigravity, cursor]
---
TEST
"#,
        )
        // This test requires the project layer to NOT define targets, so the user-layer
        // promptpack config participates in target selection.
        .without_project_config_file()
        .with_user_promptpack_config(
            r#"
[targets]
enabled = ["antigravity"]
"#,
        )
        .build();

    // Deploy without `--targets` (should use merged layer config).
    let result = env.run(&["deploy", "--yes"]);

    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    // Antigravity outputs should exist...
    assert!(env.project_path(".agent/rules/test.md").exists());
    // ...but Cursor outputs should not (targets limited to antigravity).
    assert!(!env.project_path(".cursor/rules/test/RULE.md").exists());
}
