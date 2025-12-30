//! Integration tests for deploying skills (PRD Phase 3)

mod common;

use common::*;

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

#[test]
fn deploy_writes_binary_skill_assets_and_tracks_in_lockfile() {
    let env = TestEnv::builder().build();

    // Write skill with text content
    write_project_skill(
        &env,
        "with-binary",
        r#"---
description: Skill with binary asset
kind: skill
targets: [codex]
---
# Instructions
"#,
        &[("references/info.md", "# Info\n")],
    );

    // Write a binary asset - must contain NUL bytes to be detected as binary
    // This is a minimal PNG header + some NUL bytes
    let binary_content: [u8; 16] = [
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR chunk length (contains NUL)
        0x49, 0x48, 0x44, 0x52, // "IHDR"
    ];
    env.write_project_binary(
        ".promptpack/skills/with-binary/assets/icon.png",
        &binary_content,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "codex"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    // Verify text assets are deployed
    assert_deployed!(&env, ".codex/skills/with-binary/SKILL.md");
    assert_deployed!(&env, ".codex/skills/with-binary/references/info.md");

    // Verify binary asset is deployed
    let binary_path = env.project_path(".codex/skills/with-binary/assets/icon.png");
    assert!(
        binary_path.exists(),
        "expected binary asset to be deployed at {:?}",
        binary_path
    );

    // Verify binary content is correct
    let deployed_content = std::fs::read(&binary_path).expect("Failed to read deployed binary");
    assert_eq!(
        deployed_content.as_slice(),
        &binary_content[..],
        "deployed binary content should match original"
    );

    // Verify lockfile tracks the binary file
    let lockfile = env.read_lockfile().replace('\\', "/");
    assert!(
        lockfile.contains(".codex/skills/with-binary/assets/icon.png"),
        "expected binary asset entry in lockfile:\n{lockfile}"
    );

    // Verify is_binary flag is set in lockfile
    assert!(
        lockfile.contains("is_binary = true"),
        "expected is_binary flag in lockfile:\n{lockfile}"
    );
}

#[test]
fn deploy_cleanup_removes_orphaned_binary_files() {
    let env = TestEnv::builder().build();

    // Write skill with binary asset
    write_project_skill(
        &env,
        "binary-cleanup",
        r#"---
description: Skill for binary cleanup test
kind: skill
targets: [codex]
---
# Instructions
"#,
        &[],
    );

    // Write a binary asset - must contain NUL bytes to be detected as binary
    let binary_content: [u8; 8] = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46]; // JPEG with NUL
    env.write_project_binary(
        ".promptpack/skills/binary-cleanup/assets/photo.jpg",
        &binary_content,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "codex"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    // Verify binary is deployed
    assert!(
        env.project_path(".codex/skills/binary-cleanup/assets/photo.jpg")
            .exists(),
        "binary should be deployed"
    );

    // Remove the binary from source
    std::fs::remove_file(env.project_path(".promptpack/skills/binary-cleanup/assets/photo.jpg"))
        .unwrap();

    // Deploy with cleanup
    let result = env.run(&["deploy", "--yes", "--targets", "codex", "--cleanup"]);
    assert!(
        result.success,
        "deploy --cleanup failed:\n{}",
        result.combined_output()
    );

    // Verify binary orphan is removed
    assert!(
        !env.project_path(".codex/skills/binary-cleanup/assets/photo.jpg")
            .exists(),
        "orphaned binary should be deleted"
    );

    // Verify empty assets directory is cleaned up
    assert!(
        !env.project_path(".codex/skills/binary-cleanup/assets")
            .exists(),
        "empty assets directory should be removed"
    );
}
