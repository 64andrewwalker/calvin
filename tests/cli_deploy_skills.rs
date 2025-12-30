//! Integration tests for deploying skills (PRD Phase 3)
//!
//! Variant tests use double underscore naming convention per docs/testing/test-variants-rationale.md
#![allow(non_snake_case)]

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

// === Binary Skill Test Variants ===

/// Input boundary: binary file with minimal content (just NUL bytes)
#[test]
fn deploy_writes_binary_skill_assets__with_single_byte_binary() {
    let env = TestEnv::builder().build();

    write_project_skill(
        &env,
        "min-binary",
        r#"---
description: Skill with minimal binary
kind: skill
targets: [codex]
---
# Instructions
"#,
        &[],
    );

    // Single NUL byte is the minimum binary content
    env.write_project_binary(".promptpack/skills/min-binary/assets/tiny.bin", &[0x00]);

    let result = env.run(&["deploy", "--yes", "--targets", "codex"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    let binary_path = env.project_path(".codex/skills/min-binary/assets/tiny.bin");
    assert!(binary_path.exists(), "minimal binary should be deployed");

    let deployed = std::fs::read(&binary_path).unwrap();
    assert_eq!(deployed, vec![0x00], "single NUL byte should be preserved");
}

/// Input boundary: multiple binary files in same skill
#[test]
fn deploy_writes_binary_skill_assets__with_multiple_binary_files() {
    let env = TestEnv::builder().build();

    write_project_skill(
        &env,
        "multi-binary",
        r#"---
description: Skill with multiple binaries
kind: skill
targets: [codex]
---
# Instructions
"#,
        &[("text-file.md", "# Text\n")],
    );

    // Multiple binary files with different content
    env.write_project_binary(
        ".promptpack/skills/multi-binary/assets/image.png",
        &[0x89, 0x50, 0x4E, 0x47, 0x00], // PNG magic + NUL
    );
    env.write_project_binary(
        ".promptpack/skills/multi-binary/assets/data.bin",
        &[0x00, 0x01, 0x02, 0x03],
    );

    let result = env.run(&["deploy", "--yes", "--targets", "codex"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    // Verify all files deployed
    assert!(env
        .project_path(".codex/skills/multi-binary/assets/image.png")
        .exists());
    assert!(env
        .project_path(".codex/skills/multi-binary/assets/data.bin")
        .exists());
    assert!(env
        .project_path(".codex/skills/multi-binary/text-file.md")
        .exists());

    // Verify lockfile tracks both binaries
    let lockfile = env.read_lockfile();
    assert!(
        lockfile.contains("image.png"),
        "lockfile should track image.png"
    );
    assert!(
        lockfile.contains("data.bin"),
        "lockfile should track data.bin"
    );
}

/// State variation: binary files in deeply nested directories
#[test]
fn deploy_writes_binary_skill_assets__with_nested_binary_paths() {
    let env = TestEnv::builder().build();

    write_project_skill(
        &env,
        "nested-binary",
        r#"---
description: Skill with nested binary paths
kind: skill
targets: [codex]
---
# Instructions
"#,
        &[],
    );

    // Binary in deeply nested path
    env.write_project_binary(
        ".promptpack/skills/nested-binary/assets/images/icons/logo.png",
        &[0x89, 0x50, 0x00], // PNG with NUL
    );

    let result = env.run(&["deploy", "--yes", "--targets", "codex"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    // Verify nested path is created
    let nested_path = env.project_path(".codex/skills/nested-binary/assets/images/icons/logo.png");
    assert!(
        nested_path.exists(),
        "nested binary should be deployed at {:?}",
        nested_path
    );
}

/// State variation: skill with only binary files (no text supplementals)
#[test]
fn deploy_writes_binary_skill_assets__with_only_binary_no_text() {
    let env = TestEnv::builder().build();

    write_project_skill(
        &env,
        "binary-only",
        r#"---
description: Skill with only binary assets
kind: skill
targets: [codex]
---
# Instructions
"#,
        &[], // No text supplementals
    );

    // Only binary supplementals
    env.write_project_binary(
        ".promptpack/skills/binary-only/data/model.bin",
        &[0x00, 0xFF, 0x00, 0xFF],
    );

    let result = env.run(&["deploy", "--yes", "--targets", "codex"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    // SKILL.md should exist
    assert_deployed!(&env, ".codex/skills/binary-only/SKILL.md");
    // Binary should exist
    assert!(env
        .project_path(".codex/skills/binary-only/data/model.bin")
        .exists());
}

/// State variation: replacing binary file with new content
#[test]
fn deploy_updates_binary_skill_assets__when_content_changes() {
    let env = TestEnv::builder().build();

    write_project_skill(
        &env,
        "update-binary",
        r#"---
description: Skill to test binary update
kind: skill
targets: [codex]
---
# Instructions
"#,
        &[],
    );

    // Initial binary
    let initial_content = [0x00, 0x01, 0x02];
    env.write_project_binary(
        ".promptpack/skills/update-binary/data.bin",
        &initial_content,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "codex"]);
    assert!(result.success, "initial deploy failed");

    // Update binary with different content
    let updated_content = [0xFF, 0xFE, 0xFD, 0xFC, 0x00];
    env.write_project_binary(
        ".promptpack/skills/update-binary/data.bin",
        &updated_content,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "codex"]);
    assert!(result.success, "update deploy failed");

    // Verify new content
    let deployed = std::fs::read(env.project_path(".codex/skills/update-binary/data.bin")).unwrap();
    assert_eq!(
        deployed.as_slice(),
        &updated_content[..],
        "binary content should be updated"
    );
}

/// Off-by-one boundary: binary file at NUL detection threshold
#[test]
fn deploy_writes_binary_skill_assets__with_nul_at_end_of_text() {
    let env = TestEnv::builder().build();

    write_project_skill(
        &env,
        "nul-threshold",
        r#"---
description: Skill with text+nul file
kind: skill
targets: [codex]
---
# Instructions
"#,
        &[],
    );

    // Valid UTF-8 text followed by a NUL byte (detected as binary)
    let content_with_nul = b"This is valid UTF-8 text.\x00";
    env.write_project_binary(
        ".promptpack/skills/nul-threshold/data.txt",
        content_with_nul,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "codex"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    // Should be treated as binary (due to NUL)
    let lockfile = env.read_lockfile();
    assert!(
        lockfile.contains("is_binary = true"),
        "file with NUL should be tracked as binary:\n{lockfile}"
    );
}
