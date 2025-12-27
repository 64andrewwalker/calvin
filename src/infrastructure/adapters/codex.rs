//! OpenAI Codex CLI Adapter
//!
//! Generates output for Codex CLI prompts:
//! - `.codex/prompts/<id>.md` - Project-level prompts
//! - `~/.codex/prompts/<id>.md` - User-level prompts
//!
//! Path matrix (from platform.md):
//! - Project scope: `.codex/prompts/`
//! - User scope: `~/.codex/prompts/`
//!
//! Improvement over legacy adapter:
//! - Only Action/Agent include $ARGUMENTS placeholder (Policy does not)

use std::path::PathBuf;

use super::skills;
use crate::domain::entities::{Asset, AssetKind, OutputFile};
use crate::domain::ports::target_adapter::{
    AdapterDiagnostic, AdapterError, DiagnosticSeverity, TargetAdapter,
};
use crate::domain::value_objects::{Scope, Target};

/// Codex adapter
pub struct CodexAdapter;

impl CodexAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Get the prompts directory based on scope
    fn prompts_dir(&self, scope: Scope) -> PathBuf {
        match scope {
            Scope::User => PathBuf::from("~/.codex/prompts"),
            Scope::Project => PathBuf::from(".codex/prompts"),
        }
    }

    fn skills_dir(&self, scope: Scope) -> PathBuf {
        match scope {
            Scope::User => PathBuf::from("~/.codex/skills"),
            Scope::Project => PathBuf::from(".codex/skills"),
        }
    }

    /// Generate YAML frontmatter for Codex prompts
    fn generate_frontmatter(&self, asset: &Asset) -> String {
        let mut fm = String::from("---\n");
        fm.push_str(&format!("description: {}\n", asset.description()));

        // Only include argument-hint for Action/Agent (not Policy)
        match asset.kind() {
            AssetKind::Action | AssetKind::Agent => {
                fm.push_str("argument-hint: <arguments>\n");
            }
            AssetKind::Policy => {
                // Policy prompts don't need arguments
            }
            AssetKind::Skill => {
                // Skills use SKILL.md format, not Codex prompts frontmatter.
            }
        }

        fm.push_str("---\n");
        fm
    }

    fn compile_skill(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError> {
        let footer = self.footer(&asset.source_path_normalized());
        skills::compile_skill_outputs(
            asset,
            self.skills_dir(asset.scope()),
            Target::Codex,
            &footer,
        )
    }
}

impl Default for CodexAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl TargetAdapter for CodexAdapter {
    fn target(&self) -> Target {
        Target::Codex
    }

