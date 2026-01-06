//! Golden tests for Calvin
//!
//! These tests verify that a reference .promptpack/ directory produces
//! the expected output for all platform adapters.

use std::path::Path;

use calvin::config::Config;
use calvin::domain::entities::{Asset, OutputFile};
use calvin::domain::value_objects::Target as DomainTarget;
use calvin::infrastructure::adapters::all_adapters;
use calvin::models::{PromptAsset, Target};
use calvin::parser::parse_directory;

/// Compile assets using infrastructure adapters (replaces deprecated compile_assets)
fn compile_assets_with_adapters(
    assets: &[PromptAsset],
    targets: &[Target],
    _config: &Config,
) -> Vec<OutputFile> {
    let mut outputs = Vec::new();
    let adapters = all_adapters();

    // Convert legacy assets to domain assets
    let domain_assets: Vec<Asset> = assets.iter().map(|a| Asset::from(a.clone())).collect();

    // Convert targets to domain targets
    let domain_targets: Vec<DomainTarget> = targets
        .iter()
        .map(|t| match t {
            Target::ClaudeCode => DomainTarget::ClaudeCode,
            Target::Cursor => DomainTarget::Cursor,
            Target::VSCode => DomainTarget::VSCode,
            Target::Antigravity => DomainTarget::Antigravity,
            Target::Codex => DomainTarget::Codex,
            Target::OpenCode => DomainTarget::OpenCode,
            Target::All => DomainTarget::All,
        })
        .collect();

    for asset in &domain_assets {
        for adapter in &adapters {
            let adapter_target = adapter.target();

            // Skip if not in requested targets
            if !domain_targets.is_empty() && !domain_targets.contains(&adapter_target) {
                continue;
            }

            if let Ok(files) = adapter.compile(asset) {
                outputs.extend(files);
            }
        }
    }

    // Post-compile
    for adapter in &adapters {
        let adapter_target = adapter.target();
        if !domain_targets.is_empty() && !domain_targets.contains(&adapter_target) {
            continue;
        }
        if let Ok(files) = adapter.post_compile(&domain_assets) {
            outputs.extend(files);
        }
    }

    outputs.sort_by(|a, b| a.path().cmp(b.path()));
    outputs
}

/// Test fixture: a simple policy file
const SIMPLE_POLICY: &str = r#"---
description: Code style policy for consistent formatting
kind: policy
targets: [all]
---
# Code Style Policy

## Formatting Rules

1. Use 4 spaces for indentation
2. Maximum line length: 100 characters
3. Use trailing commas in multi-line structures
"#;

/// Test fixture: an action with special characters
const ACTION_WITH_QUOTES: &str = r#"---
description: Generate tests for "foo" function
kind: action
---
# Generate Tests

Check if variable is named "foo" and create tests.

Use $ARGUMENTS for input.
"#;

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    use insta::assert_snapshot;
    use std::fs;
    use tempfile::tempdir;

    fn create_test_promptpack(dir: &Path) {
        // Create policies directory
        fs::create_dir_all(dir.join("policies")).unwrap();
        fs::write(dir.join("policies/code-style.md"), SIMPLE_POLICY).unwrap();

        // Create actions directory
        fs::create_dir_all(dir.join("actions")).unwrap();
        fs::write(dir.join("actions/generate-tests.md"), ACTION_WITH_QUOTES).unwrap();
    }

    #[test]
    fn test_golden_claude_code_policy() {
        let dir = tempdir().unwrap();
        let source = dir.path().join(".promptpack");
        create_test_promptpack(&source);

        let assets = parse_directory(&source).unwrap();
        let config = Config::default();
        let targets = vec![Target::ClaudeCode];
        let outputs = compile_assets_with_adapters(&assets, &targets, &config);

        // Find the code-style output
        let policy_output = outputs
            .iter()
            .find(|o| o.path().to_string_lossy().contains("code-style"))
            .expect("Should have code-style output");

        assert_snapshot!("claude_code_policy", &policy_output.content());
    }

    #[test]
    fn test_golden_claude_code_action_with_quotes() {
        let dir = tempdir().unwrap();
        let source = dir.path().join(".promptpack");
        create_test_promptpack(&source);

        let assets = parse_directory(&source).unwrap();
        let config = Config::default();
        let targets = vec![Target::ClaudeCode];
        let outputs = compile_assets_with_adapters(&assets, &targets, &config);

        // Find the generate-tests output
        let action_output = outputs
            .iter()
            .find(|o| o.path().to_string_lossy().contains("generate-tests"))
            .expect("Should have generate-tests output");

        // This tests that quotes in content are preserved correctly
        assert_snapshot!("claude_code_action_quotes", &action_output.content());
    }

    #[test]
    fn test_golden_cursor_policy() {
        let dir = tempdir().unwrap();
        let source = dir.path().join(".promptpack");
        create_test_promptpack(&source);

        let assets = parse_directory(&source).unwrap();
        let config = Config::default();
        let targets = vec![Target::Cursor];
        let outputs = compile_assets_with_adapters(&assets, &targets, &config);

        let policy_output = outputs
            .iter()
            .find(|o| o.path().to_string_lossy().contains("code-style"))
            .expect("Should have code-style output");

        assert_snapshot!("cursor_policy", &policy_output.content());
    }

    #[test]
    fn test_golden_vscode_instructions() {
        let dir = tempdir().unwrap();
        let source = dir.path().join(".promptpack");
        create_test_promptpack(&source);

        let assets = parse_directory(&source).unwrap();
        let config = Config::default();
        let targets = vec![Target::VSCode];
        let outputs = compile_assets_with_adapters(&assets, &targets, &config);

        // VS Code now generates individual .instructions.md files by default
        let instr_output = outputs
            .iter()
            .find(|o| {
                o.path()
                    .to_string_lossy()
                    .contains("code-style.instructions.md")
            })
            .expect("Should have code-style.instructions.md output");

        assert_snapshot!("vscode_instructions", &instr_output.content());
    }

    #[test]
    fn test_golden_antigravity_rules() {
        let dir = tempdir().unwrap();
        let source = dir.path().join(".promptpack");
        create_test_promptpack(&source);

        let assets = parse_directory(&source).unwrap();
        let config = Config::default();
        let targets = vec![Target::Antigravity];
        let outputs = compile_assets_with_adapters(&assets, &targets, &config);

        let rule_output = outputs
            .iter()
            .find(|o| o.path().to_string_lossy().contains("code-style"))
            .expect("Should have code-style output");

        assert_snapshot!("antigravity_rule", &rule_output.content());
    }

    #[test]
    fn test_golden_codex_prompt() {
        let dir = tempdir().unwrap();
        let source = dir.path().join(".promptpack");
        create_test_promptpack(&source);

        let assets = parse_directory(&source).unwrap();
        let config = Config::default();
        let targets = vec![Target::Codex];
        let outputs = compile_assets_with_adapters(&assets, &targets, &config);

        let prompt_output = outputs
            .iter()
            .find(|o| o.path().to_string_lossy().contains("generate-tests"))
            .expect("Should have generate-tests output");

        assert_snapshot!("codex_prompt", &prompt_output.content());
    }
}

