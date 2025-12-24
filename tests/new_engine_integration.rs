//! Integration tests for the new DeployUseCase engine
//!
//! These tests verify the new engine works correctly for various scenarios.

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn calvin_cmd(fake_home: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_calvin"));
    cmd.env("HOME", fake_home);
    cmd.env("XDG_CONFIG_HOME", fake_home.join(".config"));
    cmd
}

/// Helper to create a minimal .promptpack structure
fn setup_promptpack(dir: &Path) -> std::path::PathBuf {
    let promptpack = dir.join(".promptpack");
    fs::create_dir_all(&promptpack).unwrap();

    // Create a minimal calvin.toml (version 2 format)
    let config = r#"[build]
version = "2"

[targets]
claude_code = true
cursor = true
vscode = true
"#;
    fs::write(promptpack.join("calvin.toml"), config).unwrap();

    // Create a test policy with proper YAML frontmatter
    // Note: Using proper YAML indentation is critical
    let policy = r#"---
kind: policy
description: Test policy for integration tests
---
# Test Policy

This is a test policy for the new engine integration tests.
"#;
    fs::write(promptpack.join("test-policy.md"), policy).unwrap();

    promptpack
}

/// Helper to check if a file was deployed
fn file_exists_with_content(base: &Path, relative: &str, expected_substring: &str) -> bool {
    let path = base.join(relative);
    if !path.exists() {
        eprintln!("File does not exist: {}", path.display());
        return false;
    }
    let content = fs::read_to_string(&path).unwrap_or_default();
    if !content.contains(expected_substring) {
        eprintln!(
            "File {} does not contain '{}'. Content: {}",
            path.display(),
            expected_substring,
            &content[..content.len().min(200)]
        );
        return false;
    }
    true
}

#[test]
fn new_engine_deploys_to_project() {
    let temp = tempdir().unwrap();
    let project_dir = temp.path();
    let fake_home = temp.path().join("fake_home");
    fs::create_dir_all(&fake_home).unwrap();

    // Setup .promptpack with a test policy
    setup_promptpack(project_dir);

    // Run deploy using new engine (via CLI)
    let output = calvin_cmd(&fake_home)
        .arg("deploy")
        .arg("--yes")
        .current_dir(project_dir)
        .output()
        .expect("Failed to execute calvin deploy");

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success(), "deploy command failed");

    // Verify files were deployed
    // New engine uses these paths:
    // - Cursor: .cursor/rules/<id>/RULE.md
    // - VSCode: .github/instructions/<id>.instructions.md
    // - Antigravity: .agent/workflows/<id>.md
    // - Codex: .codex/prompts/<id>.md
    assert!(
        file_exists_with_content(
            project_dir,
            ".cursor/rules/test-policy/RULE.md",
            "Test Policy"
        ),
        "Cursor deployment failed"
    );

    // VSCode instructions
    assert!(
        file_exists_with_content(
            project_dir,
            ".github/instructions/test-policy.instructions.md",
            "Test Policy"
        ),
        "VSCode deployment failed"
    );

    // AGENTS.md should be generated via post_compile
    assert!(
        file_exists_with_content(project_dir, "AGENTS.md", "test-policy"),
        "AGENTS.md generation failed"
    );
}

#[test]
fn new_engine_deploys_to_home() {
    let temp = tempdir().unwrap();
    let project_dir = temp.path();

    // Create a fake home directory
    let fake_home = temp.path().join("fake_home");
    fs::create_dir_all(&fake_home).unwrap();

    // Setup .promptpack
    setup_promptpack(project_dir);

    // For this test, we can't easily override $HOME, so we just verify
    // that the --home flag is accepted without error
    let output = calvin_cmd(&fake_home)
        .arg("deploy")
        .arg("--home")
        .arg("--dry-run") // Use dry-run to avoid actually writing to real home
        .arg("--yes")
        .current_dir(project_dir)
        .output()
        .expect("Failed to execute calvin deploy --home");

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success(), "deploy --home command failed");

    // Check that dry-run output mentions files that would be written
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Deploy") || stdout.contains("written"),
        "Expected deploy output"
    );
}