    fn compile(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError> {
        // Skills are compiled to `.codex/skills/<id>/SKILL.md` (implemented separately).
        if asset.kind() == AssetKind::Skill {
            return self.compile_skill(asset);
        }

        let mut outputs = Vec::new();

        let prompts_dir = self.prompts_dir(asset.scope());
        let path = prompts_dir.join(format!("{}.md", asset.id()));

        let frontmatter = self.generate_frontmatter(asset);
        let footer = self.footer(&asset.source_path_normalized());

        // Only Action/Agent include $ARGUMENTS after frontmatter
        let content = match asset.kind() {
            AssetKind::Action | AssetKind::Agent => {
                format!(
                    "{}\n$ARGUMENTS\n\n{}\n\n{}",
                    frontmatter,
                    asset.content().trim(),
                    footer
                )
            }
            AssetKind::Policy => {
                format!("{}\n{}\n\n{}", frontmatter, asset.content().trim(), footer)
            }
            AssetKind::Skill => String::new(), // unreachable (guarded above)
        };

        outputs.push(OutputFile::new(path, content, self.target()));

        Ok(outputs)
    }

    fn validate(&self, output: &OutputFile) -> Vec<AdapterDiagnostic> {
        let mut diagnostics = Vec::new();

        if output
            .path()
            .file_name()
            .is_some_and(|n| n == std::ffi::OsStr::new("SKILL.md"))
        {
            diagnostics.extend(skills::validate_skill_allowed_tools(output));
        }

        // Check for named placeholders without documentation
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
                    // Skip common placeholders
                    if !["ARGUMENTS", "1", "2", "3", "4", "5", "6", "7", "8", "9"]
                        .contains(&key.as_str())
                    {
                        // This is a named placeholder - should be documented
                        if !content.contains(&format!("`{}`", key)) {
                            diagnostics.push(AdapterDiagnostic {
                                severity: DiagnosticSeverity::Warning,
                                message: format!(
                                    "Named placeholder ${} should be documented in usage section",
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

    fn security_baseline(
        &self,
        _config: &crate::config::Config,
    ) -> Result<Vec<OutputFile>, AdapterError> {
        // Codex doesn't have project-level security config
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        Asset::new(id, format!("skills/{}/SKILL.md", id), description, content)
            .with_kind(AssetKind::Skill)
            .with_supplementals(supplementals)
    }

    // === TDD: Compile Tests ===

    #[test]
    fn compile_action_includes_arguments() {
        let adapter = CodexAdapter::new();
        let asset = create_action_asset("gen-tests", "Generate tests", "# Generate");

        let outputs = adapter.compile(&asset).unwrap();

        assert!(outputs[0].content().contains("$ARGUMENTS"));
        assert!(outputs[0].content().contains("argument-hint: <arguments>"));
    }

    #[test]
    fn compile_policy_no_arguments() {
        let adapter = CodexAdapter::new();
        let asset = create_policy_asset("code-style", "Code style", "# Style");

        let outputs = adapter.compile(&asset).unwrap();

        assert!(!outputs[0].content().contains("$ARGUMENTS"));
        assert!(!outputs[0].content().contains("argument-hint"));
    }

    #[test]
    fn compile_user_scope_uses_home() {
        let adapter = CodexAdapter::new();
        let asset = create_action_asset("test", "desc", "content").with_scope(Scope::User);

        let outputs = adapter.compile(&asset).unwrap();

        assert_eq!(
            outputs[0].path(),
            &PathBuf::from("~/.codex/prompts/test.md")
        );
    }

    #[test]
    fn compile_project_scope_local_path() {
        let adapter = CodexAdapter::new();
        let asset = create_action_asset("test", "desc", "content").with_scope(Scope::Project);

        let outputs = adapter.compile(&asset).unwrap();

        assert_eq!(outputs[0].path(), &PathBuf::from(".codex/prompts/test.md"));
        assert!(!outputs[0].path().to_string_lossy().starts_with("~"));
    }

    #[test]
    fn compile_includes_frontmatter() {
        let adapter = CodexAdapter::new();
        let asset = create_action_asset("test", "Test description", "content");

        let outputs = adapter.compile(&asset).unwrap();

        assert!(outputs[0].content().starts_with("---\n"));
        assert!(outputs[0]
            .content()
            .contains("description: Test description"));
    }

    #[test]
    fn compile_includes_footer() {
        let adapter = CodexAdapter::new();
        let asset = create_action_asset("test", "desc", "content");

        let outputs = adapter.compile(&asset).unwrap();

        assert!(outputs[0].content().contains("Generated by Calvin"));
        assert!(outputs[0].content().contains("DO NOT EDIT"));
    }

    // === TDD: Validate Tests ===

    #[test]
    fn validate_undocumented_placeholder_warns() {
        let adapter = CodexAdapter::new();
        let output = OutputFile::new(
            ".codex/prompts/test.md",
            "Use $PROJECT_NAME for the project",
            Target::Codex,
        );

        let diags = adapter.validate(&output);

        assert!(!diags.is_empty());
        assert!(diags[0].message.contains("PROJECT_NAME"));
    }

    #[test]
    fn validate_documented_placeholder_ok() {
        let adapter = CodexAdapter::new();
        let output = OutputFile::new(
            ".codex/prompts/test.md",
            "Use $PROJECT_NAME for the project.\n\n`PROJECT_NAME` - The name of the project",
            Target::Codex,
        );

        let diags = adapter.validate(&output);

        // Should not warn about documented placeholder
        assert!(diags.iter().all(|d| !d.message.contains("PROJECT_NAME")));
    }

    #[test]
    fn validate_standard_placeholders_ok() {
        let adapter = CodexAdapter::new();
        let output = OutputFile::new(
            ".codex/prompts/test.md",
            "Use $ARGUMENTS and $1 and $2",
            Target::Codex,
        );

        let diags = adapter.validate(&output);

        assert!(diags.is_empty());
    }

    // === TDD: Security Baseline ===

    #[test]
    fn security_baseline_returns_empty() {
        let adapter = CodexAdapter::new();
        let config = crate::config::Config::default();

        let baseline = adapter.security_baseline(&config).unwrap();

        assert!(baseline.is_empty());
    }

    // === TDD: Trait Implementation ===

    #[test]
    fn adapter_target_is_codex() {
        let adapter = CodexAdapter::new();
        assert_eq!(adapter.target(), Target::Codex);
    }

    #[test]
    fn adapter_version_is_one() {
        let adapter = CodexAdapter::new();
        assert_eq!(adapter.version(), 1);
    }

    // === TDD: Skills ===

    #[test]
    fn test_codex_compile_skill_path() {
        let adapter = CodexAdapter::new();
        let asset = create_skill_asset("my-skill", "My skill", "# Instructions");

        let outputs = adapter.compile(&asset).unwrap();

        assert!(outputs
            .iter()
            .any(|o| o.path() == &PathBuf::from(".codex/skills/my-skill/SKILL.md")));
    }
}
