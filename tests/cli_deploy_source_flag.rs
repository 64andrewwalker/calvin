//! Integration tests for Phase 3 `deploy --source`

mod common;

use common::*;

#[test]
fn deploy_source_flag_overrides_project_layer_directory() {
    let env = TestEnv::builder()
        .with_project_asset("should-not-deploy.md", SIMPLE_POLICY)
        .with_project_config(CONFIG_DEPLOY_PROJECT)
        .build();

    let source = env.project_path("custom-pack");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(
        source.join("policy.md"),
        r#"---
kind: policy
description: Policy
scope: project
targets: [cursor]
---
POLICY
"#,
    )
    .unwrap();

    let source_arg = source.to_string_lossy().to_string();
    let result = env.run(&[
        "deploy",
        "--source",
        &source_arg,
        "--yes",
        "--targets",
        "cursor",
    ]);

    assert!(
        result.success,
        "deploy --source failed:\n{}",
        result.combined_output()
    );

    assert_deployed!(&env, ".cursor/rules/policy/RULE.md");
    assert_not_deployed!(&env, ".cursor/rules/should-not-deploy/RULE.md");
}
