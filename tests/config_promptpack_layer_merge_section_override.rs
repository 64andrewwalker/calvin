//! Unit-style integration test for PRD §11.2 section-level override semantics.

use std::fs;

use tempfile::tempdir;

#[test]
fn promptpack_layer_config_section_override_is_not_deep_merge() {
    use calvin::config::SecurityMode;
    use calvin::config::{merge_promptpack_layer_configs, Config, PromptpackLayerInputs};
    use calvin::Target;

    let dir = tempdir().unwrap();
    let project_root = dir.path();

    let user_layer = project_root.join("user_layer");
    fs::create_dir_all(&user_layer).unwrap();
    fs::write(
        user_layer.join("config.toml"),
        r#"
[targets]
enabled = ["cursor"]

[security]
mode = "strict"
allow_naked = false
"#,
    )
    .unwrap();

    let project_layer = project_root.join(".promptpack");
    fs::create_dir_all(&project_layer).unwrap();
    fs::write(
        project_layer.join("config.toml"),
        r#"
[targets]
enabled = ["vscode"]

[security]
allow_naked = true
"#,
    )
    .unwrap();

    let base = Config::default();
    let (merged, _warnings) = merge_promptpack_layer_configs(
        &base,
        PromptpackLayerInputs {
            project_root: project_root.to_path_buf(),
            project_layer_path: project_layer,
            disable_project_layer: false,
            user_layer_path: Some(user_layer),
            use_user_layer: true,
            additional_layers: Vec::new(),
            use_additional_layers: true,
            remote_mode: false,
        },
    )
    .unwrap();

    // targets from project layer override user layer
    assert_eq!(merged.enabled_targets(), vec![Target::VSCode]);

    // security section from project layer overrides user layer entirely:
    // mode falls back to default, it is NOT inherited from the user layer.
    assert_eq!(merged.security.mode, SecurityMode::Balanced);
    assert!(merged.security.allow_naked);
}
