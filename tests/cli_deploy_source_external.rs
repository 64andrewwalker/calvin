//! Integration tests for `--source` semantics (multi-layer Phase 3).
//!
//! PRD: `--source PATH` replaces the project layer path, but should not change the project root
//! (outputs + lockfile should still be anchored to the current project directory).

mod common;

use common::*;

#[test]
fn deploy_with_external_source_writes_lockfile_to_project_root() {
    let env = TestEnv::builder().without_project_promptpack().build();

    // "External" promptpack (not at ./ .promptpack).
    env.write_project_file("shared/.promptpack/solo.md", SIMPLE_POLICY);

    let external_root = env.project_path("shared");
    let external_promptpack = env.project_path("shared/.promptpack");
    let source_arg = external_promptpack.to_string_lossy().to_string();
    let result = env.run(&[
        "deploy",
        "--yes",
        "--targets",
        "cursor",
        "--source",
        &source_arg,
    ]);

    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    // Output should be written to the project root (relative paths).
    assert_deployed!(&env, ".cursor/rules/solo/RULE.md");

    // Lockfile should be in the project root, not next to the external promptpack.
    assert!(
        env.project_path("calvin.lock").exists(),
        "expected lockfile at project root"
    );
    assert!(
        !external_root.join("calvin.lock").exists(),
        "did not expect lockfile next to external promptpack"
    );
}
