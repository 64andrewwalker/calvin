//! Claude Code Adapter
//!
//! Generates output for Claude Code (Anthropic):
//! - `.claude/commands/<id>.md` - Slash commands
//!
//! Path matrix (from platform.md):
//! - Project scope: `.claude/commands/`
//! - User scope: `~/.claude/commands/`

use std::path::PathBuf;

use super::skills;
use crate::domain::entities::{Asset, AssetKind, OutputFile};
use crate::domain::ports::target_adapter::{
    AdapterDiagnostic, AdapterError, DiagnosticSeverity, TargetAdapter,
};
use crate::domain::value_objects::{Scope, Target};

/// Claude Code adapter
pub struct ClaudeCodeAdapter;

impl ClaudeCodeAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Get the commands directory based on scope
    fn commands_dir(&self, scope: Scope) -> PathBuf {
        match scope {
            Scope::User => PathBuf::from("~/.claude/commands"),
            Scope::Project => PathBuf::from(".claude/commands"),
        }
    }

    /// Get the skills directory based on scope
    fn skills_dir(&self, scope: Scope) -> PathBuf {
        match scope {
            Scope::User => PathBuf::from("~/.claude/skills"),
            Scope::Project => PathBuf::from(".claude/skills"),
        }
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

    fn compile(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError> {
        if asset.kind() == AssetKind::Skill {
            let footer = self.footer(&asset.source_path_normalized());
            return skills::compile_skill_outputs(
                asset,
                self.skills_dir(asset.scope()),
                Target::ClaudeCode,
                &footer,
            );
        }

        let mut outputs = Vec::new();

        // Generate command file for all asset types
        // Claude Code uses .claude/commands/ for commands
        // Note: Even policies are generated as commands to maintain backward compatibility
        let commands_dir = self.commands_dir(asset.scope());
        let command_path = commands_dir.join(format!("{}.md", asset.id()));

        // Put footer at the END so it doesn't interfere with Claude Code's command preview
        // Claude Code shows the first line as the command description
        let footer = self.footer(&asset.source_path_normalized());

        // Determine first line for command preview:
        // Priority: description > H1 title > first paragraph
        let has_description = !asset.description().trim().is_empty();

        let content = if has_description {
            // Has description - prepend it as first line for command preview
            format!(
                "{}\n\n{}\n\n{}",
                asset.description(),
                asset.content().trim(),
                footer
            )
        } else {
            // No description - use content as-is (H1 or first line becomes preview)
            format!("{}\n\n{}", asset.content().trim(), footer)
        };

        outputs.push(OutputFile::new(command_path, content, Target::ClaudeCode));

        Ok(outputs)
    }

    fn validate(&self, output: &OutputFile) -> Vec<AdapterDiagnostic> {
        let mut diagnostics = Vec::new();

        // Skill: warn on dangerous tools (best-effort; do not fail compilation).
        if output
            .path()
            .file_name()
            .is_some_and(|n| n == std::ffi::OsStr::new("SKILL.md"))
        {
            diagnostics.extend(skills::validate_skill_allowed_tools(output));
        }

        if output.content().trim().is_empty() {
            diagnostics.push(AdapterDiagnostic {
                severity: DiagnosticSeverity::Warning,
                message: "Generated output is empty".to_string(),
            });
        }

        // Check for undocumented named placeholders
        let content = output.content();
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
                    if !["ARGUMENTS", "1", "2", "3", "4", "5", "6", "7", "8", "9"]
                        .contains(&key.as_str())
                    {
                        // This is a named placeholder - check if documented
                        if !content.contains(&format!("`{}`", key)) {
                            diagnostics.push(AdapterDiagnostic {
                                severity: DiagnosticSeverity::Warning,
                                message: format!(
                                    "Named placeholder ${} should be documented in command description",
                                    key
                                ),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::AssetKind;
    use std::collections::HashMap;

    fn create_action_asset(id: &str, description: &str, content: &str) -> Asset {
        Asset::new(id, format!("actions/{}.md", id), description, content)
            .with_kind(AssetKind::Action)
    }

    fn create_policy_asset(id: &str, description: &str, content: &str) -> Asset {
        Asset::new(id, format!("policies/{}.md", id), description, content)
            .with_kind(AssetKind::Policy)
    }

    fn create_skill_asset(id: &str, description: &str, content: &str) -> Asset {
        let mut supplementals: HashMap<PathBuf, String> = HashMap::new();
        supplementals.insert(PathBuf::from("reference.md"), "# Ref".to_string());
        supplementals.insert(
            PathBuf::from("scripts/validate.py"),
            "print('ok')\n".to_string(),
        );

        Asset::new(id, format!("skills/{}/SKILL.md", id), description, content)
            .with_kind(AssetKind::Skill)
            .with_supplementals(supplementals)
    }

    // === TDD: Compile Tests ===

    #[test]
    fn compile_action_generates_command() {
        let adapter = ClaudeCodeAdapter::new();
        let asset = create_action_asset("test-cmd", "Test command", "# Test\n\nContent here");

        let outputs = adapter.compile(&asset).unwrap();

        assert_eq!(outputs.len(), 1);
        assert_eq!(
            outputs[0].path(),
            &PathBuf::from(".claude/commands/test-cmd.md")
        );
    }

    #[test]
    fn compile_action_with_description_prepends_it() {
        let adapter = ClaudeCodeAdapter::new();
        let asset = create_action_asset("test", "My description", "# Title\n\nBody");

        let outputs = adapter.compile(&asset).unwrap();

        // Description should be first line
        assert!(outputs[0].content().starts_with("My description"));
        assert!(outputs[0].content().contains("# Title"));
    }

    #[test]
    fn compile_action_without_description_uses_content() {
        let adapter = ClaudeCodeAdapter::new();
        let asset = create_action_asset("test", "", "# First Line\n\nBody");

        let outputs = adapter.compile(&asset).unwrap();

        // Content starts with the H1
        assert!(outputs[0].content().starts_with("# First Line"));
    }

    #[test]
    fn compile_action_includes_footer() {
        let adapter = ClaudeCodeAdapter::new();
        let asset = create_action_asset("test", "desc", "content");

        let outputs = adapter.compile(&asset).unwrap();

        assert!(outputs[0].content().contains("Generated by Calvin"));
        assert!(outputs[0].content().contains("DO NOT EDIT"));
    }

    #[test]
    fn compile_policy_generates_command() {
        // For backward compatibility, policies also generate commands
        let adapter = ClaudeCodeAdapter::new();
        let asset = create_policy_asset("security", "Security rules", "# Rules");

        let outputs = adapter.compile(&asset).unwrap();

        assert_eq!(outputs.len(), 1);
        assert_eq!(
            outputs[0].path(),
            &PathBuf::from(".claude/commands/security.md")
        );
    }

    #[test]
    fn compile_user_scope_uses_home_path() {
        let adapter = ClaudeCodeAdapter::new();
        let asset = create_action_asset("test", "desc", "content").with_scope(Scope::User);

        let outputs = adapter.compile(&asset).unwrap();

        assert_eq!(
            outputs[0].path(),
            &PathBuf::from("~/.claude/commands/test.md")
        );
    }

    // === TDD: Skills ===

    #[test]
    fn test_claude_code_compile_skill_path() {
        let adapter = ClaudeCodeAdapter::new();
        let asset = create_skill_asset("my-skill", "My skill", "# Instructions");

        let outputs = adapter.compile(&asset).unwrap();

        assert!(outputs
            .iter()
            .any(|o| o.path() == &PathBuf::from(".claude/skills/my-skill/SKILL.md")));
    }

    #[test]
    fn test_claude_code_compile_skill_supplementals() {
        let adapter = ClaudeCodeAdapter::new();
        let asset = create_skill_asset("my-skill", "My skill", "# Instructions");

        let outputs = adapter.compile(&asset).unwrap();

        assert!(outputs
            .iter()
            .any(|o| o.path() == &PathBuf::from(".claude/skills/my-skill/reference.md")));
        assert!(outputs.iter().any(|o| {
            o.path() == &PathBuf::from(".claude/skills/my-skill/scripts/validate.py")
        }));
    }

    #[test]
    fn test_claude_code_skill_frontmatter() {
        let adapter = ClaudeCodeAdapter::new();
        let asset = create_skill_asset("my-skill", "My skill", "# Instructions")
            .with_allowed_tools(vec!["git".to_string(), "cat".to_string()]);

        let outputs = adapter.compile(&asset).unwrap();
        let skill = outputs
            .iter()
            .find(|o| o.path() == &PathBuf::from(".claude/skills/my-skill/SKILL.md"))
            .unwrap();

        assert!(skill.content().starts_with("---\n"));
        assert!(skill.content().contains("name: my-skill"));
        assert!(skill.content().contains("description: My skill"));
        assert!(skill.content().contains("allowed-tools:"));
        assert!(skill.content().contains("- git"));
        assert!(skill.content().contains("- cat"));
    }

    #[test]
    fn test_skill_user_scope_uses_home_path() {
        let adapter = ClaudeCodeAdapter::new();
        let asset =
            create_skill_asset("my-skill", "My skill", "# Instructions").with_scope(Scope::User);

        let outputs = adapter.compile(&asset).unwrap();

        assert!(outputs
            .iter()
            .any(|o| { o.path() == &PathBuf::from("~/.claude/skills/my-skill/SKILL.md") }));
    }

    #[test]
    fn test_skill_dangerous_tool_warning() {
        let adapter = ClaudeCodeAdapter::new();
        let asset = create_skill_asset("my-skill", "My skill", "# Instructions")
            .with_allowed_tools(vec!["rm".to_string()]);

        let outputs = adapter.compile(&asset).unwrap();
        let skill = outputs
            .iter()
            .find(|o| o.path() == &PathBuf::from(".claude/skills/my-skill/SKILL.md"))
            .unwrap();

        let diags = adapter.validate(skill);
        assert!(
            diags
                .iter()
                .any(|d| d.severity == DiagnosticSeverity::Warning),
            "expected warning diagnostics, got: {:?}",
            diags
        );
        assert!(diags.iter().any(|d| d.message.contains("rm")));
    }

    // === TDD: Validate Tests ===

    #[test]
    fn validate_empty_content_warns() {
        let adapter = ClaudeCodeAdapter::new();
        let output = OutputFile::new(".claude/commands/test.md", "", Target::ClaudeCode);

        let diags = adapter.validate(&output);

        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, DiagnosticSeverity::Warning);
    }

    #[test]
    fn validate_standard_placeholders_no_warning() {
        let adapter = ClaudeCodeAdapter::new();
        let output = OutputFile::new(
            ".claude/commands/test.md",
            "Use $ARGUMENTS and $1 and $2",
            Target::ClaudeCode,
        );

        let diags = adapter.validate(&output);

        assert!(diags.is_empty());
    }

    #[test]
    fn validate_undocumented_placeholder_warns() {
        let adapter = ClaudeCodeAdapter::new();
        let output = OutputFile::new(
            ".claude/commands/test.md",
            "Use $PROJECT_NAME in your code",
            Target::ClaudeCode,
        );

        let diags = adapter.validate(&output);

        assert!(!diags.is_empty());
        assert!(diags[0].message.contains("PROJECT_NAME"));
    }

    #[test]
    fn validate_documented_placeholder_no_warning() {
        let adapter = ClaudeCodeAdapter::new();
        let output = OutputFile::new(
            ".claude/commands/test.md",
            "Use $PROJECT_NAME in your code.\n\n`PROJECT_NAME` - The project name",
            Target::ClaudeCode,
        );

        let diags = adapter.validate(&output);

        // Should not warn about documented placeholder
        assert!(
            diags.iter().all(|d| !d.message.contains("PROJECT_NAME")),
            "Should not warn about documented placeholder"
        );
    }

    // === TDD: Trait Implementation ===

    #[test]
    fn adapter_target_is_claude_code() {
        let adapter = ClaudeCodeAdapter::new();
        assert_eq!(adapter.target(), Target::ClaudeCode);
    }

    #[test]
    fn adapter_version_is_one() {
        let adapter = ClaudeCodeAdapter::new();
        assert_eq!(adapter.version(), 1);
    }
}
