//! Integration test to ensure `calvin init` writes a valid, up-to-date config.toml schema.

use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

#[test]
fn init_writes_targets_enabled_schema_not_legacy_include_exclude() {
    let dir = tempdir().unwrap();

    let output = Command::new(bin())
        .current_dir(dir.path())
        .args(["init", "--json"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let config_path = dir.path().join(".promptpack/config.toml");
    let content = fs::read_to_string(config_path).unwrap();

    assert!(content.contains("[targets]"), "config missing [targets]");
    assert!(
        content.contains("enabled"),
        "config should reference targets.enabled"
    );
    assert!(
        !content.contains("include ="),
        "legacy targets.include should not appear"
    );
    assert!(
        !content.contains("exclude ="),
        "legacy targets.exclude should not appear"
    );
    assert!(
        !content.contains("default_scope"),
        "unknown deploy.default_scope should not appear"
    );
}
