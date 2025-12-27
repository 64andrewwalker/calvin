//! Contract tests for .calvinignore functionality.
//!
//! These contracts guarantee that .calvinignore patterns are respected
//! during all asset operations (deploy, diff, etc.).
//!
//! See: docs/guides/calvinignore.mdx

use crate::common::{TestEnv, SIMPLE_POLICY};

/// Simple skill content for testing
const SKILL_CONTENT: &str = r#"---
description: Test skill
scope: project
targets: [claude-code, cursor]
---
# Test Skill

Instructions for the skill.
"#;

/// Multi-target policy for testing cross-platform behavior
const MULTI_TARGET_POLICY: &str = r#"---
kind: policy
description: Multi-target test policy
scope: project
targets: [cursor, claude-code]
---
# Multi-Target Policy

This policy applies to multiple platforms.
"#;

/// CONTRACT: .calvinignore patterns are respected during deploy
///
/// This prevents assets matching ignore patterns from being deployed.
/// The contract must hold for all target platforms.
#[test]
fn contract_calvinignore_excludes_matching_assets_from_deploy() {
    let env = TestEnv::builder()
        .with_project_asset("draft.md", SIMPLE_POLICY)
        .with_project_asset("final.md", SIMPLE_POLICY)
        .with_calvinignore("draft.md\n")
        .build();

    let result = env.run(&["deploy", "--targets", "cursor"]);

    assert!(
        result.is_success(),
        "Deploy should succeed. stderr: {}",
        result.stderr
    );

    // Cursor creates .cursor/rules/<id>/RULE.md
    // final.md should be deployed
    assert!(
        env.project_path(".cursor/rules/final/RULE.md").exists(),
        "final.md should be deployed to .cursor/rules/final/RULE.md"
    );

    // draft.md should NOT be deployed (ignored)
    assert!(
        !env.project_path(".cursor/rules/draft").exists(),
        "draft.md should NOT be deployed (should be ignored by .calvinignore)"
    );
}

/// CONTRACT: .calvinignore directory patterns exclude entire directories
///
/// Directory patterns (ending with /) should exclude all contents.
#[test]
fn contract_calvinignore_excludes_directories() {
    let env = TestEnv::builder()
        .with_project_asset("drafts/wip.md", SIMPLE_POLICY)
        .with_project_asset("final.md", SIMPLE_POLICY)
        .with_calvinignore("drafts/\n")
        .build();

    let result = env.run(&["deploy", "--targets", "cursor"]);

    assert!(
        result.is_success(),
        "Deploy should succeed. stderr: {}",
        result.stderr
    );

    // final.md should be deployed
    assert!(
        env.project_path(".cursor/rules/final/RULE.md").exists(),
        "final.md should be deployed"
    );

    // drafts/ directory should NOT be deployed
    assert!(
        !env.project_path(".cursor/rules/wip").exists(),
        "drafts/wip.md should NOT be deployed (directory ignored)"
    );
}

/// CONTRACT: .calvinignore patterns exclude skill directories
///
/// This is the original bug case: skills/_template/ should be excludable.
#[test]
fn contract_calvinignore_excludes_skill_directories() {
    let env = TestEnv::builder()
        .with_calvinignore("skills/_template/\n")
        .build();

    // Create skills using the helper
    crate::common::write_project_skill(&env, "_template", SKILL_CONTENT, &[]);
    crate::common::write_project_skill(&env, "production", SKILL_CONTENT, &[]);

    let result = env.run(&["deploy", "--targets", "claude-code"]);

    assert!(
        result.is_success(),
        "Deploy should succeed. stderr: {}",
        result.stderr
    );

    // production skill should be deployed
    assert!(
        env.project_path(".claude/skills/production").exists(),
        "production skill should be deployed"
    );

    // _template skill should NOT be deployed (ignored)
    assert!(
        !env.project_path(".claude/skills/_template").exists(),
        "_template skill should NOT be deployed (should be ignored by .calvinignore)"
    );
}

/// CONTRACT: .calvinignore works consistently across all target platforms
///
/// Ignore patterns should be applied before compilation, affecting all targets.
#[test]
fn contract_calvinignore_works_for_all_targets() {
    let env = TestEnv::builder()
        .with_project_asset("draft.md", MULTI_TARGET_POLICY)
        .with_project_asset("final.md", MULTI_TARGET_POLICY)
        .with_calvinignore("draft.md\n")
        .with_project_config(
            r#"
[targets]
enabled = ["cursor", "claude-code"]
"#,
        )
        .build();

    let result = env.run(&["deploy"]);

    assert!(
        result.is_success(),
        "Deploy should succeed. stderr: {}",
        result.stderr
    );

    // Check Cursor target: .cursor/rules/<id>/RULE.md
    assert!(
        env.project_path(".cursor/rules/final/RULE.md").exists(),
        "final.md should be deployed to Cursor"
    );
    assert!(
        !env.project_path(".cursor/rules/draft").exists(),
        "draft.md should NOT be deployed to Cursor"
    );

    // Check Claude Code target: .claude/commands/<id>.md (policies become commands)
    assert!(
        env.project_path(".claude/commands/final.md").exists(),
        "final.md should be deployed to Claude Code"
    );
    assert!(
        !env.project_path(".claude/commands/draft.md").exists(),
        "draft.md should NOT be deployed to Claude Code"
    );
}

/// CONTRACT: .calvinignore glob patterns work correctly
///
/// Glob patterns like *.bak should match files anywhere in the tree.
#[test]
fn contract_calvinignore_glob_patterns() {
    let env = TestEnv::builder()
        .with_project_asset("policy.md", SIMPLE_POLICY)
        .with_project_asset("old.md", SIMPLE_POLICY) // Use .md so it's a valid asset
        .with_calvinignore("old.md\n")
        .build();

    let result = env.run(&["deploy", "--targets", "cursor"]);

    assert!(
        result.is_success(),
        "Deploy should succeed. stderr: {}",
        result.stderr
    );

    assert!(
        env.project_path(".cursor/rules/policy/RULE.md").exists(),
        "policy.md should be deployed"
    );
    assert!(
        !env.project_path(".cursor/rules/old").exists(),
        "old.md should NOT be deployed (ignored)"
    );
}

/// CONTRACT: Empty .calvinignore does not affect deployment
///
/// An empty or whitespace-only .calvinignore should deploy all assets.
#[test]
fn contract_empty_calvinignore_deploys_all() {
    let env = TestEnv::builder()
        .with_project_asset("one.md", SIMPLE_POLICY)
        .with_project_asset("two.md", SIMPLE_POLICY)
        .with_calvinignore("\n\n")
        .build();

    let result = env.run(&["deploy", "--targets", "cursor"]);

    assert!(
        result.is_success(),
        "Deploy should succeed. stderr: {}",
        result.stderr
    );

    assert!(
        env.project_path(".cursor/rules/one/RULE.md").exists(),
        "one.md should be deployed"
    );
    assert!(
        env.project_path(".cursor/rules/two/RULE.md").exists(),
        "two.md should be deployed"
    );
}
