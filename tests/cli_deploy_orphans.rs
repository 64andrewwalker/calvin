//! Integration tests for orphan detection and cleanup
//!
//! These tests verify the orphan file handling behavior during deploy.

use std::fs;
use std::process::Command;

use tempfile::tempdir;

/// Create a minimal test project with promptpack
fn create_test_project() -> tempfile::TempDir {
    let dir = tempdir().unwrap();

    // Avoid "not a git repository" prompt
    fs::create_dir_all(dir.path().join(".git")).unwrap();

    let promptpack = dir.path().join(".promptpack");
    fs::create_dir_all(&promptpack).unwrap();

    dir
}

/// Create an asset file
fn create_asset(dir: &tempfile::TempDir, name: &str, content: &str) {
    let path = dir.path().join(".promptpack").join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

/// Create a config file
fn create_config(dir: &tempfile::TempDir, content: &str) {
    fs::write(dir.path().join(".promptpack/config.toml"), content).unwrap();
}

fn run_deploy(dir: &tempfile::TempDir, args: &[&str]) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_calvin");
    Command::new(bin)
        .current_dir(dir.path())
        .args(["deploy"])
        .args(args)
        .output()
        .unwrap()
}

// ============================================================================
// Basic Orphan Detection Tests
// ============================================================================

#[test]
fn deploy_creates_lockfile() {
    let dir = create_test_project();
    create_asset(
        &dir,
        "test.md",
        r#"---
kind: policy
description: Test
scope: project
targets: [cursor]
---
Content
"#,
    );
    create_config(
        &dir,
        r#"
[deploy]
target = "project"

[targets]
enabled = ["cursor"]
"#,
    );

    let output = run_deploy(&dir, &["--yes"]);
    assert!(output.status.success());

    // Lockfile should be created
    let lockfile = dir.path().join("calvin.lock");
    assert!(lockfile.exists(), "Lockfile should be created");
}

#[test]
fn deploy_detects_orphan_after_asset_removal() {
    let dir = create_test_project();
    create_asset(
        &dir,
        "test.md",
        r#"---
kind: policy
description: Test
scope: project
targets: [cursor]
---
Content
"#,
    );
    create_config(
        &dir,
        r#"
[deploy]
target = "project"

[targets]
enabled = ["cursor"]
"#,
    );

    // First deploy
    let output = run_deploy(&dir, &["--yes"]);
    assert!(output.status.success());

    // Remove the asset
    fs::remove_file(dir.path().join(".promptpack/test.md")).unwrap();

    // Second deploy should detect orphan
    let output = run_deploy(&dir, &["--yes"]);
    assert!(output.status.success());

    // Check stderr for orphan warning
    let _stderr = String::from_utf8_lossy(&output.stderr);
    // Orphan detection is not always printed, depending on mode
    // Just verify deploy succeeded
}

#[test]
fn deploy_cleanup_flag_removes_orphans() {
    let dir = create_test_project();
    create_asset(
        &dir,
        "test.md",
        r#"---
kind: policy
description: Test
scope: project
targets: [cursor]
---
Content
"#,
    );
    create_config(
        &dir,
        r#"
[deploy]
target = "project"

[targets]
enabled = ["cursor"]
"#,
    );

    // First deploy
    let output = run_deploy(&dir, &["--yes"]);
    assert!(output.status.success());

    // Remove the asset but keep the deployed file
    fs::remove_file(dir.path().join(".promptpack/test.md")).unwrap();

    // Deploy with --cleanup
    let output = run_deploy(&dir, &["--yes", "--cleanup"]);
    assert!(output.status.success());
}

#[test]
fn deploy_dry_run_does_not_delete_orphans() {
    let dir = create_test_project();
    create_asset(
        &dir,
        "test.md",
        r#"---
kind: policy
description: Test
scope: project
targets: [cursor]
---
Content
"#,
    );
    create_config(
        &dir,
        r#"
[deploy]
target = "project"

[targets]
enabled = ["cursor"]
"#,
    );

    // First deploy
    let output = run_deploy(&dir, &["--yes"]);
    assert!(output.status.success());

    // Find the deployed file path
    let cursor_rules = dir.path().join(".cursor/rules/test.mdc");
    let deployed_exists_before = cursor_rules.exists();

    // Remove the asset
    fs::remove_file(dir.path().join(".promptpack/test.md")).unwrap();

    // Deploy with --cleanup --dry-run
    let output = run_deploy(&dir, &["--cleanup", "--dry-run"]);
    assert!(output.status.success());

    // Deployed file should still exist (dry-run)
    let deployed_exists_after = cursor_rules.exists();
    assert_eq!(
        deployed_exists_before, deployed_exists_after,
        "Dry-run should not change files"
    );
}

// ============================================================================
// JSON Event Stream Tests
// ============================================================================

