//! Claude Code adapter
//!
//! Generates output for Claude Code (Anthropic):
//! - `.claude/commands/<id>.md` - Slash commands
//! - `.claude/settings.json` - Permission settings with deny lists

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::adapters::{Diagnostic, DiagnosticSeverity, OutputFile, TargetAdapter};
use crate::error::CalvinResult;
use crate::models::{PromptAsset, Target};

/// Minimum deny list (always applied per TD-16)
const MINIMUM_DENY: &[&str] = &[
    ".env",
    ".env.*",
    "*.pem",
    "*.key",
    "id_rsa",
    "id_ed25519",
    ".git/",
];

/// Claude Code adapter
pub struct ClaudeCodeAdapter;

impl ClaudeCodeAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Generate settings.json content with deny lists
    pub fn generate_settings(&self, custom_deny: &[String]) -> String {
        let settings = ClaudeSettings::with_deny(custom_deny);
        serde_json::to_string_pretty(&settings).unwrap_or_default()
    }
}

impl Default for ClaudeCodeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl TargetAdapter for ClaudeCodeAdapter {
    fn target(&self) -> Target {
        Target::ClaudeCode
    }

    fn compile(&self, asset: &PromptAsset) -> CalvinResult<Vec<OutputFile>> {
        let mut outputs = Vec::new();

        // Generate command file
        let command_path = PathBuf::from(".claude/commands")
            .join(format!("{}.md", asset.id));

        let header = self.header(&asset.source_path.display().to_string());
        let content = format!(
            "{}{}\n\n{}",
            header,
            asset.frontmatter.description,
            asset.content.trim()
        );

        outputs.push(OutputFile::new(command_path, content));

        Ok(outputs)
    }

    fn validate(&self, output: &OutputFile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        
        if output.content.trim().is_empty() {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Warning,
                message: "Generated output is empty".to_string(),
                file: Some(output.path.clone()),
            });
        }
        
        // Check for undocumented named placeholders (like Codex adapter)
        // This addresses TODO.md L70: Handle $ARGUMENTS for inputs
        let content = &output.content;
        let chars: Vec<char> = content.chars().collect();
        let mut i = 0;
        
        while i < chars.len() {
            if chars[i] == '$' && i + 1 < chars.len() {
                let start = i + 1;
                let mut end = start;
                while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
                    end += 1;
                }
                if end > start {
                    let key: String = chars[start..end].iter().collect();
                    // Skip standard placeholders
                    if !["ARGUMENTS", "1", "2", "3", "4", "5", "6", "7", "8", "9"].contains(&key.as_str()) {
                        // This is a named placeholder - check if documented
                        if !content.contains(&format!("`{}`", key)) {
                            diagnostics.push(Diagnostic {
                                severity: DiagnosticSeverity::Warning,
                                message: format!(
                                    "Named placeholder ${} should be documented in command description",
                                    key
                                ),
                                file: Some(output.path.clone()),
                            });
                        }
                    }
                }
                i = end;
            } else {
                i += 1;
            }
        }
        
        diagnostics
    }

    fn security_baseline(&self, _config: &crate::config::Config) -> Vec<OutputFile> {
        let settings = ClaudeSettings::with_deny(&[]);
        let content = serde_json::to_string_pretty(&settings).unwrap_or_default();
        
        vec![
            OutputFile::new(".claude/settings.json", content),
            // Generate basic local settings file if it doesn't exist
            // This file is intended for local overrides and should be gitignored
            OutputFile::new(".claude/settings.local.json", "{}"),
        ]
    }
}

/// Claude settings.json structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Permissions>,
}

/// Permissions section of settings.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permissions {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub deny: Vec<String>,
}

impl ClaudeSettings {
    /// Create settings with minimum deny list plus custom entries
    pub fn with_deny(custom: &[String]) -> Self {
        let mut deny: Vec<String> = MINIMUM_DENY.iter().map(|s| s.to_string()).collect();
        deny.extend(custom.iter().cloned());
        // Remove duplicates while preserving order
        deny.sort();
        deny.dedup();

        Self {
            permissions: Some(Permissions { deny }),
        }
    }

    /// Create empty settings
    pub fn empty() -> Self {
        Self { permissions: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Frontmatter;

    #[test]
    fn test_claude_adapter_compile_basic() {
        let adapter = ClaudeCodeAdapter::new();
        let fm = Frontmatter::new("Test command");
        let asset = PromptAsset::new(
            "test-command",
            "actions/test-command.md",
            fm,
            "# Test\n\nContent here",
        );

        let outputs = adapter.compile(&asset).unwrap();
        
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].path, PathBuf::from(".claude/commands/test-command.md"));
        assert!(outputs[0].content.contains("Generated by Calvin"));
        assert!(outputs[0].content.contains("Test command"));
        assert!(outputs[0].content.contains("# Test"));
    }

    #[test]
    fn test_claude_settings_with_minimum_deny() {
        let settings = ClaudeSettings::with_deny(&[]);
        
        let perms = settings.permissions.unwrap();
        assert!(perms.deny.contains(&".env".to_string()));
        assert!(perms.deny.contains(&"*.pem".to_string()));
        assert!(perms.deny.contains(&".git/".to_string()));
    }

