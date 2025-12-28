//! Deploy should display the effective source layer when no project layer exists.
//!
//! Regression test: when `.promptpack/` is missing in the project but global layers exist,
//! the deploy header should show the user layer path (e.g. `~/.calvin/.promptpack`) rather
//! than the non-existent project `.promptpack`.

mod common;

use common::*;

#[test]
fn deploy_header_uses_user_layer_as_source_when_project_promptpack_missing() {
    let env = TestEnv::builder()
        .without_project_promptpack()
        .with_user_layer_asset("global.md", SIMPLE_POLICY)
        .build();

    let result = env.run(&["deploy", "--yes", "--targets", "cursor"]);

    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    // Build this path using segments (not forward slashes) for Windows compatibility.
    let user_layer = env.home_path(".calvin").join(".promptpack");
    assert!(
        result
            .stdout
            .contains(&format!("Source: {}", user_layer.display())),
        "expected deploy header to show user layer as source:\n{}",
        result.stdout
    );
    assert!(
        !result.stdout.contains("Source: .promptpack"),
        "expected deploy header to not claim a project .promptpack source:\n{}",
        result.stdout
    );
}
