//! Integration tests for calvin clean command

use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

/// Normalize path for use in lockfile keys (Windows uses backslashes)
fn normalize_path_for_key(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

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

#[test]
fn clean_json_output_includes_key_field() {
    let dir = create_deployed_project();

    let output = Command::new(bin())
        .current_dir(dir.path())
        .args(["clean", "--project", "--yes", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success(), "clean --json should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut has_file_deleted = false;
    let mut has_key_field = false;

    for line in stdout.lines() {
        if line.contains("\"type\":\"file_deleted\"") {
            has_file_deleted = true;
            if line.contains("\"key\":") {
                has_key_field = true;
            }
        }
    }

    // If we have deleted events, they should include key field
    if has_file_deleted {
        assert!(
            has_key_field,
            "file_deleted events should include key field for programmatic use"
        );
    }
}

#[test]
fn clean_json_complete_includes_errors_count() {
    let dir = create_deployed_project();

    let output = Command::new(bin())
        .current_dir(dir.path())
        .args(["clean", "--project", "--yes", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success(), "clean --json should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        if line.contains("\"type\":\"clean_complete\"") {
            assert!(
                line.contains("\"errors\":"),
                "clean_complete should include errors count"
            );
        }
    }
}

// === Test Variants: Input Boundaries ===
// Using double underscore to mark variant tests per test-variants spec

#[test]
#[allow(non_snake_case)]
fn clean__with_empty_lockfile() {
    let dir = tempdir().unwrap();
    let promptpack = dir.path().join(".promptpack");
    std::fs::create_dir_all(&promptpack).unwrap();

    // Create empty lockfile
    std::fs::write(promptpack.join(".calvin.lock"), "version = 1\n\n[files]\n").unwrap();

    // Create minimal config
    std::fs::write(
        promptpack.join("config.toml"),
        "[deploy]\ntarget = \"project\"\n",
    )
    .unwrap();

    let output = Command::new(bin())
        .current_dir(dir.path())
        .args(["clean", "--project", "--yes"])
        .output()
        .unwrap();

    // Should handle empty lockfile gracefully
    assert!(
        output.status.success(),
        "clean should succeed with empty lockfile"
    );
}

#[test]
#[allow(non_snake_case)]
fn clean__with_nonexistent_source() {
    let dir = tempdir().unwrap();

    let output = Command::new(bin())
        .current_dir(dir.path())
        .args([
            "clean",
            "--source",
            "/nonexistent/path",
            "--project",
            "--yes",
        ])
        .output()
        .unwrap();

    // Should fail gracefully
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success()
            || combined.to_lowercase().contains("no ")
            || combined.to_lowercase().contains("lockfile"),
        "Should indicate missing source or lockfile"
    );
}

#[test]
#[allow(non_snake_case)]
fn clean__conflicting_scope_flags() {
    let dir = tempdir().unwrap();
    let promptpack = dir.path().join(".promptpack");
    std::fs::create_dir_all(&promptpack).unwrap();

    let output = Command::new(bin())
        .current_dir(dir.path())
        .args(["clean", "--home", "--project"])
        .output()
        .unwrap();

    // Should fail due to conflicting flags (clap conflicts_with)
    assert!(
        !output.status.success(),
        "--home and --project should conflict"
    );
}

// === Test Variants: State Variations ===

#[test]
#[allow(non_snake_case)]
fn clean__with_already_deleted_files() {
    let dir = tempdir().unwrap();
    let promptpack = dir.path().join(".promptpack");
    std::fs::create_dir_all(&promptpack).unwrap();

    // Create lockfile that references a file that doesn't exist
    let missing_file = dir.path().join(".cursor/rules/missing.mdc");
    std::fs::write(
        promptpack.join(".calvin.lock"),
        format!(
            r#"version = 1

[files."project:{}"]
hash = "sha256:somehash"
"#,
            normalize_path_for_key(&missing_file)
        ),
    )
    .unwrap();

    std::fs::write(
        promptpack.join("config.toml"),
        "[deploy]\ntarget = \"project\"\n",
    )
    .unwrap();

    let output = Command::new(bin())
        .current_dir(dir.path())
        .args(["clean", "--project", "--yes"])
        .output()
        .unwrap();

    // Should handle missing files gracefully
    assert!(
        output.status.success(),
        "clean should succeed with missing files"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should indicate files were skipped
    assert!(
        stdout.contains("skipped") || stdout.contains("0 files deleted"),
        "Should show skipped or 0 deleted"
    );
}

#[test]
#[allow(non_snake_case)]
fn clean__with_mixed_scope_entries() {
    let dir = tempdir().unwrap();
    let promptpack = dir.path().join(".promptpack");
    std::fs::create_dir_all(&promptpack).unwrap();

    // Create file in project scope
    let project_file = dir.path().join(".cursor/rules/project.mdc");
    std::fs::create_dir_all(project_file.parent().unwrap()).unwrap();
    std::fs::write(
        &project_file,
        "<!-- Generated by Calvin -->\nProject content",
    )
    .unwrap();

    // Compute hash
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update("<!-- Generated by Calvin -->\nProject content".as_bytes());
    let hash = format!("sha256:{:x}", hasher.finalize());

    // Create lockfile with both home and project entries
    std::fs::write(
        promptpack.join(".calvin.lock"),
        format!(
            r#"version = 1

[files."project:{}"]
hash = "{}"

[files."home:~/.claude/commands/home-file.md"]
hash = "sha256:homefilehash"
"#,
            normalize_path_for_key(&project_file),
            hash
        ),
    )
    .unwrap();

    std::fs::write(
        promptpack.join("config.toml"),
        "[deploy]\ntarget = \"project\"\n",
    )
    .unwrap();

    // Clean only project scope
    let output = Command::new(bin())
        .current_dir(dir.path())
        .args(["clean", "--project", "--yes"])
        .output()
        .unwrap();

    assert!(output.status.success(), "clean --project should succeed");

    // Project file should be deleted
    assert!(!project_file.exists(), "Project file should be deleted");
}

// === Test Variants: Error Injection ===

#[test]
#[allow(non_snake_case)]
fn clean__with_file_without_signature() {
    let dir = tempdir().unwrap();
    let promptpack = dir.path().join(".promptpack");
    std::fs::create_dir_all(&promptpack).unwrap();

    // Create file WITHOUT Calvin signature
    let file = dir.path().join(".cursor/rules/nosig.mdc");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "User created content without signature").unwrap();

    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update("User created content without signature".as_bytes());
    let hash = format!("sha256:{:x}", hasher.finalize());

    std::fs::write(
        promptpack.join(".calvin.lock"),
        format!(
            r#"version = 1

[files."project:{}"]
hash = "{}"
"#,
            normalize_path_for_key(&file),
            hash
        ),
    )
    .unwrap();

    std::fs::write(
        promptpack.join("config.toml"),
        "[deploy]\ntarget = \"project\"\n",
    )
    .unwrap();

    let output = Command::new(bin())
        .current_dir(dir.path())
        .args(["clean", "--project", "--yes"])
        .output()
        .unwrap();

    // Should succeed but skip the file
    assert!(output.status.success(), "clean should succeed");

    // File should still exist (skipped due to no signature)
    assert!(
        file.exists(),
        "File without signature should not be deleted"
    );
}

#[test]
#[allow(non_snake_case)]
fn clean__with_force_deletes_unsigned_file() {
    let dir = tempdir().unwrap();
    let promptpack = dir.path().join(".promptpack");
    std::fs::create_dir_all(&promptpack).unwrap();

    // Create file WITHOUT Calvin signature
    let file = dir.path().join(".cursor/rules/nosig.mdc");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "User created content without signature").unwrap();

    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update("User created content without signature".as_bytes());
    let hash = format!("sha256:{:x}", hasher.finalize());

    std::fs::write(
        promptpack.join(".calvin.lock"),
        format!(
            r#"version = 1

[files."project:{}"]
hash = "{}"
"#,
            normalize_path_for_key(&file),
            hash
        ),
    )
    .unwrap();

    std::fs::write(
        promptpack.join("config.toml"),
        "[deploy]\ntarget = \"project\"\n",
    )
    .unwrap();

    let output = Command::new(bin())
        .current_dir(dir.path())
        .args(["clean", "--project", "--yes", "--force"])
        .output()
        .unwrap();

    // Should succeed
    assert!(output.status.success(), "clean --force should succeed");

    // File should be deleted with --force
    assert!(!file.exists(), "File should be deleted with --force");
}

#[test]
#[allow(non_snake_case)]
fn clean__with_modified_file() {
    let dir = tempdir().unwrap();
    let promptpack = dir.path().join(".promptpack");
    std::fs::create_dir_all(&promptpack).unwrap();

    // Create file with Calvin signature but modified content
    let file = dir.path().join(".cursor/rules/modified.mdc");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "<!-- Generated by Calvin -->\nModified content").unwrap();

    // Use a DIFFERENT hash in lockfile (simulating modification)
    std::fs::write(
        promptpack.join(".calvin.lock"),
        format!(
            r#"version = 1

[files."project:{}"]
hash = "sha256:originalhashbeforemodification1234567890abcdef"
"#,
            normalize_path_for_key(&file)
        ),
    )
    .unwrap();

    std::fs::write(
        promptpack.join("config.toml"),
        "[deploy]\ntarget = \"project\"\n",
    )
    .unwrap();

    let output = Command::new(bin())
        .current_dir(dir.path())
        .args(["clean", "--project", "--yes"])
        .output()
        .unwrap();

    // Should succeed but skip modified file
    assert!(output.status.success(), "clean should succeed");

    // Modified file should still exist
    assert!(file.exists(), "Modified file should not be deleted");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("skipped"),
        "Should indicate file was skipped"
    );
}

// === Test Variants: Output Format ===

#[test]
#[allow(non_snake_case)]
fn clean__dry_run_shows_would_be_deleted() {
    let dir = create_deployed_project();

    let output = Command::new(bin())
        .current_dir(dir.path())
        .args(["clean", "--project", "--dry-run"])
        .output()
        .unwrap();

    assert!(output.status.success(), "clean --dry-run should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should mention "would be" in dry run mode
    assert!(
        stdout.to_lowercase().contains("would")
            || stdout.to_lowercase().contains("dry")
            || stdout.to_lowercase().contains("preview"),
        "Dry run output should indicate preview mode"
    );
}