    #[test]
    fn test_claude_settings_with_custom_deny() {
        let custom = vec!["secrets/**".to_string(), "credentials.json".to_string()];
        let settings = ClaudeSettings::with_deny(&custom);
        
        let perms = settings.permissions.unwrap();
        assert!(perms.deny.contains(&".env".to_string())); // minimum
        assert!(perms.deny.contains(&"secrets/**".to_string())); // custom
        assert!(perms.deny.contains(&"credentials.json".to_string())); // custom
    }

    #[test]
    fn test_claude_settings_json_format() {
        let settings = ClaudeSettings::with_deny(&["secrets/**".to_string()]);
        let json = serde_json::to_string_pretty(&settings).unwrap();
        
        assert!(json.contains("permissions"));
        assert!(json.contains("deny"));
        assert!(json.contains(".env"));
        
        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["permissions"]["deny"].is_array());
    }

    #[test]
    fn test_claude_security_baseline() {
        let adapter = ClaudeCodeAdapter::new();
        let config = crate::config::Config::default();
        let baseline = adapter.security_baseline(&config);
        
        assert_eq!(baseline.len(), 2);
        assert_eq!(baseline[0].path, PathBuf::from(".claude/settings.json"));
        assert_eq!(baseline[1].path, PathBuf::from(".claude/settings.local.json"));
        assert!(baseline[0].content.contains("deny"));
    }

    #[test]
    fn test_claude_adapter_header() {
        let adapter = ClaudeCodeAdapter::new();
        let header = adapter.header("actions/test.md");
        
        assert!(header.contains("Generated by Calvin"));
        assert!(header.contains("actions/test.md"));
        assert!(header.contains("DO NOT EDIT"));
    }

    // === TDD: $ARGUMENTS Handling Tests (P0 Fix) ===

    #[test]
    fn test_claude_arguments_preserved() {
        // $ARGUMENTS should be preserved in output
        let adapter = ClaudeCodeAdapter::new();
        let fm = Frontmatter::new("Test with args");
        let asset = PromptAsset::new(
            "test-args",
            "actions/test.md",
            fm,
            "Process input: $ARGUMENTS",
        );

        let outputs = adapter.compile(&asset).unwrap();
        
        assert!(outputs[0].content.contains("$ARGUMENTS"));
    }

    #[test]
    fn test_claude_arguments_json_escaping() {
        // When $ARGUMENTS contains quotes, they should be escaped properly
        // This is the core P0 fix for TD-4 in Claude Code context
        let adapter = ClaudeCodeAdapter::new();
        let fm = Frontmatter::new("Check variable");
        let asset = PromptAsset::new(
            "check-var",
            "actions/check.md",
            fm,
            r#"Check if variable is named "foo" using $ARGUMENTS"#,
        );

        let outputs = adapter.compile(&asset).unwrap();
        
        // Content should be preserved without corruption
        assert!(outputs[0].content.contains(r#""foo""#) || outputs[0].content.contains(r#"\"foo\""#));
        assert!(outputs[0].content.contains("$ARGUMENTS"));
    }

    #[test]
    fn test_claude_validate_undocumented_placeholder() {
        // Claude Code should warn about undocumented named placeholders like Codex does
        let adapter = ClaudeCodeAdapter::new();
        let output = OutputFile::new(
            ".claude/commands/test.md",
            "Use $PROJECT_NAME for the project",
        );

        let diagnostics = adapter.validate(&output);
        
        // Should warn about undocumented placeholder
        assert!(!diagnostics.is_empty(), "Should have diagnostic for undocumented $PROJECT_NAME");
        assert!(diagnostics.iter().any(|d| d.message.contains("PROJECT_NAME")));
    }

    #[test]
    fn test_claude_validate_documented_placeholder() {
        // Documented placeholders should not trigger warnings
        let adapter = ClaudeCodeAdapter::new();
        let output = OutputFile::new(
            ".claude/commands/test.md",
            "Use $PROJECT_NAME for the project.\n\n`PROJECT_NAME` - The name of the project",
        );

        let diagnostics = adapter.validate(&output);
        
        // Should NOT warn about documented placeholder
        assert!(
            diagnostics.iter().all(|d| !d.message.contains("PROJECT_NAME")),
            "Should not warn about documented placeholder"
        );
    }

    #[test]
    fn test_claude_validate_standard_placeholders_ignored() {
        // Standard placeholders like $ARGUMENTS, $1-$9 should not trigger warnings
        let adapter = ClaudeCodeAdapter::new();
        let output = OutputFile::new(
            ".claude/commands/test.md",
            "Use $ARGUMENTS and $1 and $2 for inputs",
        );

        let diagnostics = adapter.validate(&output);
        
        // Standard placeholders should not trigger warnings
        assert!(
            diagnostics.is_empty(),
            "Standard placeholders should not trigger warnings"
        );
    }
}
