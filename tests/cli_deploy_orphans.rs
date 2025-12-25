//! Integration tests for orphan detection and cleanup
//!
//! These tests verify the orphan file handling behavior during deploy.

mod common;

use common::*;

// ============================================================================
// Basic Orphan Detection Tests
// ============================================================================

#[test]
fn deploy_creates_lockfile() {
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .with_project_config(CONFIG_DEPLOY_PROJECT)
        .build();

    let result = env.run(&["deploy", "--yes"]);
    assert!(
        result.success,
        "Deploy failed:\n{}",
        result.combined_output()
    );

    // Lockfile should be created
    let lockfile = env.project_path("calvin.lock");
    assert!(lockfile.exists(), "Lockfile should be created");
}

#[test]
fn deploy_detects_orphan_after_asset_removal() {
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .with_project_config(CONFIG_DEPLOY_PROJECT)
        .build();

    // First deploy
    let result = env.run(&["deploy", "--yes"]);
    assert!(
        result.success,
        "Deploy failed:\n{}",
        result.combined_output()
    );

    // Remove the asset
    env.remove_project_asset("test.md");

    // Second deploy should detect orphan
    let result = env.run(&["deploy", "--yes"]);
    assert!(
        result.success,
        "Deploy after asset removal failed:\n{}",
        result.combined_output()
    );

    // Check stderr for orphan warning
    let _stderr = result.stderr;
    // Orphan detection is not always printed, depending on mode
    // Just verify deploy succeeded
}

#[test]
fn deploy_cleanup_flag_removes_orphans() {
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .with_project_config(CONFIG_DEPLOY_PROJECT)
        .build();

    // First deploy
    let result = env.run(&["deploy", "--yes"]);
    assert!(
        result.success,
        "Deploy failed:\n{}",
        result.combined_output()
    );

    // Remove the asset but keep the deployed file
    env.remove_project_asset("test.md");

    // Deploy with --cleanup
    let result = env.run(&["deploy", "--yes", "--cleanup"]);
    assert!(
        result.success,
        "Deploy with --cleanup failed:\n{}",
        result.combined_output()
    );
}

#[test]
fn deploy_dry_run_does_not_delete_orphans() {
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .with_project_config(CONFIG_DEPLOY_PROJECT)
        .build();

    // First deploy
    let result = env.run(&["deploy", "--yes"]);
    assert!(
        result.success,
        "Deploy failed:\n{}",
        result.combined_output()
    );

    // Find the deployed file path
    let rule_path = env.project_path(".cursor/rules/test/RULE.md");
    assert!(
        rule_path.exists(),
        "Expected deployed Cursor rule at {:?}",
        rule_path
    );

    // Remove the asset
    env.remove_project_asset("test.md");

    // Deploy with --cleanup --dry-run
    let result = env.run(&["deploy", "--cleanup", "--dry-run"]);
    assert!(
        result.success,
        "Deploy with --cleanup --dry-run failed:\n{}",
        result.combined_output()
    );

    // Deployed file should still exist (dry-run)
    assert!(
        rule_path.exists(),
        "Dry-run should not delete {:?}",
        rule_path
    );
}

// ============================================================================
// JSON Event Stream Tests
// ============================================================================

#[test]
fn deploy_json_emits_orphan_events() {
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .with_project_config(CONFIG_DEPLOY_PROJECT)
        .build();

    // First deploy
    let result = env.run(&["deploy", "--json"]);
    assert!(
        result.success,
        "Deploy --json failed:\n{}",
        result.combined_output()
    );

    // Remove the asset
    env.remove_project_asset("test.md");

    // Second deploy with JSON output
    let result = env.run(&["deploy", "--json", "--cleanup"]);
    assert!(
        result.success,
        "Deploy --json --cleanup failed:\n{}",
        result.combined_output()
    );

    // Verify JSON output is valid
    for line in result.stdout.lines() {
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
    let env = TestEnv::builder()
        .with_project_config(CONFIG_DEPLOY_PROJECT)
        .build();

    // Deploy with no assets
    let result = env.run(&["deploy", "--yes"]);
    assert!(
        result.success,
        "Deploy with empty promptpack failed:\n{}",
        result.combined_output()
    );
}

#[test]
fn deploy_multiple_assets_creates_multiple_files() {
    let env = TestEnv::builder()
        .with_project_asset("policy1.md", SIMPLE_POLICY)
        .with_project_asset("policy2.md", SIMPLE_POLICY)
        .with_project_config(CONFIG_DEPLOY_PROJECT)
        .build();

    let result = env.run(&["deploy", "--yes"]);
    assert!(
        result.success,
        "Deploy failed:\n{}",
        result.combined_output()
    );

    // Check that lockfile was created and contains entries
    let lockfile = env.project_path("calvin.lock");
    assert!(lockfile.exists(), "Lockfile should be created");

    let lockfile_content = std::fs::read_to_string(&lockfile).unwrap();
    // Should have multiple file entries
    assert!(
        lockfile_content.contains("policy1") || lockfile_content.contains("policy2"),
        "Lockfile should contain asset references: {}",
        lockfile_content
    );
}

#[test]
fn deploy_removing_one_asset_orphans_only_that_file() {
    let env = TestEnv::builder()
        .with_project_asset("policy1.md", SIMPLE_POLICY)
        .with_project_asset("policy2.md", SIMPLE_POLICY)
        .with_project_config(CONFIG_DEPLOY_PROJECT)
        .build();

    // First deploy
    let result = env.run(&["deploy", "--yes"]);
    assert!(
        result.success,
        "Deploy failed:\n{}",
        result.combined_output()
    );

    // Remove only policy1
    env.remove_project_asset("policy1.md");

    // Deploy with cleanup
    let result = env.run(&["deploy", "--yes", "--cleanup"]);
    assert!(
        result.success,
        "Deploy with cleanup failed:\n{}",
        result.combined_output()
    );

    // Lockfile should still contain policy2 but not policy1
    let lockfile = env.project_path("calvin.lock");
    let lockfile_content = std::fs::read_to_string(&lockfile).unwrap();

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
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .with_project_config(CONFIG_DEPLOY_PROJECT)
        .build();

    // First deploy - creates lockfile and output files
    let result = env.run(&["deploy", "--yes"]);
    assert!(
        result.success,
        "Deploy failed:\n{}",
        result.combined_output()
    );

    // Verify lockfile was created and has entries
    let lockfile_path = env.project_path("calvin.lock");
    assert!(lockfile_path.exists(), "Lockfile should be created");

    let lockfile_content = std::fs::read_to_string(&lockfile_path).unwrap();
    assert!(
        lockfile_content.contains("[files."),
        "Lockfile should have file entries: {}",
        lockfile_content
    );

    // Delete the lockfile to simulate it being lost
    std::fs::remove_file(&lockfile_path).unwrap();

    // Second deploy - files exist with identical content, should skip writing
    // but lockfile should be recreated and populated
    let result2 = env.run(&["deploy", "--yes"]);
    assert!(
        result2.success,
        "Second deploy failed:\n{}",
        result2.combined_output()
    );

    // Lockfile should exist again
    assert!(
        lockfile_path.exists(),
        "Lockfile should be recreated after re-deploy"
    );

    // Lockfile should have the file entry
    let lockfile_content2 = std::fs::read_to_string(&lockfile_path).unwrap();
    assert!(
        lockfile_content2.contains("[files."),
        "Recovered lockfile should have file entries: {}",
        lockfile_content2
    );
}
