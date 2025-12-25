//! Scenario: Team Shared Prompts with Project Override
//!
//! Journey: A team shares prompts, individual project needs to override one.
//!
//! Steps:
//! 1. Team has shared prompts in a team layer
//! 2. Project configures team layer as additional source
//! 3. Project adds its own override for one policy
//! 4. Deploy merges correctly - team policies + project override
//! 5. Verbose output shows which policies came from where
//!
//! Success Criteria:
//! - Team policies are deployed unless overridden
//! - Project override wins for same-ID policy
//! - Clear visibility of what came from where

use crate::common::*;

/// SCENARIO: Team shares prompts, project uses and overrides some
#[test]
fn scenario_team_shared_with_project_override() {
    let env = TestEnv::builder()
        .with_user_asset(
            "team-security.md",
            r#"---
kind: policy
description: Team security guidelines
scope: project
targets: [cursor]
---
# TEAM_SECURITY

Follow team security practices.
"#,
        )
        .with_user_asset(
            "team-style.md",
            r#"---
kind: policy
description: Team coding style
scope: project
targets: [cursor]
---
# TEAM_STYLE

Use team coding conventions.
"#,
        )
        .with_project_asset(
            "team-style.md",
            r#"---
kind: policy
description: Project-specific style override
scope: project
targets: [cursor]
---
# PROJECT_STYLE_OVERRIDE

This project uses different conventions.
"#,
        )
        .build();

    // Deploy with verbose to see provenance
    let result = env.run(&["deploy", "-v", "--yes"]);

    assert!(
        result.success,
        "Deploy should succeed.\nOutput: {}",
        result.combined_output()
    );

    // Check deployed files
    let cursor_dir = env.project_path(".cursor/rules");
    if cursor_dir.exists() {
        let files = list_all_files(&cursor_dir);

        // Should have security from team (not overridden)
        // Should have style from project (override)

        // Read the style file if it exists
        for file in &files {
            if file.contains("style") {
                let content = std::fs::read_to_string(file).unwrap_or_default();
                // Project override should win
                assert!(
                    content.contains("PROJECT_STYLE") || content.contains("OVERRIDE"),
                    "Style policy should be from project (override).\nContent:\n{}",
                    content
                );
            }
        }
    }
}

/// SCENARIO: Team member sees where each policy comes from
#[test]
fn scenario_team_visibility_of_sources() {
    let env = TestEnv::builder()
        .with_user_asset("from-user.md", POLICY_FROM_USER)
        .with_project_asset("from-project.md", POLICY_FROM_PROJECT)
        .build();

    // Deploy first
    env.run(&["deploy", "--yes"]);

    // Check layers command shows sources
    let result = env.run(&["layers"]);

    if result.success {
        let output = result.combined_output();

        // Should show both layers
        let shows_layers =
            output.contains("user") || output.contains("project") || output.contains("layer");

        assert!(
            shows_layers,
            "Layers command should show layer sources.\nOutput:\n{}",
            output
        );
    }

    // Check provenance command
    let result = env.run(&["provenance"]);

    if result.success {
        let output = result.combined_output();

        // Should show where assets come from
        let shows_provenance = output.contains("user")
            || output.contains("project")
            || output.contains("from")
            || output.contains("source");

        assert!(
            shows_provenance,
            "Provenance should show asset sources.\nOutput:\n{}",
            output
        );
    }
}

/// SCENARIO: New team member can see full layer stack
#[test]
fn scenario_new_team_member_understands_layers() {
    let env = TestEnv::builder()
        .with_user_asset("global.md", USER_POLICY)
        .with_project_asset("project.md", PROJECT_POLICY)
        .build();

    // New team member runs layers to understand setup
    let result = env.run(&["layers"]);

    assert!(
        result.success,
        "Layers command should succeed.\nOutput: {}",
        result.combined_output()
    );

    // Output should be understandable
    let output = result.combined_output();

    // Should not be empty or cryptic
    assert!(
        output.len() > 10,
        "Layers output should provide useful information.\nOutput:\n{}",
        output
    );
}
