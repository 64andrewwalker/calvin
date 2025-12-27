//! Cursor IDE Adapter
//!
//! Generates output for Cursor IDE:
//! - `.cursor/rules/<id>/RULE.md` - Rules with frontmatter
//!
//! Path matrix (from platform.md):
//! - Project scope: `.cursor/rules/`
//! - User scope: `~/.cursor/rules/` (macOS/Linux) or `%APPDATA%\Cursor\User\globalStorage\cursor.rules\` (Windows)
//!
//! Note: Cursor reads Claude's commands from ~/.claude/commands/,
//! so we only generate rules here, not commands.

use std::path::PathBuf;

use crate::domain::entities::{Asset, AssetKind, OutputFile};
use crate::domain::ports::target_adapter::{
    AdapterDiagnostic, AdapterError, DiagnosticSeverity, TargetAdapter,
};
use crate::domain::value_objects::{Scope, Target};

/// Cursor adapter
pub struct CursorAdapter;

impl CursorAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Get the rules directory based on scope
    fn rules_dir(&self, scope: Scope) -> PathBuf {
        match scope {
            Scope::User => PathBuf::from("~/.cursor/rules"),
            Scope::Project => PathBuf::from(".cursor/rules"),
        }
    }

    fn skills_dir(&self, scope: Scope) -> PathBuf {
        // Cursor reads skills from Claude Code's skill paths.
        match scope {
            Scope::User => PathBuf::from("~/.claude/skills"),
            Scope::Project => PathBuf::from(".claude/skills"),
        }
    }

    /// Generate Cursor RULE.md frontmatter
    fn generate_rule_frontmatter(&self, asset: &Asset) -> String {
        let mut fm = String::from("---\n");

        fm.push_str(&format!("description: {}\n", asset.description()));

        // Add globs if apply pattern is specified
        if let Some(apply) = asset.apply() {
            fm.push_str(&format!("globs: \"{}\"\n", apply));
        }

        // alwaysApply: true only for policies without apply pattern
        // Actions/Agents should NOT be alwaysApply
        if asset.kind() == AssetKind::Policy && asset.apply().is_none() {
            fm.push_str("alwaysApply: true\n");
        } else {
            fm.push_str("alwaysApply: false\n");
        }

        fm.push_str("---\n");
        fm
    }

    fn compile_skill(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError> {
        let mut outputs = Vec::new();

        let skill_dir = self.skills_dir(asset.scope()).join(asset.id());

        outputs.push(OutputFile::new(
            skill_dir.join("SKILL.md"),
            self.generate_skill_md(asset),
            Target::Cursor,
        ));

        for (rel_path, content) in asset.supplementals() {
            if rel_path.is_absolute()
                || rel_path
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                return Err(AdapterError::CompilationFailed {
                    message: format!(
                        "Invalid supplemental path for skill '{}': {}",
                        asset.id(),
                        rel_path.display()
                    ),
                });
            }
            outputs.push(OutputFile::new(
                skill_dir.join(rel_path),
                content.clone(),
                Target::Cursor,
            ));
        }

        Ok(outputs)
    }

    fn generate_skill_md(&self, asset: &Asset) -> String {
        let mut out = String::new();

        out.push_str("---\n");
        out.push_str(&format!("name: {}\n", asset.id()));
        out.push_str(&format!("description: {}\n", asset.description()));

        if !asset.allowed_tools().is_empty() {
            out.push_str("allowed-tools:\n");
            for tool in asset.allowed_tools() {
                out.push_str(&format!("  - {}\n", tool));
            }
        }

        out.push_str("---\n\n");
        out.push_str(asset.content().trim());
        out.push_str("\n\n");
        out.push_str(&self.footer(&asset.source_path_normalized()));

        out
    }
}

impl Default for CursorAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl TargetAdapter for CursorAdapter {
    fn target(&self) -> Target {
        Target::Cursor
    }

