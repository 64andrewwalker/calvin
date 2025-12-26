use std::fs;
use std::process::Command;

use serde_json::Value;
use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_calvin")
}

fn write_asset(promptpack: &std::path::Path, filename: &str) {
    fs::create_dir_all(promptpack).unwrap();
    fs::write(
        promptpack.join(filename),
        r#"---
kind: policy
description: Policy
scope: project
targets: [cursor]
---
POLICY
"#,
    )
    .unwrap();
}

fn deploy_project(project_dir: &std::path::Path, home: &std::path::Path) {
    fs::create_dir_all(project_dir.join(".git")).unwrap();
    let promptpack = project_dir.join(".promptpack");
    write_asset(&promptpack, "policy.md");

    // Create registry directory
    let calvin_dir = home.join(".calvin");
    fs::create_dir_all(&calvin_dir).unwrap();
    let registry_path = calvin_dir.join("registry.toml");
    let user_layer_path = calvin_dir.join(".promptpack");

    let output = Command::new(bin())
        .current_dir(project_dir)
        .env("HOME", home)
        .env("USERPROFILE", home) // Windows: USERPROFILE fallback for some crates
        .env("XDG_CONFIG_HOME", home.join(".config"))
        // Windows-specific: dirs::home_dir uses Windows API, not env vars
        .env("CALVIN_REGISTRY_PATH", &registry_path)
        .env("CALVIN_SOURCES_USER_LAYER_PATH", &user_layer_path)
        .args(["deploy", "--yes", "--targets", "cursor"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "deploy failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn check_all_emits_project_field_in_json_events() {
    let dir = tempdir().unwrap();
    let home = dir.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let project_a = dir.path().join("project-a");
    let project_b = dir.path().join("project-b");
    fs::create_dir_all(&project_a).unwrap();
    fs::create_dir_all(&project_b).unwrap();

    deploy_project(&project_a, &home);
    deploy_project(&project_b, &home);

    // Setup registry and user layer paths for Windows compatibility
    let calvin_dir = home.join(".calvin");
    let registry_path = calvin_dir.join("registry.toml");
    let user_layer_path = calvin_dir.join(".promptpack");

    let output = Command::new(bin())
        .current_dir(dir.path())
        .env("HOME", &home)
        .env("USERPROFILE", &home) // Windows: USERPROFILE fallback for some crates
        .env("XDG_CONFIG_HOME", home.join(".config"))
        // Windows-specific: dirs::home_dir uses Windows API, not env vars
        .env("CALVIN_REGISTRY_PATH", &registry_path)
        .env("CALVIN_SOURCES_USER_LAYER_PATH", &user_layer_path)
        .args(["check", "--all", "--json"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "check --all failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut projects = std::collections::HashSet::new();
    for line in stdout.lines().filter(|l| !l.trim().is_empty()) {
        let v: Value = serde_json::from_str(line).unwrap();
        if let Some(p) = v.get("project").and_then(|p| p.as_str()) {
            projects.insert(p.to_string());
        }
    }

    assert!(
        projects.iter().any(|p| p.ends_with("project-a")),
        "expected project-a in project field, got {projects:?}\n{stdout}"
    );
    assert!(
        projects.iter().any(|p| p.ends_with("project-b")),
        "expected project-b in project field, got {projects:?}\n{stdout}"
    );
}
