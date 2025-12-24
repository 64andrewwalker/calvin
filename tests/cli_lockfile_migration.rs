//! Integration tests for lockfile migration (.promptpack/.calvin.lock -> ./calvin.lock)

use std::process::Command;

use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

#[test]
fn deploy_migrates_old_lockfile() {
    let dir = tempdir().unwrap();
    let fake_home = dir.path().join("fake_home");
    std::fs::create_dir_all(&fake_home).unwrap();
    let promptpack = dir.path().join(".promptpack");
    std::fs::create_dir_all(&promptpack).unwrap();

    // Minimal asset + config so deploy succeeds.
    std::fs::write(
        promptpack.join("test.md"),
        r#"---
kind: policy
description: Test
scope: project
targets: [cursor]
---
Content
"#,
    )
    .unwrap();
    std::fs::write(
        promptpack.join("config.toml"),
        r#"
[deploy]
target = "project"

[targets]
enabled = ["cursor"]
"#,
    )
    .unwrap();

    // Create legacy lockfile location
    let old_lockfile = promptpack.join(".calvin.lock");
    std::fs::write(&old_lockfile, "version = 1\n").unwrap();

    let output = Command::new(bin())
        .current_dir(dir.path())
        .env("HOME", &fake_home)
        .env("XDG_CONFIG_HOME", fake_home.join(".config"))
        .args(["deploy", "--yes"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "deploy should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let new_lockfile = dir.path().join("calvin.lock");
    assert!(new_lockfile.exists(), "New lockfile should exist");
    assert!(!old_lockfile.exists(), "Old lockfile should be removed");
}
