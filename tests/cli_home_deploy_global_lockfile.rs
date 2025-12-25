//! Regression tests for global (home) deployments.
//!
//! When deploying to home (`calvin deploy --home`), the lockfile and registry semantics must not
//! pollute the current working directory. Home deployments are global, so they should be tracked
//! under `~/.calvin/` and be cleanable from any directory.

mod common;

use common::*;

#[test]
fn deploy_home_uses_global_lockfile_and_clean_home_from_any_dir() {
    let env = TestEnv::builder()
        .with_project_asset("policy.md", SIMPLE_POLICY)
        .build();

    let deploy = env.run(&["deploy", "--home", "--yes", "--targets", "cursor"]);
    assert!(
        deploy.success,
        "deploy --home failed:\n{}",
        deploy.combined_output()
    );

    // Home deploy should not create a project lockfile in the current directory.
    assert!(
        !env.project_path("calvin.lock").exists(),
        "did not expect project lockfile for home deploy"
    );

    // Home deploy should create a global lockfile under ~/.calvin.
    let global_lockfile = env.home_path(".calvin/calvin.lock");
    assert!(
        global_lockfile.exists(),
        "expected global lockfile at {}",
        global_lockfile.display()
    );

    // Home deploy should not register the project in the global registry.
    let registry_path = env.home_path(".calvin/registry.toml");
    assert!(
        !registry_path.exists(),
        "did not expect registry update for home deploy"
    );

    // Verify an expected output exists under the fake home.
    let deployed_file = env.home_path(".cursor/rules/policy/RULE.md");
    assert!(
        deployed_file.exists(),
        "expected home deployed file at {}",
        deployed_file.display()
    );

    // Clean home deployments should work from any directory (even without a project .promptpack).
    let clean = env.run_from(env.home_dir.path(), &["clean", "--home", "--yes"]);
    assert!(
        clean.success,
        "clean --home failed:\n{}",
        clean.combined_output()
    );

    assert!(
        !deployed_file.exists(),
        "expected clean --home to delete {}",
        deployed_file.display()
    );

    // If all tracked home files are deleted, the global lockfile should be removed.
    assert!(
        !global_lockfile.exists(),
        "expected empty global lockfile to be deleted"
    );
}
