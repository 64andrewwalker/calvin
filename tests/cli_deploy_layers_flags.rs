//! Integration tests for Phase 3 CLI layer flags (`--layer`, `--no-user-layer`, `--no-additional-layers`)

mod common;

use common::*;

#[test]
fn deploy_uses_additional_layers_from_user_config_and_layer_flag() {
    let env = TestEnv::builder()
        .with_user_asset("user-only.md", SIMPLE_POLICY)
        .with_project_asset("project-only.md", SIMPLE_POLICY)
        .build();

    // Additional layer configured via user config
    env.write_project_file("team_layer/.promptpack/team-only.md", SIMPLE_POLICY);
    let team_layer = env.project_path("team_layer/.promptpack");
    // Use forward slashes for cross-platform TOML compatibility (backslashes are escape chars)
    let team_layer_str = team_layer.to_string_lossy().replace('\\', "/");
    let home_config = format!(
        r#"
[sources]
additional_layers = ["{}"]
"#,
        team_layer_str
    );
    env.write_home_file(".config/calvin/config.toml", &home_config);

    // Extra CLI-provided layer
    env.write_project_file("cli_layer/.promptpack/cli-only.md", SIMPLE_POLICY);
    let cli_layer = env.project_path("cli_layer/.promptpack");
    let cli_layer_arg = cli_layer.to_string_lossy().to_string();

    let result = env.run(&[
        "deploy",
        "--yes",
        "--targets",
        "cursor",
        "--layer",
        &cli_layer_arg,
    ]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert_deployed!(&env, ".cursor/rules/user-only/RULE.md");
    assert_deployed!(&env, ".cursor/rules/team-only/RULE.md");
    assert_deployed!(&env, ".cursor/rules/project-only/RULE.md");
    assert_deployed!(&env, ".cursor/rules/cli-only/RULE.md");
}

#[test]
fn deploy_can_disable_user_and_additional_layers_with_flags() {
    let env = TestEnv::builder()
        .with_user_asset("user-only.md", SIMPLE_POLICY)
        .with_project_asset("project-only.md", SIMPLE_POLICY)
        .build();

    // Additional layer configured via user config
    env.write_project_file("team_layer/.promptpack/team-only.md", SIMPLE_POLICY);
    let team_layer = env.project_path("team_layer/.promptpack");
    // Use forward slashes for cross-platform TOML compatibility (backslashes are escape chars)
    let team_layer_str = team_layer.to_string_lossy().replace('\\', "/");
    let home_config = format!(
        r#"
[sources]
additional_layers = ["{}"]
"#,
        team_layer_str
    );
    env.write_home_file(".config/calvin/config.toml", &home_config);

    let result = env.run(&[
        "deploy",
        "--yes",
        "--targets",
        "cursor",
        "--no-user-layer",
        "--no-additional-layers",
    ]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert_not_deployed!(&env, ".cursor/rules/user-only/RULE.md");
    assert_not_deployed!(&env, ".cursor/rules/team-only/RULE.md");
    assert_deployed!(&env, ".cursor/rules/project-only/RULE.md");
}
