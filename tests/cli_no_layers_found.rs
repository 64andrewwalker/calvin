//! End-to-end test: missing layers surfaces a helpful error.

mod common;

use std::fs;
use std::process::Command;

use common::WindowsCompatExt;
use tempfile::tempdir;

#[test]
fn layers_errors_when_no_layers_exist() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    // Avoid git prompt behavior in other commands; harmless here.
    fs::create_dir_all(project_dir.join(".git")).unwrap();

    let fake_home = project_dir.join("fake_home");
    fs::create_dir_all(&fake_home).unwrap();

    let bin = env!("CARGO_BIN_EXE_calvin");
    let output = Command::new(bin)
        .current_dir(project_dir)
        .with_test_home(&fake_home)
        .args(["layers"])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "expected failure, stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no promptpack layers found"),
        "stderr:\n{}",
        stderr
    );
    assert!(stderr.contains("calvin init --user"), "stderr:\n{}", stderr);
}
