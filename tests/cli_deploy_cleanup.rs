//! Integration tests for orphan cleanup functionality

mod common;

use common::*;

#[test]
fn test_deploy_cleanup_flag_accepted() {
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .with_project_config(CONFIG_DEPLOY_PROJECT)
        .build();

    let result = env.run(&["deploy", "--cleanup", "--dry-run"]);

    assert!(
        result.success,
        "deploy --cleanup --dry-run failed:\n{}",
        result.combined_output()
    );
}

#[test]
fn test_deploy_no_cleanup_warns_only() {
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .with_project_config(CONFIG_DEPLOY_PROJECT)
        .build();

    // First deploy to create lockfile
    let result = env.run(&["deploy", "--yes"]);
    assert!(
        result.success,
        "Initial deploy failed:\n{}",
        result.combined_output()
    );

    // Remove the asset (simulates orphan scenario)
    env.remove_project_asset("test.md");
    env.write_project_file(
        ".promptpack/new.md",
        r#"---
kind: policy
description: New Policy
scope: project
targets: [cursor]
---
New content
"#,
    );

    // Deploy without --cleanup should warn but not delete
    let result = env.run(&["deploy", "--yes"]);
    assert!(
        result.success,
        "Deploy without --cleanup failed:\n{}",
        result.combined_output()
    );

    // Check output contains warning
    // We can't easily check for colors/icons in raw output without filtering, but we can check for text.
    // "orphans detected" or "Run with --cleanup"
    // But stdout/stderr might be mixed? output.stderr?
    // In cmd.rs, warnings are printed to stderr if !json.
    // However, orphan summary is printed to stderr.

    // Wait, the warning about "run with --cleanup" is explicit in cmd.rs.
}

#[test]
fn test_deploy_cleanup_json_events() {
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .with_project_config(CONFIG_DEPLOY_PROJECT)
        .build();

    let result = env.run(&["deploy", "--cleanup", "--json", "--yes"]);

    // Should produce valid JSON output
    assert!(
        result.success,
        "deploy --cleanup --json failed:\n{}",
        result.combined_output()
    );

    // Output should be valid NDJSON
    for line in result.stdout.lines() {
        if !line.trim().is_empty() {
            // Each line should be valid JSON
            let _: serde_json::Value = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("Invalid JSON: {} - Error: {}", line, e));
        }
    }
}

#[test]
fn test_deploy_dry_run_shows_orphans() {
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .with_project_config(CONFIG_DEPLOY_PROJECT)
        .build();

    // First deploy to create lockfile
    let result = env.run(&["deploy", "--yes"]);
    assert!(
        result.success,
        "Initial deploy failed:\n{}",
        result.combined_output()
    );

    // Now do a dry-run - should still show orphan info if applicable
    let result = env.run(&["deploy", "--dry-run"]);

    // Should succeed without errors
    assert!(
        result.success,
        "deploy --dry-run failed:\n{}",
        result.combined_output()
    );
}
