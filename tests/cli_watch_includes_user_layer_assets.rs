//! Regression test: `calvin watch` should compile the resolved multi-layer stack.
//!
//! PRD §11.4: watch defaults to *watching* only the project layer, but it still uses
//! the multi-layer resolution/merge semantics for compilation (user/custom/project).

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::tempdir;

fn calvin_cmd(fake_home: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_calvin"));
    cmd.env("HOME", fake_home);
    cmd.env("XDG_CONFIG_HOME", fake_home.join(".config"));
    cmd
}

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

#[test]
fn watch_initial_sync_includes_user_layer_assets() {
    let temp = tempdir().unwrap();
    let project_dir = temp.path().join("project");
    let home = temp.path().join("home");
    fs::create_dir_all(&project_dir).unwrap();
    fs::create_dir_all(&home).unwrap();

    // Project layer exists (watch needs deploy.target configured).
    let project_promptpack = project_dir.join(".promptpack");
    write_file(
        &project_promptpack.join("config.toml"),
        r#"[deploy]
target = "project"
"#,
    );

    // User layer asset (should be included in the compiled outputs).
    let user_promptpack = home.join(".calvin/.promptpack");
    write_file(
        &user_promptpack.join("user-action.md"),
        r#"---
kind: action
description: User action
scope: project
---
# User Action

Hello from user layer
"#,
    );

    let mut child = calvin_cmd(&home)
        .arg("watch")
        .arg("--json")
        .current_dir(&project_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start calvin watch");

    let deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < deadline {
        if project_dir.join(".claude/commands/user-action.md").exists() {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    let _ = child.kill();
    let output = child.wait_with_output().expect("Failed to get output");

    assert!(
        project_dir.join(".claude/commands/user-action.md").exists(),
        "expected user layer outputs under project root; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn watch_all_layers_redeploys_on_user_layer_change() {
    let temp = tempdir().unwrap();
    let project_dir = temp.path().join("project");
    let home = temp.path().join("home");
    fs::create_dir_all(&project_dir).unwrap();
    fs::create_dir_all(&home).unwrap();

    // Project layer exists (watch needs deploy.target configured).
    let project_promptpack = project_dir.join(".promptpack");
    write_file(
        &project_promptpack.join("config.toml"),
        r#"[deploy]
target = "project"
"#,
    );

    let user_promptpack = home.join(".calvin/.promptpack");
    let user_asset = user_promptpack.join("user-action.md");
    write_file(
        &user_asset,
        r#"---
kind: action
description: User action
scope: project
---
# User Action

v1
"#,
    );

    let mut child = calvin_cmd(&home)
        .arg("watch")
        .arg("--watch-all-layers")
        .arg("--json")
        .current_dir(&project_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start calvin watch");

    let out_path = project_dir.join(".claude/commands/user-action.md");

    let mut initial_ok = false;
    let deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < deadline {
        if out_path.exists()
            && fs::read_to_string(&out_path)
                .ok()
                .is_some_and(|c| c.contains("v1"))
        {
            initial_ok = true;
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    if !initial_ok {
        let _ = child.kill();
        let output = child.wait_with_output().expect("Failed to get output");
        panic!(
            "expected initial sync output (v1); stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Update user layer asset; with --watch-all-layers this should trigger a re-sync.
    write_file(
        &user_asset,
        r#"---
kind: action
description: User action
scope: project
---
# User Action

v2
"#,
    );

    // Give file watcher time to pick up the change
    thread::sleep(Duration::from_millis(100));

    // Use longer timeout for CI environments under load
    let deadline = Instant::now() + Duration::from_secs(6);
    while Instant::now() < deadline {
        if out_path.exists()
            && fs::read_to_string(&out_path)
                .ok()
                .is_some_and(|c| c.contains("v2"))
        {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    let _ = child.kill();
    let output = child.wait_with_output().expect("Failed to get output");

    assert!(
        out_path.exists()
            && fs::read_to_string(&out_path)
                .ok()
                .is_some_and(|c| c.contains("v2")),
        "expected watch to redeploy after user-layer change; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
