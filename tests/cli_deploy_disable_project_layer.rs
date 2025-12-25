//! Integration test for `sources.disable_project_layer`.
//!
//! PRD: user config may set `sources.disable_project_layer = true` for debugging/special cases.
//! When enabled, Calvin should ignore the project layer when resolving the layer stack.

mod common;

use common::*;

#[test]
fn deploy_can_disable_project_layer_via_user_config() {
    let env = TestEnv::builder()
        .with_project_asset(
            "project-only.md",
            r#"---
kind: policy
description: Project only
scope: project
targets: [cursor]
---
PROJECT
"#,
        )
        .with_user_asset(
            "user-only.md",
            r#"---
kind: policy
description: User only
scope: project
targets: [cursor]
---
USER
"#,
        )
        .with_home_config(
            r#"
[sources]
disable_project_layer = true
"#,
        )
        .build();

    let result = env.run(&["deploy", "--yes", "--targets", "cursor"]);

    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    // User layer asset should be deployed.
    assert_deployed!(&env, ".cursor/rules/user-only/RULE.md");
    // Project layer asset should be ignored.
    assert_not_deployed!(&env, ".cursor/rules/project-only/RULE.md");
}
