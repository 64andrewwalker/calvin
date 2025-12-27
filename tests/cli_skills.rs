//! Integration tests for skills support (PRD Phase 4)

mod common;

use common::*;

fn write_project_skill(env: &TestEnv, id: &str, skill_md: &str, supplementals: &[(&str, &str)]) {
    env.write_project_file(&format!(".promptpack/skills/{}/SKILL.md", id), skill_md);
    for (rel_path, content) in supplementals {
        env.write_project_file(&format!(".promptpack/skills/{}/{}", id, rel_path), content);
    }
}

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
