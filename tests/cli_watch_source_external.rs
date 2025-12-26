//! Integration tests for `calvin watch --source` semantics (multi-layer Phase 3).
//!
//! PRD: `--source PATH` replaces the project layer path, but should not change the project root.
//! Watch deploys outputs + lockfile anchored to the current working directory (project root),
//! not `source.parent()`.

mod common;

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use common::WindowsCompatExt;
use tempfile::tempdir;

fn calvin_cmd(fake_home: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_calvin"));
    cmd.with_test_home(fake_home);
    cmd
}

fn write_asset(promptpack: &Path, filename: &str, body: &str) {
    fs::create_dir_all(promptpack).unwrap();
    fs::write(promptpack.join(filename), body).unwrap();
}

#[test]
fn watch_with_external_source_writes_outputs_to_project_root() {
    let temp = tempdir().unwrap();
    let project_dir = temp.path().join("project");
    let fake_home = temp.path().join("fake_home");
    fs::create_dir_all(&project_dir).unwrap();
    fs::create_dir_all(&fake_home).unwrap();

    // External promptpack directory.
    let external_root = temp.path().join("shared");
    let external_promptpack = external_root.join(".promptpack");
    write_asset(
        &external_promptpack,
        "config.toml",
        r#"[deploy]
target = "project"
"#,
    );
    write_asset(
        &external_promptpack,
        "watch-test.md",
        r#"---
kind: action
description: External watch test
scope: project
---
# External Watch Test

Hello
"#,
    );

    // Start watch and give it time to do an initial sync.
    let mut child = calvin_cmd(&fake_home)
        .arg("watch")
        .arg("--json")
        .arg("--source")
        .arg(external_promptpack.to_string_lossy().as_ref())
        .current_dir(&project_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start calvin watch");

    let deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < deadline {
        if project_dir.join(".claude/commands/watch-test.md").exists()
            && project_dir.join("calvin.lock").exists()
        {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    let _ = child.kill();
    let output = child.wait_with_output().expect("Failed to get output");

    assert!(
        project_dir.join(".claude/commands/watch-test.md").exists(),
        "expected watch to write outputs under project root; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        project_dir.join("calvin.lock").exists(),
        "expected watch to write calvin.lock under project root; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Should not write to the external promptpack's parent directory.
    assert!(
        !external_root
            .join(".claude/commands/watch-test.md")
            .exists(),
        "did not expect outputs under external promptpack parent"
    );
    assert!(
        !external_root.join("calvin.lock").exists(),
        "did not expect calvin.lock next to external promptpack"
    );
}
