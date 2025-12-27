//! Integration tests for skills support (PRD Phase 4)

mod common;

use common::*;

#[test]
fn layers_output_shows_skill_count() {
    let env = TestEnv::builder().build();

    write_project_skill(
        &env,
        "my-skill",
        r#"---
description: My skill
kind: skill
targets: [codex]
---
# Skill
"#,
        &[],
    );

    let result = env.run(&["layers"]);
    assert!(
        result.success,
        "layers failed:\n{}",
        result.combined_output()
    );

    // The UI is free to change, but the presence of non-zero skill counts is a contract.
    assert!(
        result.stdout.contains("skills") && result.stdout.contains("1 skills"),
        "expected skills count in output:\n{}",
        result.stdout
    );
}

#[test]
fn deploy_verbose_shows_skill_structure() {
    let env = TestEnv::builder().build();

    write_project_skill(
        &env,
        "draft-commit",
        r#"---
description: Draft a conventional commit message.
kind: skill
targets: [codex]
allowed-tools:
  - git
---
# Instructions

Use `git diff --cached` and draft a commit message.
"#,
        &[
            ("reference.md", "# Reference\n"),
            ("scripts/validate.py", "print('ok')\n"),
        ],
    );

    let result = env.run(&["deploy", "-v", "--yes", "--targets", "codex"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    let stdout = result.stdout.replace('\\', "/");
    assert!(
        stdout.contains("Skills:") && stdout.contains("draft-commit"),
        "expected Skills section:\n{}",
        stdout
    );
    assert!(
        stdout.contains("SKILL.md"),
        "expected SKILL.md listed:\n{}",
        stdout
    );
    assert!(
        stdout.contains("reference.md"),
        "expected supplemental listed:\n{}",
        stdout
    );
    assert!(
        stdout.contains("scripts/validate.py"),
        "expected script listed:\n{}",
        stdout
    );
}

#[test]
fn check_validates_skills_and_warns_on_dangerous_tools() {
    let env = TestEnv::builder().fresh_environment().build();

    env.write_project_file(
        ".codex/skills/my-skill/SKILL.md",
        r#"---
description: Do something risky.
kind: skill
allowed-tools:
  - rm
---
# Instructions
"#,
    );

    let result = env.run(&["check"]);
    assert!(
        result.success,
        "check failed:\n{}",
        result.combined_output()
    );

    assert!(
        result.stdout.contains("Codex"),
        "expected Codex section:\n{}",
        result.stdout
    );
    assert!(
        result.stdout.contains("skills - 1 skills"),
        "expected skills check to count skills:\n{}",
        result.stdout
    );
    assert!(
        result
            .stdout
            .contains("uses 'rm' in allowed-tools (security warning)"),
        "expected allowed-tools warning:\n{}",
        result.stdout
    );
}

#[test]
fn deploy_warns_when_skill_targets_unsupported_platforms() {
    let env = TestEnv::builder().build();

    write_project_skill(
        &env,
        "platform-skill",
        r#"---
description: Targets unsupported platform.
kind: skill
targets: [codex, antigravity]
---
# Instructions
"#,
        &[],
    );

    let result = env.run(&["deploy", "--yes", "--targets", "codex"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert!(
        result
            .stderr
            .contains("skills are not supported on this platform; skipping"),
        "expected warning about unsupported platform:\n{}",
        result.combined_output()
    );
}

#[test]
fn deploy_warns_on_dangerous_allowed_tools_during_deploy() {
    let env = TestEnv::builder().build();

    write_project_skill(
        &env,
        "dangerous-skill",
        r#"---
description: Do something risky.
kind: skill
targets: [codex]
allowed-tools:
  - rm
---
# Instructions
"#,
        &[],
    );

    let result = env.run(&["deploy", "--yes", "--targets", "codex"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert!(
        result
            .stderr
            .contains("Tool 'rm' in allowed-tools may pose security risks"),
        "expected allowed-tools warning during deploy:\n{}",
        result.combined_output()
    );
}

#[test]
#[allow(non_snake_case)] // naming convention: `<original_test_name>__<variant_type>`
fn deploy_warns_on_dangerous_allowed_tools_during_deploy__with_no_allowed_tools() {
    let env = TestEnv::builder().build();

    write_project_skill(
        &env,
        "safe-skill",
        r#"---
description: Safe skill.
kind: skill
targets: [codex]
---
# Instructions
"#,
        &[],
    );

    let result = env.run(&["deploy", "--yes", "--targets", "codex"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert!(
        !result
            .stderr
            .contains("in allowed-tools may pose security risks"),
        "did not expect allowed-tools warning during deploy:\n{}",
        result.combined_output()
    );
}

#[test]
#[allow(non_snake_case)] // naming convention: `<original_test_name>__<variant_type>`
fn deploy_warns_on_dangerous_allowed_tools_during_deploy__with_multiple_dangerous_tools() {
    let env = TestEnv::builder().build();

    write_project_skill(
        &env,
        "multi-danger",
        r#"---
description: Do multiple risky things.
kind: skill
targets: [codex]
allowed-tools:
  - rm
  - sudo
---
# Instructions
"#,
        &[],
    );

    let result = env.run(&["deploy", "--yes", "--targets", "codex"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert!(
        result
            .stderr
            .contains("Tool 'rm' in allowed-tools may pose security risks"),
        "expected 'rm' warning during deploy:\n{}",
        result.combined_output()
    );
    assert!(
        result
            .stderr
            .contains("Tool 'sudo' in allowed-tools may pose security risks"),
        "expected 'sudo' warning during deploy:\n{}",
        result.combined_output()
    );
}

#[test]
#[allow(non_snake_case)] // naming convention: `<original_test_name>__<variant_type>`
fn deploy_warns_on_dangerous_allowed_tools_during_deploy__when_skill_not_deployed_to_target() {
    let env = TestEnv::builder().build();

    // Skill targets Claude Code only, but we deploy to Codex; there should be no SKILL.md output
    // for Codex and therefore no deploy-time allowed-tools warning for Codex.
    write_project_skill(
        &env,
        "claude-only",
        r#"---
description: Targets Claude Code only.
kind: skill
targets: [claude-code]
allowed-tools:
  - rm
---
# Instructions
"#,
        &[],
    );

    let result = env.run(&["deploy", "--yes", "--targets", "codex"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert!(
        !result
            .stderr
            .contains("in allowed-tools may pose security risks"),
        "did not expect allowed-tools warning when skill is not deployed to codex:\n{}",
        result.combined_output()
    );
}

#[test]
fn deploy_warns_when_deploy_targets_include_platforms_without_skill_support() {
    let env = TestEnv::builder().build();

    // Targets omitted => skill applies to all concrete targets, including unsupported ones.
    write_project_skill(
        &env,
        "all-targets-skill",
        r#"---
description: Applies to all targets.
kind: skill
---
# Instructions
"#,
        &[],
    );

    let result = env.run(&["deploy", "--yes", "--targets", "all"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert!(
        result.stderr.contains("Skills skipped for:"),
        "expected global skills skip warning:\n{}",
        result.combined_output()
    );
    assert!(
        result.stderr.contains("VS Code") && result.stderr.contains("Antigravity"),
        "expected unsupported platforms in warning:\n{}",
        result.combined_output()
    );
}

#[test]
#[allow(non_snake_case)] // naming convention: `<original_test_name>__<variant_type>`
fn deploy_warns_when_deploy_targets_include_platforms_without_skill_support__without_skills() {
    let env = TestEnv::builder().build();

    let result = env.run(&["deploy", "--yes", "--targets", "all"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert!(
        !result.stderr.contains("Skills skipped for:"),
        "did not expect skills skip warning when no skills exist:\n{}",
        result.combined_output()
    );
}

#[test]
#[allow(non_snake_case)] // naming convention: `<original_test_name>__<variant_type>`
fn deploy_warns_when_deploy_targets_include_platforms_without_skill_support__with_skill_targets_supported_only(
) {
    let env = TestEnv::builder().build();

    write_project_skill(
        &env,
        "codex-only",
        r#"---
description: Targets Codex only.
kind: skill
targets: [codex]
---
# Instructions
"#,
        &[],
    );

    let result = env.run(&["deploy", "--yes", "--targets", "all"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert!(
        !result.stderr.contains("Skills skipped for:"),
        "did not expect skills skip warning for codex-only skill:\n{}",
        result.combined_output()
    );
}

#[test]
#[allow(non_snake_case)] // naming convention: `<original_test_name>__<variant_type>`
fn deploy_warns_when_deploy_targets_include_platforms_without_skill_support__with_single_unsupported_target(
) {
    let env = TestEnv::builder().build();

    write_project_skill(
        &env,
        "all-targets",
        r#"---
description: Applies to all targets.
kind: skill
---
# Instructions
"#,
        &[],
    );

    // Clap ValueEnum uses `vs-code` (config parsing also accepts `vscode`).
    let result = env.run(&["deploy", "--yes", "--targets", "vs-code"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert!(
        result.stderr.contains("Skills skipped for:") && result.stderr.contains("VS Code"),
        "expected VS Code skills skip warning:\n{}",
        result.combined_output()
    );
    assert!(
        !result.stderr.contains("Antigravity"),
        "did not expect Antigravity in warning when deploying to vscode only:\n{}",
        result.combined_output()
    );
}

#[test]
#[allow(non_snake_case)] // naming convention: `<original_test_name>__<variant_type>`
fn deploy_warns_when_deploy_targets_include_platforms_without_skill_support__with_duplicate_deploy_targets(
) {
    let env = TestEnv::builder().build();

    write_project_skill(
        &env,
        "all-targets",
        r#"---
description: Applies to all targets.
kind: skill
---
# Instructions
"#,
        &[],
    );

    // Clap ValueEnum uses `vs-code` (config parsing also accepts `vscode`).
    let result = env.run(&["deploy", "--yes", "--targets", "vs-code,vs-code"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert!(
        result.stderr.contains("Skills skipped for:") && result.stderr.contains("VS Code"),
        "expected VS Code skills skip warning:\n{}",
        result.combined_output()
    );
    assert!(
        !result.stderr.contains("VS Code, VS Code"),
        "expected deploy targets to be deduplicated in warning:\n{}",
        result.combined_output()
    );
}

#[test]
#[allow(non_snake_case)] // naming convention: `<original_test_name>__<variant_type>`
fn deploy_warns_when_deploy_targets_include_platforms_without_skill_support__with_no_deploy_targets(
) {
    let env = TestEnv::builder()
        .with_project_config(CONFIG_EMPTY_TARGETS)
        .build();

    write_project_skill(
        &env,
        "all-targets",
        r#"---
description: Applies to all targets.
kind: skill
---
# Instructions
"#,
        &[],
    );

    let result = env.run(&["deploy", "--yes"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert!(
        !result.stderr.contains("Skills skipped for:"),
        "did not expect skills skip warning when no deploy targets are enabled:\n{}",
        result.combined_output()
    );
}
