//! Integration tests for deploying skills (PRD Phase 3)

mod common;

use common::*;

fn write_project_skill(env: &TestEnv, id: &str, skill_md: &str, supplementals: &[(&str, &str)]) {
    env.write_project_file(&format!(".promptpack/skills/{}/SKILL.md", id), skill_md);
    for (rel_path, content) in supplementals {
        env.write_project_file(&format!(".promptpack/skills/{}/{}", id, rel_path), content);
    }
}

#[test]
fn deploy_writes_skill_files_and_tracks_each_file_in_lockfile() {
    let env = TestEnv::builder().build();

    write_project_skill(
        &env,
        "my-skill",
        r#"---
description: My skill
kind: skill
targets: [codex]
---
# Instructions
"#,
        &[
            ("reference.md", "# Reference\n"),
            ("scripts/validate.py", "print('ok')\n"),
        ],
    );

    let result = env.run(&["deploy", "--yes", "--targets", "codex"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert_deployed!(&env, ".codex/skills/my-skill/SKILL.md");
    assert_deployed!(&env, ".codex/skills/my-skill/reference.md");
    assert_deployed!(&env, ".codex/skills/my-skill/scripts/validate.py");

    let lockfile = env.read_lockfile().replace('\\', "/");
    assert!(
        lockfile.contains(".codex/skills/my-skill/SKILL.md"),
        "expected SKILL.md entry in lockfile:\n{lockfile}"
    );
    assert!(
        lockfile.contains(".codex/skills/my-skill/reference.md"),
        "expected supplemental entry in lockfile:\n{lockfile}"
    );
    assert!(
        lockfile.contains(".codex/skills/my-skill/scripts/validate.py"),
        "expected script entry in lockfile:\n{lockfile}"
    );
}

#[test]
fn deploy_cleanup_removes_orphaned_skill_files_and_empty_directories() {
    let env = TestEnv::builder().build();

    write_project_skill(
        &env,
        "my-skill",
        r#"---
description: My skill
kind: skill
targets: [codex]
---
# Instructions
"#,
        &[("scripts/validate.py", "print('ok')\n")],
    );

    let result = env.run(&["deploy", "--yes", "--targets", "codex"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert_deployed!(&env, ".codex/skills/my-skill/scripts/validate.py");

    // Remove the supplemental from the source skill.
    std::fs::remove_file(env.project_path(".promptpack/skills/my-skill/scripts/validate.py"))
        .unwrap();

    let result = env.run(&["deploy", "--yes", "--targets", "codex", "--cleanup"]);
    assert!(
        result.success,
        "deploy --cleanup failed:\n{}",
        result.combined_output()
    );

    assert!(
        !env.project_path(".codex/skills/my-skill/scripts/validate.py")
            .exists(),
        "expected orphaned supplemental to be deleted"
    );
    assert!(
        !env.project_path(".codex/skills/my-skill/scripts").exists(),
        "expected empty skill subdirectory to be removed"
    );
}

#[test]
fn deploy_cleanup_removes_empty_skill_directory_when_skill_is_removed() {
    let env = TestEnv::builder().build();

    write_project_skill(
        &env,
        "my-skill",
        r#"---
description: My skill
kind: skill
targets: [codex]
---
# Instructions
"#,
        &[("reference.md", "# Reference\n")],
    );

    let result = env.run(&["deploy", "--yes", "--targets", "codex"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert_deployed!(&env, ".codex/skills/my-skill/SKILL.md");
    assert_deployed!(&env, ".codex/skills/my-skill/reference.md");

    // Remove the entire skill from the source promptpack.
    std::fs::remove_dir_all(env.project_path(".promptpack/skills/my-skill")).unwrap();

    let result = env.run(&["deploy", "--yes", "--targets", "codex", "--cleanup"]);
    assert!(
        result.success,
        "deploy --cleanup failed:\n{}",
        result.combined_output()
    );

    assert!(
        !env.project_path(".codex/skills/my-skill").exists(),
        "expected empty skill directory to be removed"
    );
}

#[test]
fn deploy_warns_when_skill_is_overridden_by_higher_priority_layer() {
    let env = TestEnv::builder().build();

    // User layer skill (lower priority)
    env.write_home_file(
        ".calvin/.promptpack/skills/shared/SKILL.md",
        r#"---
description: Shared skill (user)
kind: skill
targets: [codex]
---
# Instructions (user)
"#,
    );

    // Project layer skill (higher priority) with the same id.
    write_project_skill(
        &env,
        "shared",
        r#"---
description: Shared skill (project)
kind: skill
targets: [codex]
---
# Instructions (project)
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
        result.stderr.contains("overridden") && result.stderr.contains("shared"),
        "expected override warning for skills:\n{}",
        result.combined_output()
    );
}
