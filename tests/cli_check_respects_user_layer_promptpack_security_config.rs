//! Integration test for multi-layer promptpack config merge in `calvin check`.
//!
//! PRD: `~/.calvin/.promptpack/config.toml` participates in config merge across layers.
//! Security settings from the user layer should be visible to `calvin check` (doctor).

use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

#[test]
fn check_respects_security_allow_naked_from_user_layer_promptpack_config() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    // Create a minimal Claude settings file so the allow_naked warning is evaluated.
    fs::create_dir_all(project_dir.join(".claude")).unwrap();
    fs::write(project_dir.join(".claude/settings.json"), "{}\n").unwrap();

    // Fake HOME with user layer promptpack config enabling allow_naked.
    let home = project_dir.join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(home.join(".config")).unwrap();
    fs::create_dir_all(home.join(".calvin/.promptpack")).unwrap();
    fs::write(
        home.join(".calvin/.promptpack/config.toml"),
        r#"
[security]
allow_naked = true
"#,
    )
    .unwrap();

    let output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .args(["--json", "check"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "check failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut found = false;
    for line in stdout.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if v.get("event").and_then(|e| e.as_str()) != Some("check") {
            continue;
        }
        if v.get("name").and_then(|n| n.as_str()) == Some("allow_naked")
            && v.get("status").and_then(|s| s.as_str()) == Some("warning")
        {
            found = true;
            break;
        }
    }

    assert!(
        found,
        "expected allow_naked warning check event, stdout:\n{}",
        stdout
    );
}
