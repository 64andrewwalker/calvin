//! Integration tests for `calvin init --user` (Phase 3: Configuration & CLI)

use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

#[test]
fn init_user_creates_user_layer_promptpack() {
    let dir = tempdir().unwrap();
    let home = dir.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let output = Command::new(bin())
        .current_dir(dir.path())
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .args(["init", "--user", "--json"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "init --user failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let user_layer = home.join(".calvin/.promptpack");
    assert!(
        user_layer.join("config.toml").exists(),
        "expected user config.toml to exist"
    );
    assert!(user_layer.join("policies").exists());
    assert!(user_layer.join("actions").exists());
    assert!(user_layer.join("agents").exists());
}

#[test]
fn init_user_fails_if_user_layer_exists_without_force() {
    let dir = tempdir().unwrap();
    let home = dir.path().join("home");
    fs::create_dir_all(&home).unwrap();

    fs::create_dir_all(home.join(".calvin/.promptpack")).unwrap();

    let output = Command::new(bin())
        .current_dir(dir.path())
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .args(["init", "--user", "--json"])
        .output()
        .unwrap();

    assert!(!output.status.success(), "expected init --user to fail");
}