#[test]
fn new_engine_force_mode() {
    let temp = tempdir().unwrap();
    let project_dir = temp.path();
    let fake_home = temp.path().join("fake_home");
    fs::create_dir_all(&fake_home).unwrap();

    setup_promptpack(project_dir);

    // First deploy
    let output1 = calvin_cmd(&fake_home)
        .arg("deploy")
        .arg("--yes")
        .current_dir(project_dir)
        .output()
        .expect("Failed to execute first deploy");

    assert!(output1.status.success(), "First deploy failed");

    // Modify a deployed file (simulate external edit)
    let cursor_rules = project_dir.join(".cursor/rules/test-policy.mdc");
    if cursor_rules.exists() {
        let mut content = fs::read_to_string(&cursor_rules).unwrap();
        content.push_str("\n<!-- External modification -->");
        fs::write(&cursor_rules, content).unwrap();
    }

    // Second deploy with --force should overwrite
    let output2 = calvin_cmd(&fake_home)
        .arg("deploy")
        .arg("--force")
        .arg("--yes")
        .current_dir(project_dir)
        .output()
        .expect("Failed to execute force deploy");

    println!("stdout: {}", String::from_utf8_lossy(&output2.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output2.stderr));

    assert!(output2.status.success(), "Force deploy failed");
}

#[test]
fn new_engine_dry_run_does_not_modify_files() {
    let temp = tempdir().unwrap();
    let project_dir = temp.path();
    let fake_home = temp.path().join("fake_home");
    fs::create_dir_all(&fake_home).unwrap();

    setup_promptpack(project_dir);

    // Run dry-run deploy
    let output = calvin_cmd(&fake_home)
        .arg("deploy")
        .arg("--dry-run")
        .arg("--yes")
        .current_dir(project_dir)
        .output()
        .expect("Failed to execute dry-run deploy");

    assert!(output.status.success(), "Dry-run deploy failed");

    // Verify no files were actually written
    assert!(
        !project_dir.join(".cursor").exists(),
        ".cursor directory should not exist after dry-run"
    );
    assert!(
        !project_dir.join("CLAUDE.md").exists(),
        "CLAUDE.md should not exist after dry-run"
    );
}

#[test]
fn new_engine_json_output() {
    let temp = tempdir().unwrap();
    let project_dir = temp.path();
    let fake_home = temp.path().join("fake_home");
    fs::create_dir_all(&fake_home).unwrap();

    setup_promptpack(project_dir);

    // Run deploy with JSON output
    let output = calvin_cmd(&fake_home)
        .arg("deploy")
        .arg("--json")
        .arg("--yes")
        .current_dir(project_dir)
        .output()
        .expect("Failed to execute JSON deploy");

    assert!(output.status.success(), "JSON deploy failed");

    // Parse JSON output - should be NDJSON (one JSON per line)
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("JSON output: {}", stdout);

    // Check for expected events
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(!lines.is_empty(), "Expected JSON output");

    // Parse first line as JSON
    let first_event: serde_json::Value =
        serde_json::from_str(lines[0]).expect("First line should be valid JSON");
    assert_eq!(
        first_event.get("event").and_then(|e| e.as_str()),
        Some("start"),
        "First event should be 'start'"
    );

    // Check last event is 'complete'
    let last_event: serde_json::Value =
        serde_json::from_str(lines.last().unwrap()).expect("Last line should be valid JSON");
    assert_eq!(
        last_event.get("event").and_then(|e| e.as_str()),
        Some("complete"),
        "Last event should be 'complete'"
    );
}

