//! Integration tests for `calvin diff --source` semantics (multi-layer Phase 3).
//!
//! PRD: `--source PATH` replaces the project layer path, but should not change the project root.
//! Diff must be anchored to the current working directory (project root), not `source.parent()`.

mod common;

use common::env::TestEnv;
use std::fs;

fn write_asset(promptpack: &std::path::Path, filename: &str, body: &str) {
    fs::create_dir_all(promptpack).unwrap();
    fs::write(promptpack.join(filename), body).unwrap();
}

#[test]
fn diff_with_external_source_anchors_to_project_root() {
    let env = TestEnv::builder()
        .without_project_promptpack()
        .without_user_layer()
        .build();

    // "External" promptpack (not at ./ .promptpack).
    let external_root = env.project_path("shared");
    let external_promptpack = external_root.join(".promptpack");
    write_asset(
        &external_promptpack,
        "solo.md",
        r#"---
kind: policy
description: Solo
scope: project
---
SOLO
"#,
    );

    // First deploy (to establish outputs + lockfile at the *project root*).
    let deploy = env.run_with_env(
        &[
            "deploy",
            "--yes",
            "--no-user-layer",
            "--no-additional-layers",
            "--source",
            external_promptpack.to_string_lossy().as_ref(),
        ],
        &[],
    );

    assert!(deploy.success, "deploy failed: {}", deploy.stderr);
    assert!(
        env.project_path("calvin.lock").exists(),
        "expected lockfile at project root"
    );

    // Now diff the same external source. If diff incorrectly anchors to `source.parent()`,
    // it will look for outputs + lockfile next to the external promptpack and report "new".
    let diff = env.run_with_env(
        &[
            "diff",
            "--json",
            "--source",
            external_promptpack.to_string_lossy().as_ref(),
        ],
        &[],
    );

    assert!(diff.success, "diff failed: {}", diff.stderr);

    let stdout = diff.stdout;
    assert!(
        !stdout.contains("\"status\":\"new\""),
        "expected no new files when diffing immediately after deploy; got:\n{stdout}"
    );
    assert!(
        stdout.contains("\"status\":\"unchanged\""),
        "expected at least one unchanged file; got:\n{stdout}"
    );
}
