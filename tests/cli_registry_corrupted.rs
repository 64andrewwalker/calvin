//! End-to-end test: corrupted registry should surface a helpful error (not silently empty).

use std::fs;
use std::process::Command;

use tempfile::tempdir;

#[test]
fn projects_errors_when_registry_is_corrupted() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    let fake_home = project_dir.join("fake_home");
    fs::create_dir_all(&fake_home).unwrap();

    let registry_path = fake_home.join(".calvin/registry.toml");
    fs::create_dir_all(registry_path.parent().unwrap()).unwrap();
    fs::write(&registry_path, "this is not toml = = =").unwrap();

    let bin = env!("CARGO_BIN_EXE_calvin");
    let output = Command::new(bin)
        .current_dir(project_dir)
        .env("HOME", &fake_home)
        .args(["projects"])
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
        stderr.contains("registry file corrupted"),
        "stderr:\n{}",
        stderr
    );
    assert!(stderr.contains("calvin deploy"), "stderr:\n{}", stderr);
}