    fn compile(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError> {
        let mut outputs = Vec::new();

        // Cursor only generates rules for policies
        // Actions and agents are read from Claude's commands
        match asset.kind() {
            AssetKind::Policy => {
                // Generate rule file: <base>/rules/<id>/RULE.md
                let rules_dir = self.rules_dir(asset.scope());
                let rule_path = rules_dir.join(asset.id()).join("RULE.md");

                let frontmatter = self.generate_rule_frontmatter(asset);
                let footer = self.footer(&asset.source_path_normalized());
                let content = format!("{}\n{}\n\n{}", frontmatter, asset.content().trim(), footer);

                outputs.push(OutputFile::new(rule_path, content, Target::Cursor));
            }
            AssetKind::Action | AssetKind::Agent => {
                // Cursor reads Claude's commands from ~/.claude/commands/
                // No separate output needed
            }
            AssetKind::Skill => {
                outputs.extend(self.compile_skill(asset)?);
            }
        }

        Ok(outputs)
    }

    fn validate(&self, output: &OutputFile) -> Vec<AdapterDiagnostic> {
        let mut diagnostics = Vec::new();

        if output.content().trim().is_empty() {
            diagnostics.push(AdapterDiagnostic {
                severity: DiagnosticSeverity::Warning,
                message: "Generated output is empty".to_string(),
            });
        }

        if output
            .path()
            .file_name()
            .is_some_and(|n| n == std::ffi::OsStr::new("SKILL.md"))
        {
            diagnostics.extend(validate_skill_allowed_tools(output));
        }

        // Cursor rules should have frontmatter, but skills/supplementals should not be forced
        // into rule semantics.
        if output
            .path()
            .file_name()
            .is_some_and(|n| n == std::ffi::OsStr::new("RULE.md"))
            && !output.content().starts_with("---")
        {
            diagnostics.push(AdapterDiagnostic {
                severity: DiagnosticSeverity::Warning,
                message: "Cursor rule missing frontmatter".to_string(),
            });
        }

        diagnostics
    }
}

fn validate_skill_allowed_tools(output: &OutputFile) -> Vec<AdapterDiagnostic> {
    const DANGEROUS_TOOLS: &[&str] = &[
        "rm", "sudo", "chmod", "chown", "curl", "wget", "nc", "netcat", "ssh", "scp", "rsync",
    ];

    let extracted = match crate::parser::extract_frontmatter(output.content(), output.path()) {
        Ok(extracted) => extracted,
        Err(_) => return Vec::new(),
    };
    let fm = match crate::parser::parse_frontmatter(&extracted.yaml, output.path()) {
        Ok(fm) => fm,
        Err(_) => return Vec::new(),
    };

    let mut diags = Vec::new();
    for tool in &fm.allowed_tools {
        if DANGEROUS_TOOLS.contains(&tool.as_str()) {
            diags.push(AdapterDiagnostic {
                severity: DiagnosticSeverity::Warning,
                message: format!(
                    "Tool '{}' in allowed-tools may pose security risks. Ensure this is intentional.",
                    tool
                ),
            });
        }
    }
    diags
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_policy_asset(id: &str, description: &str, content: &str) -> Asset {
        Asset::new(id, format!("policies/{}.md", id), description, content)
            .with_kind(AssetKind::Policy)
    }

    fn create_action_asset(id: &str, description: &str, content: &str) -> Asset {
        Asset::new(id, format!("actions/{}.md", id), description, content)
            .with_kind(AssetKind::Action)
    }

    fn create_skill_asset(id: &str, description: &str, content: &str) -> Asset {
        let mut supplementals: HashMap<PathBuf, String> = HashMap::new();
        supplementals.insert(PathBuf::from("reference.md"), "# Ref".to_string());
        Asset::new(id, format!("skills/{}/SKILL.md", id), description, content)
            .with_kind(AssetKind::Skill)
            .with_supplementals(supplementals)
    }

    // === TDD: Compile Tests ===

    #[test]
    fn compile_policy_generates_rule() {
        let adapter = CursorAdapter::new();
        let asset = create_policy_asset("code-style", "Code style rules", "# Rules\n\nContent");

        let outputs = adapter.compile(&asset).unwrap();

        assert_eq!(outputs.len(), 1);
        assert_eq!(
            outputs[0].path(),
            &PathBuf::from(".cursor/rules/code-style/RULE.md")
        );
    }

    #[test]
    fn compile_policy_includes_frontmatter() {
        let adapter = CursorAdapter::new();
        let asset = create_policy_asset("test", "Test description", "Content");

        let outputs = adapter.compile(&asset).unwrap();

        assert!(outputs[0].content().starts_with("---\n"));
        assert!(outputs[0]
            .content()
            .contains("description: Test description"));
        assert!(outputs[0].content().contains("alwaysApply: true"));
    }

    #[test]
    fn compile_policy_with_apply_pattern() {
        let adapter = CursorAdapter::new();
        let asset = create_policy_asset("rust-style", "Rust rules", "Content").with_apply("*.rs");

        let outputs = adapter.compile(&asset).unwrap();

        assert!(outputs[0].content().contains("globs: \"*.rs\""));
        assert!(outputs[0].content().contains("alwaysApply: false"));
    }

    #[test]
    fn compile_policy_includes_footer() {
        let adapter = CursorAdapter::new();
        let asset = create_policy_asset("test", "desc", "content");

        let outputs = adapter.compile(&asset).unwrap();

        assert!(outputs[0].content().contains("Generated by Calvin"));
        assert!(outputs[0].content().contains("DO NOT EDIT"));
    }

    #[test]
    fn compile_action_generates_nothing() {
        let adapter = CursorAdapter::new();
        let asset = create_action_asset("gen-tests", "Generate tests", "# Generate");

        let outputs = adapter.compile(&asset).unwrap();

        assert!(outputs.is_empty());
    }

    #[test]
    fn compile_user_scope_uses_home_path() {
        let adapter = CursorAdapter::new();
        let asset =
            create_policy_asset("global", "Global rules", "content").with_scope(Scope::User);

        let outputs = adapter.compile(&asset).unwrap();

        assert_eq!(
            outputs[0].path(),
            &PathBuf::from("~/.cursor/rules/global/RULE.md")
        );
    }

    // === TDD: Skills ===

    #[test]
    fn test_cursor_compile_skill_uses_claude_path() {
        let adapter = CursorAdapter::new();
        let asset = create_skill_asset("my-skill", "My skill", "# Instructions");

        let outputs = adapter.compile(&asset).unwrap();

        assert!(outputs
            .iter()
            .any(|o| o.path() == &PathBuf::from(".claude/skills/my-skill/SKILL.md")));
        assert!(outputs.iter().all(|o| o.target() == Target::Cursor));
    }

    // === TDD: Validate Tests ===

    #[test]
    fn validate_empty_content_warns() {
        let adapter = CursorAdapter::new();
        let output = OutputFile::new(".cursor/rules/test/RULE.md", "", Target::Cursor);

        let diags = adapter.validate(&output);

        assert!(!diags.is_empty());
        assert!(diags.iter().any(|d| d.message.contains("empty")));
    }

    #[test]
    fn validate_missing_frontmatter_warns() {
        let adapter = CursorAdapter::new();
        let output = OutputFile::new(
            ".cursor/rules/test/RULE.md",
            "# Content without frontmatter",
            Target::Cursor,
        );

        let diags = adapter.validate(&output);

        assert!(diags.iter().any(|d| d.message.contains("frontmatter")));
    }

    #[test]
    fn validate_valid_rule_no_warnings() {
        let adapter = CursorAdapter::new();
        let output = OutputFile::new(
            ".cursor/rules/test/RULE.md",
            "---\ndescription: Test\nalwaysApply: true\n---\n\n# Content",
            Target::Cursor,
        );

        let diags = adapter.validate(&output);

        assert!(diags.is_empty());
    }

    // === TDD: Trait Implementation ===

    #[test]
    fn adapter_target_is_cursor() {
        let adapter = CursorAdapter::new();
        assert_eq!(adapter.target(), Target::Cursor);
    }

    #[test]
    fn adapter_version_is_one() {
        let adapter = CursorAdapter::new();
        assert_eq!(adapter.version(), 1);
    }

    // === TDD: Frontmatter Generation ===

    #[test]
    fn frontmatter_format_correct() {
        let adapter = CursorAdapter::new();
        let asset = create_policy_asset("test", "Test description", "");

        let fm = adapter.generate_rule_frontmatter(&asset);

        assert!(fm.starts_with("---\n"));
        assert!(fm.ends_with("---\n"));
        assert!(fm.contains("description: Test description"));
    }
}
