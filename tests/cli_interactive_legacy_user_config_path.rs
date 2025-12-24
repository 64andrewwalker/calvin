//! Integration test for the legacy user config location `~/.calvin/config.toml`.
//!
//! PRD note: `~/.config/calvin/config.toml` is the XDG-compliant location, but
//! `~/.calvin/config.toml` can be used as an alternative.

use std::fs;
use std::process::Command;

use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

#[test]
fn interactive_can_read_legacy_dot_calvin_config_toml() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    let home = project_dir.join("home");
    fs::create_dir_all(&home).unwrap();

    // Configure a non-default user layer path via legacy config location.
    let user_layer = home.join("dotfiles/.promptpack");
    fs::create_dir_all(&user_layer).unwrap();
    fs::create_dir_all(home.join(".calvin")).unwrap();

    fs::write(
        home.join(".calvin/config.toml"),
        format!(
            r#"
[sources]
user_layer_path = "{}"
"#,
            user_layer.display()
        ),
    )
    .unwrap();

    let output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .args(["--json"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "calvin --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(
        json.get("state").and_then(|v| v.as_str()),
        Some("user_layer_only")
    );
}
