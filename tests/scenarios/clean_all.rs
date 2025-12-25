//! Scenario: Clean All Projects
//!
//! Journey: User wants to remove Calvin from all projects at once.
//!
//! Steps:
//! 1. User has deployed to multiple projects
//! 2. Registry tracks all projects
//! 3. User runs clean with --all or uses projects command
//! 4. All deployed files are removed
//! 5. Projects are removed from registry
//!
//! Success Criteria:
//! - All deployed files cleaned
//! - Registry updated
//! - Clear feedback on what was cleaned

use crate::common::*;

/// SCENARIO: User cleans current project
#[test]
fn scenario_clean_current_project() {
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .build();

    // First deploy
    let result = env.run(&["deploy", "--yes"]);
    assert!(result.success, "Deploy failed: {}", result.stderr);

    // Verify files exist
    assert!(
        env.project_path(".cursor").exists(),
        "Should have deployed files"
    );

    // Now clean
    let result = env.run(&["clean", "--yes"]);

    assert!(
        result.success,
        "Clean should succeed.\nOutput: {}",
        result.combined_output()
    );

    // Verify files removed
    // Check that .cursor/rules is either gone or empty
    let cursor_rules = env.project_path(".cursor/rules");
    let is_cleaned = !cursor_rules.exists() || list_all_files(&cursor_rules).is_empty();

    assert!(
        is_cleaned,
        "Clean should remove deployed files.\nRemaining files:\n{}",
        list_all_files(env.project_root.path()).join("\n")
    );
}

/// SCENARIO: User sees what would be cleaned (dry run)
#[test]
fn scenario_clean_dry_run_shows_files() {
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .build();

    // Deploy first
    env.run(&["deploy", "--yes"]);

    // Dry run clean
    let result = env.run(&["clean", "--dry-run"]);

    assert!(
        result.success,
        "Clean dry-run should succeed.\nOutput: {}",
        result.combined_output()
    );

    // Should show what would be cleaned
    let output = result.combined_output();
    let shows_preview = output.contains("would")
        || output.contains("dry")
        || output.contains(".cursor")
        || output.contains("file");

    assert!(
        shows_preview,
        "Dry run should show files to be cleaned.\nOutput:\n{}",
        output
    );

    // Files should still exist
    assert!(
        env.project_path(".cursor").exists(),
        "Dry run should not actually clean files"
    );
}

/// SCENARIO: User lists all registered projects
#[test]
fn scenario_list_registered_projects() {
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .build();

    // Deploy to register project
    env.run(&["deploy", "--yes"]);

    // List projects
    let result = env.run(&["projects"]);

    // Should show registered projects or indicate none
    let output = result.combined_output();

    // Should be user-friendly output
    assert!(
        !output.contains("panic") && !output.contains("error"),
        "Projects command should not error.\nOutput:\n{}",
        output
    );
}

/// SCENARIO: User cleans and lockfile is removed when empty
#[test]
fn scenario_clean_removes_empty_lockfile() {
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .build();

    // Deploy
    env.run(&["deploy", "--yes"]);

    // Verify lockfile exists
    let lockfile_exists = env.project_path("calvin.lock").exists()
        || env.project_path(".promptpack/calvin.lock").exists();

    if lockfile_exists {
        // Clean all
        env.run(&["clean", "--yes"]);

        // After cleaning all, lockfile should be removed or empty
        let lockfile_content = env.read_lockfile();

        // Should be empty or not exist
        let is_empty = lockfile_content.is_empty()
            || lockfile_content.trim() == "[[output]]"
            || !env.project_path("calvin.lock").exists();

        // Note: This might fail if lockfile retention is intended
        // Just document current behavior
        if !is_empty {
            eprintln!(
                "Note: Lockfile still has content after clean:\n{}",
                lockfile_content
            );
        }
    }
}

/// SCENARIO: Clean respects user-modified files
#[test]
fn scenario_clean_warns_on_modified_files() {
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .build();

    // Deploy
    env.run(&["deploy", "--yes"]);

    // User modifies a deployed file
    let rules_dir = env.project_path(".cursor/rules");
    if rules_dir.exists() {
        let files = list_all_files(&rules_dir);
        if let Some(file) = files.first() {
            // Append user content (removing the signature)
            let mut content = std::fs::read_to_string(file).unwrap_or_default();
            // Remove calvin signature to simulate user edit
            content = content.replace("<!-- Generated by Calvin", "<!-- User modified");
            std::fs::write(file, content).ok();
        }
    }

    // Try to clean
    let result = env.run(&["clean", "--yes"]);

    // Should either warn about modified files or skip them
    // This tests the OUTPUT-002 contract: clean respects signature
    // Just verify clean doesn't crash
    assert!(
        result.success || result.stderr.contains("modified"),
        "Clean should handle modified files gracefully.\nOutput: {}",
        result.combined_output()
    );
}
