//! Integration test for interactive state detection with configured user_layer_path.
//!
//! PRD: user layer path can be configured via `~/.config/calvin/config.toml` and
//! interactive detection should respect it (not hard-code `~/.calvin/.promptpack`).

mod common;

use std::fs;
use std::process::Command;

use common::WindowsCompatExt;
use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

#[test]
fn interactive_respects_sources_user_layer_path() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    let home = project_dir.join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(home.join(".config/calvin")).unwrap();

    // Configure a non-default user layer path.
    let user_layer = home.join("dotfiles/.promptpack");
    fs::create_dir_all(&user_layer).unwrap();

    // Use forward slashes in TOML to avoid Windows backslash escaping issues
    let user_layer_toml_path = user_layer.display().to_string().replace('\\', "/");
    fs::write(
        home.join(".config/calvin/config.toml"),
        format!(
            r#"
[sources]
user_layer_path = "{}"
"#,
            user_layer_toml_path
        ),
    )
    .unwrap();

    let output = Command::new(bin())
        .current_dir(project_dir)
        .with_test_home(&home)
        // Ensure env var doesn't override config file we just wrote
        .env_remove("CALVIN_SOURCES_USER_LAYER_PATH")
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
        Some("global_layers_only")
    );
    assert!(json["state_aliases"]
        .as_array()
        .is_some_and(|a| a.iter().any(|v| v.as_str() == Some("user_layer_only"))));
}
