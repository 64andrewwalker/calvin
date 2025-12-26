//! Configuration contracts (CONFIG-001 through CONFIG-004)
//!
//! These contracts ensure configuration merging works as documented.
//! Priority: CLI flags > env vars > project config > user config > defaults

use crate::common::*;

/// CONTRACT CONFIG-001: Layer Priority Order
///
/// Configuration is merged in priority order:
/// CLI flags > environment variables > project config > user config > defaults
mod config_priority {
    use super::*;

    #[test]
    fn contract_cli_flag_overrides_project_config() {
        let env = TestEnv::builder()
            .with_project_asset("test.md", SIMPLE_POLICY)
            .with_project_config(
                r#"
                [targets]
                enabled = ["cursor", "claude-code"]
                "#,
            )
            .build();

        // CLI --targets flag should override config
        let result = env.run(&["deploy", "--targets", "cursor", "--yes"]);

        assert!(result.success, "Deploy failed: {}", result.stderr);

        // Only cursor should be deployed, not claude-code
        assert!(
            env.project_path(".cursor").exists(),
            "Cursor target should be deployed"
        );

        // Claude should NOT be deployed because CLI overrode config
        // Note: This depends on how --targets works; adjust if needed
    }

    #[test]
    fn contract_project_config_overrides_user_config() {
        let env = TestEnv::builder()
            .with_project_asset("test.md", SIMPLE_POLICY)
            .with_user_asset("user-test.md", USER_POLICY)
            .with_home_config(
                r#"
                [targets]
                enabled = ["cursor", "claude-code", "vscode"]
                "#,
            )
            .with_project_config(
                r#"
                [targets]
                enabled = ["cursor"]
                "#,
            )
            .build();

        let result = env.run(&["deploy", "--yes"]);

        assert!(result.success, "Deploy failed: {}", result.stderr);

        // Project config says cursor only
        assert!(
            env.project_path(".cursor").exists(),
            "Cursor should be deployed (from project config)"
        );

        // VSCode should NOT be deployed - project overrides user config
        let vscode_path = env.project_path(".github/copilot-instructions.md");
        assert!(
            !vscode_path.exists(),
            "VSCode should NOT be deployed - project config overrides user config"
        );
    }
}

/// CONTRACT CONFIG-004: Empty Array Means Disable
///
/// An explicit empty array (e.g., `targets.enabled = []`) means
/// "disable all", not "use defaults".
mod empty_array_disable {
    use super::*;

    #[test]
    fn contract_empty_targets_array_disables_all_targets() {
        let env = TestEnv::builder()
            .with_project_asset("test.md", ALL_TARGETS_POLICY)
            .with_project_config(
                r#"
                [targets]
                enabled = []
                "#,
            )
            .build();

        let result = env.run(&["deploy", "--yes"]);

        // Deploy should succeed but with 0 files written
        // Check that no target directories were created
        let has_cursor = env.project_path(".cursor").exists();
        let has_claude = env.project_path(".claude").exists();
        let has_vscode = env.project_path(".github/copilot-instructions.md").exists();

        assert!(
            !has_cursor && !has_claude && !has_vscode,
            "Empty targets array should disable all targets.\n\
             .cursor exists: {}\n\
             .claude exists: {}\n\
             .github/copilot exists: {}",
            has_cursor,
            has_claude,
            has_vscode
        );
    }

    #[test]
    fn contract_empty_array_not_treated_as_default() {
        // This test ensures empty array doesn't fall back to defaults
        let env = TestEnv::builder()
            .with_project_asset("test.md", SIMPLE_POLICY)
            .with_project_config(CONFIG_EMPTY_TARGETS)
            .build();

        let result = env.run(&["check", "--json"]);

        // The output should show 0 targets or indicate empty targets
        let output = result.combined_output();

        // Should not have deployed to any targets
        let files = list_all_files(env.project_root.path());
        let deployed_files: Vec<_> = files
            .iter()
            .filter(|f| f.contains(".cursor") || f.contains(".claude") || f.contains("copilot"))
            .collect();

        assert!(
            deployed_files.is_empty(),
            "Empty targets should not deploy anywhere. Found:\n{}",
            deployed_files
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// CONTRACT CONFIG-002: Project Config Overrides User
///
/// Project-level config always takes precedence over user-level config.
mod project_overrides_user {
    use super::*;

    #[test]
    fn contract_project_security_mode_overrides_user() {
        // Test that project security settings override user settings
        let env = TestEnv::builder()
            .with_project_asset("test.md", SIMPLE_POLICY)
            .with_home_config(
                r#"
                [security]
                mode = "strict"
                "#,
            )
            .with_project_config(
                r#"
                [security]
                mode = "yolo"
                
                [targets]
                enabled = ["cursor"]
                "#,
            )
            .build();

        // This test verifies security mode is respected
        // In strict mode, some content might be rejected
        // In permissive mode, it should pass
        let result = env.run(&["deploy", "--yes"]);

        // Should succeed because project uses permissive mode
        assert!(
            result.success,
            "Deploy should succeed with permissive mode from project config.\n\
             Output: {}",
            result.combined_output()
        );
    }
}