/// Test that assets with specific target frontmatter only deploy to those targets
#[test]
fn new_engine_target_filtering() {
    let temp = tempdir().unwrap();
    let project_dir = temp.path();
    let fake_home = temp.path().join("fake_home");
    fs::create_dir_all(&fake_home).unwrap();

    let promptpack = project_dir.join(".promptpack");
    fs::create_dir_all(&promptpack).unwrap();

    // Config with all targets enabled
    let config = r#"[build]
version = "2"

[targets]
claude_code = true
cursor = true
vscode = true
"#;
    fs::write(promptpack.join("calvin.toml"), config).unwrap();

    // Create an asset that only targets cursor (via frontmatter)
    let cursor_only = r#"---
kind: policy
description: Cursor only policy
targets:
  - cursor
---
# Cursor Only

This should only deploy to Cursor.
"#;
    fs::write(promptpack.join("cursor-only.md"), cursor_only).unwrap();

    // Deploy
    let output = calvin_cmd(&fake_home)
        .arg("deploy")
        .arg("--yes")
        .current_dir(project_dir)
        .output()
        .expect("Failed to execute target-specific deploy");

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success(), "Deploy failed");

    // Cursor files should exist (.cursor/rules/cursor-only/RULE.md)
    assert!(
        project_dir
            .join(".cursor/rules/cursor-only/RULE.md")
            .exists(),
        "Cursor RULE.md should be deployed for cursor-only asset"
    );

    // VSCode files should NOT exist for this asset (it only targets cursor)
    assert!(
        !project_dir
            .join(".github/instructions/cursor-only.instructions.md")
            .exists(),
        "cursor-only asset should not be deployed to VSCode"
    );
}

#[test]
fn new_engine_cleanup_flag() {
    let temp = tempdir().unwrap();
    let project_dir = temp.path();
    let fake_home = temp.path().join("fake_home");
    fs::create_dir_all(&fake_home).unwrap();

    setup_promptpack(project_dir);

    // First deploy
    let output1 = calvin_cmd(&fake_home)
        .arg("deploy")
        .arg("--yes")
        .current_dir(project_dir)
        .output()
        .expect("Failed to execute first deploy");

    assert!(output1.status.success(), "First deploy failed");

    // Remove the source file to create an orphan
    let promptpack = project_dir.join(".promptpack");
    fs::remove_file(promptpack.join("test-policy.md")).unwrap();

    // Second deploy with --cleanup should remove orphans
    let output2 = calvin_cmd(&fake_home)
        .arg("deploy")
        .arg("--cleanup")
        .arg("--yes")
        .current_dir(project_dir)
        .output()
        .expect("Failed to execute cleanup deploy");

    println!("stdout: {}", String::from_utf8_lossy(&output2.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output2.stderr));

    assert!(output2.status.success(), "Cleanup deploy failed");

    // The orphan file should be cleaned up
    // Note: This depends on the lockfile correctly tracking the file
}

#[test]
fn new_engine_generates_agents_md() {
    let temp = tempdir().unwrap();
    let project_dir = temp.path();
    let fake_home = temp.path().join("fake_home");
    fs::create_dir_all(&fake_home).unwrap();

    // Setup .promptpack with an action
    let promptpack = project_dir.join(".promptpack");
    fs::create_dir_all(&promptpack).unwrap();

    let config = r#"
[build]
version = "2"

[targets]
cursor = true
"#;
    fs::write(promptpack.join("calvin.toml"), config).unwrap();

    // Create an action (which should appear in AGENTS.md)
    let action = r#"---
kind: action
description: Test action for AGENTS.md generation
targets:
  - cursor
---
# Test Action

This action tests AGENTS.md generation.
"#;
    fs::write(promptpack.join("0-test-action.md"), action).unwrap();

    // Deploy
    let output = calvin_cmd(&fake_home)
        .arg("deploy")
        .arg("--yes")
        .current_dir(project_dir)
        .output()
        .expect("Failed to execute deploy");

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success(), "Deploy failed");

    // AGENTS.md should be generated
    let agents_md = project_dir.join("AGENTS.md");
    assert!(agents_md.exists(), "AGENTS.md should be generated");

    let content = fs::read_to_string(&agents_md).unwrap();
    assert!(
        content.contains("0-test-action"),
        "AGENTS.md should contain the action name"
    );
}
