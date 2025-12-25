//! Regression tests for global (home) deployments.
//!
//! When deploying to home (`calvin deploy --home`), the lockfile and registry semantics must not
//! pollute the current working directory. Home deployments are global, so they should be tracked
//! under `~/.calvin/` and be cleanable from any directory.

use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

#[test]
fn deploy_home_uses_global_lockfile_and_clean_home_from_any_dir() {
    let dir = tempdir().unwrap();

    let home = dir.path().join("home");
    fs::create_dir_all(home.join(".config")).unwrap();

    let project = dir.path().join("project");
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(project.join(".git")).unwrap();

    let promptpack = project.join(".promptpack");
    fs::create_dir_all(&promptpack).unwrap();

    fs::write(
        promptpack.join("policy.md"),
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

    let deploy = Command::new(bin())
        .current_dir(&project)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .args(["deploy", "--home", "--yes", "--targets", "cursor"])
        .output()
        .unwrap();

    assert!(
        deploy.status.success(),
        "deploy --home failed: {}",
        String::from_utf8_lossy(&deploy.stderr)
    );

    // Home deploy should not create a project lockfile in the current directory.
    assert!(
        !project.join("calvin.lock").exists(),
        "did not expect project lockfile for home deploy"
    );

    // Home deploy should create a global lockfile under ~/.calvin.
    let global_lockfile = home.join(".calvin").join("calvin.lock");
    assert!(
        global_lockfile.exists(),
        "expected global lockfile at {}",
        global_lockfile.display()
    );

    // Home deploy should not register the project in the global registry.
    let registry_path = home.join(".calvin").join("registry.toml");
    assert!(
        !registry_path.exists(),
        "did not expect registry update for home deploy"
    );

    // Verify an expected output exists under the fake home.
    let deployed_file = home.join(".cursor/rules/policy/RULE.md");
    assert!(
        deployed_file.exists(),
        "expected home deployed file at {}",
        deployed_file.display()
    );

    // Clean home deployments should work from any directory (even without a project .promptpack).
    let clean = Command::new(bin())
        .current_dir(dir.path())
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .args(["clean", "--home", "--yes"])
        .output()
        .unwrap();

    assert!(
        clean.status.success(),
        "clean --home failed: {}",
        String::from_utf8_lossy(&clean.stderr)
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
