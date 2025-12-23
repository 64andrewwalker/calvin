//! Integration tests for calvin clean command

use std::process::Command;
use tempfile::tempdir;

/// Get the path to the calvin binary
fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

/// Create a test project with promptpack structure and deploy it
fn create_deployed_project() -> tempfile::TempDir {
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

    // Deploy first to create lockfile
    let output = Command::new(bin())
        .current_dir(dir.path())
        .args(["deploy", "--yes"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Deploy failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    dir
}

// === Phase 1.1: CLI Definition Tests ===

#[test]
fn clean_help_shows_options() {
    let output = Command::new(bin())
        .args(["clean", "--help"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show help successfully
    assert!(output.status.success(), "clean --help should succeed");

    // Should include key options
    assert!(stdout.contains("--home"), "Should have --home option");
    assert!(stdout.contains("--project"), "Should have --project option");
    assert!(stdout.contains("--dry-run"), "Should have --dry-run option");
    assert!(
        stdout.contains("--yes") || stdout.contains("-y"),
        "Should have --yes option"
    );
}

#[test]
fn clean_requires_lockfile() {
    let dir = tempdir().unwrap();
    let promptpack = dir.path().join(".promptpack");
    std::fs::create_dir_all(&promptpack).unwrap();

    // Create minimal config but no lockfile
    std::fs::write(
        promptpack.join("config.toml"),
        r#"
[deploy]
target = "project"
"#,
    )
    .unwrap();

    let output = Command::new(bin())
        .current_dir(dir.path())
        .args(["clean", "--project", "--yes"])
        .output()
        .unwrap();

    // Should fail or show message about missing lockfile
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);

    // Either exit with error or show message about no deployments
    assert!(
        !output.status.success()
            || combined.to_lowercase().contains("no ")
            || combined.to_lowercase().contains("nothing")
            || combined.to_lowercase().contains("lockfile"),
        "Should indicate no deployments found or missing lockfile"
    );
}

#[test]
fn clean_dry_run_no_delete() {
    let dir = create_deployed_project();

    // Find deployed file
    let cursor_rules = dir.path().join(".cursor/rules");
    let deployed_files: Vec<_> = if cursor_rules.exists() {
        std::fs::read_dir(&cursor_rules)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|s| s == "mdc").unwrap_or(false))
            .collect()
    } else {
        vec![]
    };

    // Run clean with dry-run
    let output = Command::new(bin())
        .current_dir(dir.path())
        .args(["clean", "--project", "--dry-run"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "clean --dry-run should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Files should still exist
    for file in &deployed_files {
        assert!(
            file.path().exists(),
            "File {} should still exist after dry-run",
            file.path().display()
        );
    }
}

#[test]
fn clean_project_removes_files() {
    let dir = create_deployed_project();

    // Find deployed file
    let cursor_rules = dir.path().join(".cursor/rules");
    assert!(
        cursor_rules.exists(),
        "Cursor rules directory should exist after deploy"
    );

    // Run clean
    let output = Command::new(bin())
        .current_dir(dir.path())
        .args(["clean", "--project", "--yes"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "clean --project --yes should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Deployed files should be removed
    let remaining_files: Vec<_> = if cursor_rules.exists() {
        std::fs::read_dir(&cursor_rules)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|s| s == "mdc").unwrap_or(false))
            .collect()
    } else {
        vec![]
    };

    assert!(
        remaining_files.is_empty(),
        "Deployed files should be removed: {:?}",
        remaining_files
    );
}

#[test]
fn clean_respects_yes_flag() {
    let dir = create_deployed_project();

    // Run clean with --yes should not prompt
    let output = Command::new(bin())
        .current_dir(dir.path())
        .args(["clean", "--project", "--yes"])
        .output()
        .unwrap();

    // Should succeed without waiting for input
    assert!(
        output.status.success(),
        "clean --yes should succeed without prompting"
    );
}

#[test]
fn clean_json_output() {
    let dir = create_deployed_project();

    let output = Command::new(bin())
        .current_dir(dir.path())
        .args(["clean", "--project", "--yes", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success(), "clean --json should succeed");

    // Output should be valid NDJSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if !line.trim().is_empty() {
            let _: serde_json::Value = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("Invalid JSON: {} - Error: {}", line, e));
        }
    }
}
