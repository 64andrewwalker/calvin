//! Regression test for .calvinignore not applied during deploy.
//!
//! REGRESSION: .calvinignore patterns were not applied during deploy
//!
//! Bug: Running `calvin deploy` deployed assets that should have been
//! ignored by .calvinignore patterns. The `.calvinignore` file worked
//! for display commands (like `calvin layers --verbose`) but NOT for
//! actual deployment.
//!
//! Root cause: DeployUseCase used AssetRepository.load_all() which
//! did not apply .calvinignore filtering. Only LayerLoader.load_layer_assets()
//! applied the filtering, causing a divergence between display and deploy paths.
//!
//! Related contract: contract_calvinignore_excludes_skill_directories

use crate::common::TestEnv;

/// Skill content for testing
const SKILL_CONTENT: &str = r#"---
description: Test skill
scope: project
targets: [claude-code, cursor]
---
# Test Skill

Instructions for the skill.
"#;

/// REGRESSION: skills/_template/ was deployed despite .calvinignore
///
/// This is the exact reproduction case from the user report:
/// - .promptpack/skills/_template/ exists (as a template for new skills)
/// - .promptpack/.calvinignore contains "skills/_template/"
/// - User runs `calvin deploy` (interactive mode, selecting Cursor target)
/// - Expected: .claude/skills/_template should NOT exist
/// - Actual (bug): .claude/skills/_template was created
#[test]
fn regression_skills_template_deployed_despite_calvinignore() {
    let env = TestEnv::builder()
        .with_calvinignore("skills/_template/\n")
        .build();

    // Create skills using the helper - exactly as in the user's setup
    crate::common::write_project_skill(
        &env,
        "_template",
        SKILL_CONTENT,
        &[("README.md", "# Template instructions")],
    );
    crate::common::write_project_skill(&env, "real-skill", SKILL_CONTENT, &[]);

    // Deploy to Cursor (as the user did)
    let result = env.run(&["deploy", "--targets", "cursor"]);

    assert!(
        result.is_success(),
        "Deploy should succeed. stderr: {}",
        result.stderr
    );

    // The real skill should be deployed
    assert!(
        env.project_path(".claude/skills/real-skill").exists(),
        "real-skill should be deployed to .claude/skills/"
    );

    // The _template skill should NOT be deployed
    assert!(
        !env.project_path(".claude/skills/_template").exists(),
        "REGRESSION: _template skill was deployed despite .calvinignore pattern 'skills/_template/'\n\
         This was fixed in the asset loading unification refactor.\n\
         stderr: {}",
        result.stderr
    );
}

/// REGRESSION: Interactive mode should also respect .calvinignore
///
/// The bug manifested in interactive mode specifically because it used
/// a different code path than CLI mode with explicit --targets flag.
#[test]
fn regression_interactive_mode_respects_calvinignore() {
    // Note: We can't truly test interactive mode in a non-interactive test,
    // but we can verify that the same deploy options produce correct results.
    // The fix ensures both interactive and CLI modes use the same loading path.

    let env = TestEnv::builder()
        .with_project_asset(
            "draft.md",
            r#"---
kind: policy
description: Draft policy
scope: project
targets: [cursor]
---
# Draft

Work in progress.
"#,
        )
        .with_project_asset(
            "final.md",
            r#"---
kind: policy
description: Final policy
scope: project
targets: [cursor]
---
# Final

Ready for use.
"#,
        )
        .with_calvinignore("draft.md\n")
        .build();

    // Use --yes to auto-confirm (simulates interactive confirmation)
    let result = env.run(&["deploy", "--targets", "cursor", "--yes"]);

    assert!(
        result.is_success(),
        "Deploy should succeed. stderr: {}",
        result.stderr
    );

    // Only final.md should be deployed (Cursor uses .cursor/rules/<id>/RULE.md)
    assert!(
        env.project_path(".cursor/rules/final/RULE.md").exists(),
        "final.md should be deployed"
    );
    assert!(
        !env.project_path(".cursor/rules/draft").exists(),
        "REGRESSION: draft.md was deployed despite .calvinignore pattern\n\
         stderr: {}",
        result.stderr
    );
}
