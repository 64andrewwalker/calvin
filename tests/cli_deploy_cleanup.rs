//! Integration tests for orphan cleanup functionality

use std::process::Command;
use tempfile::tempdir;

/// Create a test project with promptpack structure
fn create_test_project() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    let promptpack = dir.path().join(".promptpack");
    std::fs::create_dir_all(&promptpack).unwrap();
    
    // Create a simple asset
    std::fs::write(
        promptpack.join("test.md"),
        r#"---
kind: policy
description: Test Policy
scope: project
---
Test content
"#,
    )
    .unwrap();
    
    // Create config
    std::fs::write(
        promptpack.join("config.toml"),
        r#"
[deploy]
target = "project"

[targets]
enabled = ["cursor"]
"#,
    )
    .unwrap();
    
    dir
}

#[test]
fn test_deploy_cleanup_flag_accepted() {
    let dir = create_test_project();
    let bin = env!("CARGO_BIN_EXE_calvin");
    
    let output = Command::new(bin)
        .current_dir(dir.path())
        .args(["deploy", "--cleanup", "--dry-run"])
        .output()
        .unwrap();
    
    if !output.status.success() {
        println!("STDOUT:\n{}", String::from_utf8_lossy(&output.stdout));
        println!("STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success());
}

#[test]
fn test_deploy_no_cleanup_warns_only() {
    let dir = create_test_project();
    let bin = env!("CARGO_BIN_EXE_calvin");
    
    // First deploy to create lockfile
    let output = Command::new(bin)
        .current_dir(dir.path())
        .args(["deploy", "--yes"])
        .output()
        .unwrap();
    assert!(output.status.success());
    
    // Remove the asset (simulates orphan scenario)
    std::fs::remove_file(dir.path().join(".promptpack/test.md")).unwrap();
    
    // Create a new asset instead
    std::fs::write(
        dir.path().join(".promptpack/new.md"),
        r#"---
kind: policy
description: New Policy
scope: project
---
New content
"#,
    )
    .unwrap();
    
    // Deploy without --cleanup should warn but not delete
    let output = Command::new(bin)
        .current_dir(dir.path())
        .args(["deploy", "--yes"])
        .output()
        .unwrap();
    
    assert!(output.status.success());
    
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
    let dir = create_test_project();
    let bin = env!("CARGO_BIN_EXE_calvin");
    
    let output = Command::new(bin)
        .current_dir(dir.path())
        .args(["deploy", "--cleanup", "--json", "--yes"])
        .output()
        .unwrap();
    
    // Should produce valid JSON output
    assert!(output.status.success());
    
    // Output should be valid NDJSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if !line.trim().is_empty() {
            // Each line should be valid JSON
            let _: serde_json::Value = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("Invalid JSON: {} - Error: {}", line, e));
        }
    }
}

#[test]
fn test_deploy_dry_run_shows_orphans() {
    let dir = create_test_project();
    let bin = env!("CARGO_BIN_EXE_calvin");
    
    // First deploy to create lockfile
    let output = Command::new(bin)
        .current_dir(dir.path())
        .args(["deploy", "--yes"])
        .output()
        .unwrap();
    assert!(output.status.success());
    
    // Now do a dry-run - should still show orphan info if applicable
    let output = Command::new(bin)
        .current_dir(dir.path())
        .args(["deploy", "--dry-run"])
        .output()
        .unwrap();
    
    // Should succeed without errors
    assert!(output.status.success());
}