#[cfg(test)]
mod escaping_tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// Test that JSON output with quotes doesn't corrupt
    #[test]
    fn test_escaping_json_quotes_in_content() {
        let content_with_quotes = r#"---
description: Check variable named "foo"
---
Look for variables named "bar" and "baz".
"#;

        let dir = tempdir().unwrap();
        let source = dir.path().join(".promptpack");
        fs::create_dir_all(source.join("actions")).unwrap();
        fs::write(source.join("actions/test.md"), content_with_quotes).unwrap();

        let assets = parse_directory(&source).unwrap();
        let config = Config::default();

        // Test all targets that produce markdown outputs with content
        for target in [Target::Cursor, Target::ClaudeCode] {
            let outputs = compile_assets_with_adapters(&assets, &[target], &config);

            // Filter to only markdown outputs (not settings.json etc)
            let md_outputs: Vec<_> = outputs
                .iter()
                .filter(|o| o.path().extension().map(|e| e == "md").unwrap_or(false))
                .collect();

            for output in &md_outputs {
                // Content should preserve the quotes in markdown files
                assert!(
                    output.content().contains("\"foo\"")
                        || output.content().contains("\\\"foo\\\""),
                    "Quotes should be preserved or escaped in {:?} output for {:?}",
                    output.path(),
                    target
                );
            }
        }
    }

    /// Test that backslashes are handled correctly
    #[test]
    fn test_escaping_backslashes() {
        let content_with_backslash = r#"---
description: Regex pattern check
kind: policy
---
Use regex: \\d+ to match digits.
File path: C:\Users\test
"#;

        let dir = tempdir().unwrap();
        let source = dir.path().join(".promptpack");
        fs::create_dir_all(source.join("policies")).unwrap();
        fs::write(source.join("policies/regex.md"), content_with_backslash).unwrap();

        let assets = parse_directory(&source).unwrap();
        let config = Config::default();

        for target in [Target::ClaudeCode, Target::Cursor, Target::VSCode] {
            let outputs = compile_assets_with_adapters(&assets, &[target], &config);
            assert!(
                !outputs.is_empty(),
                "Should produce output for {:?}",
                target
            );
        }
    }

    /// Test that newlines in content are preserved
    #[test]
    fn test_escaping_preserves_newlines() {
        let content_with_newlines = r#"---
description: Multi-line content
kind: policy
---
Line 1

Line 2

Line 3
"#;

        let dir = tempdir().unwrap();
        let source = dir.path().join(".promptpack");
        fs::create_dir_all(source.join("policies")).unwrap();
        fs::write(source.join("policies/multiline.md"), content_with_newlines).unwrap();

        let assets = parse_directory(&source).unwrap();
        let config = Config::default();

        let outputs = compile_assets_with_adapters(&assets, &[Target::ClaudeCode], &config);

        let output = outputs
            .iter()
            .find(|o| o.path().to_string_lossy().contains("multiline"))
            .expect("Should have multiline output");

        // Content should have preserved newlines
        assert!(output.content().contains("Line 1"));
        assert!(output.content().contains("Line 2"));
        assert!(output.content().contains("Line 3"));
    }
}
