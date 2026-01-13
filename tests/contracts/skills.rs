//! Contract tests for skill deployment.
//!
//! These contracts ensure skill deployment behavior is consistent
//! across environments and invocation methods.

use crate::common::skills::write_project_skill;
use crate::common::*;

/// CONTRACT: Binary files in skill directories are deployed intact
///
/// Binary files (images, PDFs, etc.) in skill directories must be:
/// 1. Detected as binary (not treated as UTF-8 text)
/// 2. Deployed to the target with identical content
/// 3. Tracked in the lockfile with `is_binary = true`
///
/// This ensures skills with documentation images, reference PDFs,
/// or other binary assets work correctly.
#[test]
fn contract_skill_binary_assets_deployed_intact() {
    let env = TestEnv::builder().build();

    // Create skill with text content
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
        &[],
    );

    // Create binary content (PNG-like with NUL bytes for detection)
    let binary_content: [u8; 16] = [
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR length (contains NUL)
        0x49, 0x48, 0x44, 0x52, // "IHDR"
    ];
    env.write_project_binary(
        ".promptpack/skills/with-binary/assets/diagram.png",
        &binary_content,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "codex"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    // CONTRACT: Binary content is deployed byte-for-byte
    let deployed_path = env.project_path(".codex/skills/with-binary/assets/diagram.png");
    let deployed_content = std::fs::read(&deployed_path).expect("binary file should be deployed");
    assert_eq!(
        deployed_content.as_slice(),
        &binary_content[..],
        "CONTRACT: Deployed binary content must match source exactly"
    );

    // CONTRACT: Binary file is tracked in lockfile with is_binary flag
    let lockfile = env.read_lockfile();
    assert!(
        lockfile.contains("is_binary = true"),
        "CONTRACT: Binary files must be tracked with is_binary flag in lockfile:\n{lockfile}"
    );
}

/// CONTRACT: Orphaned binary skill assets are cleaned up
///
/// When a binary file is removed from a skill's source directory,
/// `deploy --cleanup` must remove the orphaned binary from the target.
#[test]
fn contract_skill_binary_orphans_cleaned_up() {
    let env = TestEnv::builder().build();

    // Create skill with binary asset
    write_project_skill(
        &env,
        "cleanup-test",
        r#"---
description: Cleanup test skill
kind: skill
targets: [codex]
---
# Instructions
"#,
        &[],
    );

    let binary_content: [u8; 8] = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
    env.write_project_binary(
        ".promptpack/skills/cleanup-test/assets/photo.jpg",
        &binary_content,
    );

    // Initial deploy
    env.run(&["deploy", "--yes", "--targets", "codex"]);

    // Verify binary is deployed
    let binary_path = env.project_path(".codex/skills/cleanup-test/assets/photo.jpg");
    assert!(binary_path.exists(), "binary should be deployed initially");

    // Remove binary from source
    std::fs::remove_file(env.project_path(".promptpack/skills/cleanup-test/assets/photo.jpg"))
        .unwrap();

    // Deploy with cleanup
    let result = env.run(&["deploy", "--yes", "--targets", "codex", "--cleanup"]);
    assert!(
        result.success,
        "deploy --cleanup failed:\n{}",
        result.combined_output()
    );

    // CONTRACT: Orphaned binary is removed
    assert!(
        !binary_path.exists(),
        "CONTRACT: Orphaned binary files must be cleaned up"
    );
}

/// CONTRACT: Unchanged binary skill assets are not counted as written
///
/// When a skill contains binary supplemental files (images, PDFs, etc.) and a deploy is
/// re-run with no changes, Calvin must treat those binary outputs as up-to-date:
/// - they should be reported as skipped (not written)
/// - the deploy summary should report `0 files written`
///
/// This prevents confusing output like "N files written" when nothing changed.
#[test]
fn contract_skill_binary_assets_not_counted_as_written_when_unchanged() {
    let env = TestEnv::builder().build();

    write_project_skill(
        &env,
        "binary-idempotent",
        r#"---
description: Skill with binary asset
kind: skill
targets: [codex]
---
# Instructions
"#,
        &[],
    );

    let binary_content: [u8; 16] = [
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR length (contains NUL)
        0x49, 0x48, 0x44, 0x52, // "IHDR"
    ];
    env.write_project_binary(
        ".promptpack/skills/binary-idempotent/assets/diagram.png",
        &binary_content,
    );

    // Initial deploy writes the binary file.
    let first = env.run(&["deploy", "--yes", "--targets", "codex"]);
    assert!(
        first.success,
        "initial deploy failed:\n{}",
        first.combined_output()
    );

    // Re-deploy with no changes should not count binary outputs as written.
    let second = env.run(&["deploy", "--yes", "--targets", "codex"]);
    assert!(
        second.success,
        "second deploy failed:\n{}",
        second.combined_output()
    );

    let output = second.combined_output();
    assert!(
        output.contains("0 files written"),
        "CONTRACT: unchanged binary assets must not be counted as written.\nOutput:\n{}",
        output
    );
}
