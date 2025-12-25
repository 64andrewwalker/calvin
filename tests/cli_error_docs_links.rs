//! TDD Test: Error messages should include documentation links (PRD §13.1)

use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

#[test]
fn no_layers_error_includes_docs_link() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    // Setup: fake git repo, fake home with no user layer
    fs::create_dir_all(project_dir.join(".git")).unwrap();
    let fake_home = project_dir.join("fake_home");
    fs::create_dir_all(&fake_home).unwrap();

    // NO .promptpack, NO user layer -> should error with "no layers found"
    let output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &fake_home)
        .env("XDG_CONFIG_HOME", fake_home.join(".config"))
        .args(["layers"])
        .output()
        .unwrap();

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should include a docs link
    assert!(
        stderr.contains("https://") || stderr.contains("Docs:") || stderr.contains("docs"),
        "error should include documentation link:\n{}",
        stderr
    );

    // Should include multi-layer guide
    assert!(
        stderr.contains("multi-layer")
            || stderr.contains("guides")
            || stderr.contains("64andrewwalker"),
        "error should link to multi-layer guide:\n{}",
        stderr
    );
}

#[test]
fn registry_corrupted_error_includes_docs_link() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    let fake_home = project_dir.join("fake_home");
    fs::create_dir_all(&fake_home).unwrap();

    // Create corrupted registry
    let registry_path = fake_home.join(".calvin/registry.toml");
    fs::create_dir_all(registry_path.parent().unwrap()).unwrap();
    fs::write(&registry_path, "not valid toml = = =").unwrap();

    let output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &fake_home)
        .args(["projects"])
        .output()
        .unwrap();

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should include helpful information
    assert!(
        stderr.contains("registry file corrupted"),
        "should mention registry corrupted:\n{}",
        stderr
    );

    // Should include recovery instructions
    assert!(
        stderr.contains("calvin deploy") || stderr.contains("rm "),
        "should include recovery instructions:\n{}",
        stderr
    );
}