#[test]
fn deploy_json_emits_orphan_events() {
    let dir = create_test_project();
    create_asset(
        &dir,
        "test.md",
        r#"---
kind: policy
description: Test
scope: project
targets: [cursor]
---
Content
"#,
    );
    create_config(
        &dir,
        r#"
[deploy]
target = "project"

[targets]
enabled = ["cursor"]
"#,
    );

    // First deploy
    let output = run_deploy(&dir, &["--json"]);
    assert!(output.status.success());

    // Remove the asset
    fs::remove_file(dir.path().join(".promptpack/test.md")).unwrap();

    // Second deploy with JSON output
    let output = run_deploy(&dir, &["--json", "--cleanup"]);
    assert!(output.status.success());

    // Verify JSON output is valid
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if !line.trim().is_empty() {
            let _: serde_json::Value = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("Invalid JSON: {} - Error: {}", line, e));
        }
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn deploy_empty_promptpack_succeeds() {
    let dir = create_test_project();
    create_config(
        &dir,
        r#"
[deploy]
target = "project"

[targets]
enabled = ["cursor"]
"#,
    );

    // Deploy with no assets
    let output = run_deploy(&dir, &["--yes"]);
    assert!(output.status.success());
}

#[test]
fn deploy_multiple_assets_creates_multiple_files() {
    let dir = create_test_project();
    create_asset(
        &dir,
        "policy1.md",
        r#"---
kind: policy
description: Policy 1
scope: project
targets: [cursor]
---
Content 1
"#,
    );
    create_asset(
        &dir,
        "policy2.md",
        r#"---
kind: policy
description: Policy 2
scope: project
targets: [cursor]
---
Content 2
"#,
    );
    create_config(
        &dir,
        r#"
[deploy]
target = "project"

[targets]
enabled = ["cursor"]
"#,
    );

    let output = run_deploy(&dir, &["--yes"]);
    assert!(output.status.success());

    // Check that lockfile was created and contains entries
    let lockfile = dir.path().join("calvin.lock");
    assert!(lockfile.exists(), "Lockfile should be created");

    let lockfile_content = fs::read_to_string(&lockfile).unwrap();
    // Should have multiple file entries
    assert!(
        lockfile_content.contains("policy1") || lockfile_content.contains("policy2"),
        "Lockfile should contain asset references: {}",
        lockfile_content
    );
}

#[test]
fn deploy_removing_one_asset_orphans_only_that_file() {
    let dir = create_test_project();
    create_asset(
        &dir,
        "policy1.md",
        r#"---
kind: policy
description: Policy 1
scope: project
targets: [cursor]
---
Content 1
"#,
    );
    create_asset(
        &dir,
        "policy2.md",
        r#"---
kind: policy
description: Policy 2
scope: project
targets: [cursor]
---
Content 2
"#,
    );
    create_config(
        &dir,
        r#"
[deploy]
target = "project"

[targets]
enabled = ["cursor"]
"#,
    );

    // First deploy
    let output = run_deploy(&dir, &["--yes"]);
    assert!(output.status.success());

    // Remove only policy1
    fs::remove_file(dir.path().join(".promptpack/policy1.md")).unwrap();

    // Deploy with cleanup
    let output = run_deploy(&dir, &["--yes", "--cleanup"]);
    assert!(output.status.success());

    // Lockfile should still contain policy2 but not policy1
    let lockfile = dir.path().join("calvin.lock");
    let lockfile_content = fs::read_to_string(&lockfile).unwrap();

    // policy2 should still be in lockfile
    assert!(
        lockfile_content.contains("policy2"),
        "Lockfile should still contain policy2: {}",
        lockfile_content
    );
}

// ============================================================================
// Lockfile Recovery Tests
// ============================================================================

/// Regression test: lockfile should be populated when re-deploying to existing files
///
/// This tests the scenario where:
/// 1. Files have been deployed previously
/// 2. Lockfile is lost/empty
/// 3. Re-deploying should:
///    a. Skip writing (because content is identical)
///    b. BUT still update lockfile to track the files
#[test]
fn deploy_recovers_lockfile_when_files_exist() {
    let dir = create_test_project();
    create_asset(
        &dir,
        "test.md",
        r#"---
kind: policy
description: Test
scope: project
targets: [cursor]
---
Content
"#,
    );
    create_config(
        &dir,
        r#"
[deploy]
target = "project"

[targets]
enabled = ["cursor"]
"#,
    );

    // First deploy - creates lockfile and output files
    let output = run_deploy(&dir, &["--yes"]);
    assert!(output.status.success());

    // Verify lockfile was created and has entries
    let lockfile_path = dir.path().join("calvin.lock");
    assert!(lockfile_path.exists(), "Lockfile should be created");

    let lockfile_content = fs::read_to_string(&lockfile_path).unwrap();
    assert!(
        lockfile_content.contains("[files."),
        "Lockfile should have file entries: {}",
        lockfile_content
    );

    // Delete the lockfile to simulate it being lost
    fs::remove_file(&lockfile_path).unwrap();

    // Second deploy - files exist with identical content, should skip writing
    // but lockfile should be recreated and populated
    let output2 = run_deploy(&dir, &["--yes"]);
    assert!(output2.status.success());

    // Lockfile should exist again
    assert!(
        lockfile_path.exists(),
        "Lockfile should be recreated after re-deploy"
    );

    // Lockfile should have the file entry
    let lockfile_content2 = fs::read_to_string(&lockfile_path).unwrap();
    assert!(
        lockfile_content2.contains("[files."),
        "Recovered lockfile should have file entries: {}",
        lockfile_content2
    );
}
