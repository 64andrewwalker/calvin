//! Regression test: `calvin deploy --project` must override `[deploy].target = "home"`.
//!
//! This fixes the UX trap where a project config defaults deploys to home,
//! but the user wants a one-off deploy to the project without editing config.

mod common;

use common::*;

#[test]
fn deploy_project_flag_overrides_home_config_without_modifying_config() {
    let env = TestEnv::builder()
        .with_project_asset(
            "test.md",
            r#"---
kind: policy
description: Test
scope: project
targets: [cursor]
---
TEST
"#,
        )
        .with_project_config(CONFIG_DEPLOY_HOME)
        .build();

    let result = env.run(&["deploy", "--project", "--yes"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    // Must deploy to the project, not HOME.
    assert!(
        env.project_path(".cursor").exists(),
        "expected project .cursor/"
    );
    assert!(
        !env.home_path(".cursor").exists(),
        "expected no home .cursor/ when using --project override"
    );

    // CLI override must not rewrite the user's persisted preference.
    let config = std::fs::read_to_string(env.project_path(".promptpack/config.toml")).unwrap();
    assert!(
        config.contains("target = \"home\""),
        "expected config to remain home; got:\n{config}"
    );
}
