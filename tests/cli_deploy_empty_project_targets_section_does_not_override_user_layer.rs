//! Regression test: an *empty* `[targets]` section in the project layer config.toml
//! must not override a valid user-layer `[targets] enabled = [...]`.
//!
//! Real-world trigger: generated templates often include `[targets]` with only commented
//! examples, which produces an empty TOML table and previously caused Calvin to fall back
//! to "all targets" (mis-deploying).

mod common;

use common::*;

#[test]
fn deploy_does_not_let_empty_project_targets_section_override_user_layer_targets() {
    let env = TestEnv::builder()
        .with_user_promptpack_config(
            r#"
[targets]
enabled = ["antigravity"]
"#,
        )
        .with_project_config(
            r#"
[targets]
# enabled = ["cursor"]
"#,
        )
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
        .build();

    // Deploy without `--targets` (should use merged layer config).
    let result = env.run(&["deploy", "--yes"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    // Antigravity outputs should exist...
    assert_deployed!(&env, ".agent/rules/test.md");
    // ...but Cursor outputs should not (targets limited to antigravity).
    assert_not_deployed!(&env, ".cursor/rules/test/RULE.md");
}
