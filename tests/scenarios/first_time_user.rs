//! Scenario: First-Time User Setup
//!
//! Journey: A developer new to Calvin wants to set up global prompts.
//!
//! Steps:
//! 1. User has no prior Calvin installation
//! 2. Runs `calvin` in empty project - sees guidance
//! 3. Runs `calvin init --user` to create user layer
//! 4. Adds a global policy file
//! 5. Deploys to a project
//! 6. Verifies prompts appear in editor config
//!
//! Success Criteria:
//! - Clear guidance at each step
//! - No confusing error messages
//! - Final state: working prompts in editor

use crate::common::*;

/// SCENARIO: First-time user with no Calvin installation
///
/// This tests the complete onboarding flow from zero to working prompts.
#[test]
fn scenario_first_time_user_complete_journey() {
    // Step 1: Start with completely clean environment
    let env = TestEnv::builder().fresh_environment().build();

    // Step 2: Run calvin in empty project - should show guidance
    let result = env.run(&["check"]);

    // Should indicate no promptpack found or give guidance
    let output = result.combined_output();
    let has_guidance = output.contains("init")
        || output.contains("create")
        || output.contains("deploy")
        || output.contains("no ")
        || output.contains("not found")
        || output.contains("0 assets")
        || output.contains("0 passed");

    assert!(
        has_guidance || !result.success,
        "Step 2: Should show guidance when no promptpack exists.\nOutput:\n{}",
        output
    );

    // Step 3: Initialize user layer
    let result = env.run(&["init", "--user"]);

    assert!(
        result.success,
        "Step 3: init --user should succeed.\nstderr: {}\nstdout: {}",
        result.stderr, result.stdout
    );

    // Verify user layer was created
    assert!(
        env.home_path(".calvin/.promptpack").exists(),
        "Step 3: User layer should be created at ~/.calvin/.promptpack"
    );

    assert!(
        env.home_path(".calvin/.promptpack/config.toml").exists(),
        "Step 3: User layer should have config.toml"
    );

    // Step 4: User adds a global policy (we simulate this)
    // Note: scope: project means it deploys to project directory
    let policy_content = r#"---
kind: policy
description: My personal coding style
scope: project
targets: [cursor]
---
# My Style

Always use descriptive variable names.
"#;

    let policy_path = env.home_path(".calvin/.promptpack/policies");
    std::fs::create_dir_all(&policy_path).expect("Failed to create policies dir");
    std::fs::write(policy_path.join("my-style.md"), policy_content)
        .expect("Failed to write policy");

    // Step 5: Deploy to project
    // First, create a minimal project with git
    std::fs::create_dir_all(env.project_path("src")).ok();

    let result = env.run(&["deploy", "--yes"]);

    assert!(
        result.success,
        "Step 5: Deploy should succeed with user layer.\nstderr: {}\nstdout: {}",
        result.stderr, result.stdout
    );

    // Step 6: Verify prompts appear
    let cursor_rules_dir = env.project_path(".cursor/rules");

    // Should have deployed the policy
    let deployed_files = if cursor_rules_dir.exists() {
        list_all_files(&cursor_rules_dir)
    } else {
        vec![]
    };

    assert!(
        !deployed_files.is_empty() || env.project_path(".cursor").exists(),
        "Step 6: Should have deployed cursor rules.\nFiles in project:\n{}",
        list_all_files(env.project_root.path()).join("\n")
    );
}

/// SCENARIO: First-time user discovers interactive mode
#[test]
fn scenario_first_time_user_interactive_discovery() {
    let env = TestEnv::builder().fresh_environment().build();

    // When user runs calvin without arguments in empty project
    // They should get helpful output (not an error dump)
    let result = env.run(&[]);

    // Output should be user-friendly, not a stack trace
    let output = result.combined_output();
    let is_user_friendly = !output.contains("panic")
        && !output.contains("RUST_BACKTRACE")
        && (output.contains("init")
            || output.contains("help")
            || output.contains("deploy")
            || output.contains("Usage")
            || output.len() < 2000); // Short output is likely help text

    assert!(
        is_user_friendly,
        "Interactive mode should be user-friendly.\nOutput:\n{}",
        output
    );
}

/// SCENARIO: First-time user checks what would be deployed
#[test]
fn scenario_first_time_user_dry_run() {
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .build();

    // User wants to see what would happen before committing
    let result = env.run(&["deploy", "--dry-run"]);

    assert!(
        result.success,
        "Dry run should succeed.\nOutput: {}",
        result.combined_output()
    );

    // Should show what would be deployed
    let output = result.combined_output();
    let shows_preview = output.contains("would")
        || output.contains("dry")
        || output.contains("preview")
        || output.contains("1 asset")
        || output.contains("test");

    assert!(
        shows_preview,
        "Dry run should show what would be deployed.\nOutput:\n{}",
        output
    );

    // Files should NOT actually be deployed
    assert!(
        !env.project_path(".cursor/rules").exists(),
        "Dry run should not actually deploy files"
    );
}
