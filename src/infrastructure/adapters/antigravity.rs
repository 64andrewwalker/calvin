//! Google Antigravity Adapter
//!
//! Generates output for Google Antigravity:
//! - `.agent/rules/<id>.md` - Rules (Policy)
//! - `.agent/workflows/<id>.md` - Workflows (Action/Agent)
//!
//! Path matrix (from platform.md):
//! - Project scope: `.agent/rules/` or `.agent/workflows/`
//! - User scope: `~/.gemini/antigravity/global_rules/` or `~/.gemini/antigravity/global_workflows/`
//!
//! Improvement over legacy adapter:
//! - Distinguishes between rules (Policy) and workflows (Action/Agent)
//! - Supports globs via apply field

use std::path::PathBuf;

use crate::domain::entities::{Asset, AssetKind, OutputFile};
use crate::domain::ports::target_adapter::{
    AdapterDiagnostic, AdapterError, DiagnosticSeverity, TargetAdapter,
};
use crate::domain::value_objects::{Scope, Target};

/// Antigravity adapter
pub struct AntigravityAdapter;

impl AntigravityAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Get the output directory based on asset kind and scope
    fn output_dir(&self, kind: AssetKind, scope: Scope) -> PathBuf {
        match (kind, scope) {
            (AssetKind::Policy, Scope::Project) => PathBuf::from(".agent/rules"),
            (AssetKind::Policy, Scope::User) => PathBuf::from("~/.gemini/antigravity/global_rules"),
            (AssetKind::Action | AssetKind::Agent, Scope::Project) => {
                PathBuf::from(".agent/workflows")
            }
            (AssetKind::Action | AssetKind::Agent, Scope::User) => {
                PathBuf::from("~/.gemini/antigravity/global_workflows")
            }
            // Skills are not supported on Antigravity, but we still provide a deterministic
            // path for internal callers (compile() returns early for skills).
            (AssetKind::Skill, Scope::Project) => PathBuf::from(".agent/workflows"),
            (AssetKind::Skill, Scope::User) => {
                PathBuf::from("~/.gemini/antigravity/global_workflows")
            }
        }
    }

    /// Generate frontmatter
    fn generate_frontmatter(&self, asset: &Asset) -> String {
        let mut fm = String::from("---\n");
        fm.push_str(&format!("description: {}\n", asset.description()));

        // Add globs if apply pattern is specified
        if let Some(apply) = asset.apply() {
            fm.push_str(&format!("globs: \"{}\"\n", apply));
        }

        fm.push_str("---\n");
        fm
    }
}

impl Default for AntigravityAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl TargetAdapter for AntigravityAdapter {
    fn target(&self) -> Target {
        Target::Antigravity
    }

    fn compile(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError> {
        // Skills are not supported on Antigravity (PRD: no fallback compilation).
        if asset.kind() == AssetKind::Skill {
            return Ok(Vec::new());
        }

        let mut outputs = Vec::new();

        let output_dir = self.output_dir(asset.kind(), asset.scope());
        let path = output_dir.join(format!("{}.md", asset.id()));

        let frontmatter = self.generate_frontmatter(asset);
        let footer = self.footer(&asset.source_path_normalized());
        let content = format!("{}\n{}\n\n{}", frontmatter, asset.content().trim(), footer);

        outputs.push(OutputFile::new(path, content, self.target()));

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

        diagnostics
    }

    fn security_baseline(
        &self,
        _config: &crate::config::Config,
    ) -> Result<Vec<OutputFile>, AdapterError> {
        // Antigravity security is configured via user-level config, not project-level
        Ok(Vec::new())
    }
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

    fn create_agent_asset(id: &str, description: &str, content: &str) -> Asset {
        Asset::new(id, format!("agents/{}.md", id), description, content)
            .with_kind(AssetKind::Agent)
    }

    fn create_skill_asset(id: &str, description: &str, content: &str) -> Asset {
        let supplementals: HashMap<PathBuf, String> = HashMap::new();
        Asset::new(id, format!("skills/{}/SKILL.md", id), description, content)
            .with_kind(AssetKind::Skill)
            .with_supplementals(supplementals)
    }

    // === TDD: Compile Tests ===

    #[test]
    fn compile_policy_to_rules_dir() {
        let adapter = AntigravityAdapter::new();
        let asset = create_policy_asset("code-style", "Code style", "# Rules");

        let outputs = adapter.compile(&asset).unwrap();

        assert_eq!(outputs.len(), 1);
        assert_eq!(
            outputs[0].path(),
            &PathBuf::from(".agent/rules/code-style.md")
        );
    }

    #[test]
    fn compile_action_to_workflows_dir() {
        let adapter = AntigravityAdapter::new();
        let asset = create_action_asset("build", "Build project", "# Build");

        let outputs = adapter.compile(&asset).unwrap();

        assert_eq!(
            outputs[0].path(),
            &PathBuf::from(".agent/workflows/build.md")
        );
    }

    #[test]
    fn compile_agent_to_workflows_dir() {
        let adapter = AntigravityAdapter::new();
        let asset = create_agent_asset("reviewer", "Code reviewer", "You are a reviewer.");

        let outputs = adapter.compile(&asset).unwrap();

        assert_eq!(
            outputs[0].path(),
            &PathBuf::from(".agent/workflows/reviewer.md")
        );
    }

    #[test]
    fn compile_policy_user_scope() {
        let adapter = AntigravityAdapter::new();
        let asset =
            create_policy_asset("global-rules", "Global", "# Global").with_scope(Scope::User);

        let outputs = adapter.compile(&asset).unwrap();

        assert_eq!(
            outputs[0].path(),
            &PathBuf::from("~/.gemini/antigravity/global_rules/global-rules.md")
        );
    }

    #[test]
    fn compile_action_user_scope() {
        let adapter = AntigravityAdapter::new();
        let asset =
            create_action_asset("global-build", "Global build", "# Build").with_scope(Scope::User);

        let outputs = adapter.compile(&asset).unwrap();

        assert_eq!(
            outputs[0].path(),
            &PathBuf::from("~/.gemini/antigravity/global_workflows/global-build.md")
        );
    }

    #[test]
    fn compile_with_apply_includes_globs() {
        let adapter = AntigravityAdapter::new();
        let asset = create_policy_asset("rust-rules", "Rust rules", "# Rust").with_apply("**/*.rs");

        let outputs = adapter.compile(&asset).unwrap();

        assert!(outputs[0].content().contains("globs: \"**/*.rs\""));
    }

    #[test]
    fn compile_includes_frontmatter() {
        let adapter = AntigravityAdapter::new();
        let asset = create_policy_asset("test", "Test description", "Content");

        let outputs = adapter.compile(&asset).unwrap();

        assert!(outputs[0].content().starts_with("---\n"));
        assert!(outputs[0]
            .content()
            .contains("description: Test description"));
    }

    #[test]
    fn compile_includes_footer() {
        let adapter = AntigravityAdapter::new();
        let asset = create_policy_asset("test", "desc", "content");

        let outputs = adapter.compile(&asset).unwrap();

        assert!(outputs[0].content().contains("Generated by Calvin"));
        assert!(outputs[0].content().contains("DO NOT EDIT"));
    }

    #[test]
    fn test_antigravity_compile_skill_returns_empty() {
        let adapter = AntigravityAdapter::new();
        let asset = create_skill_asset("my-skill", "My skill", "# Instructions");

        let outputs = adapter.compile(&asset).unwrap();
        assert!(outputs.is_empty());
    }

    // === TDD: Validate Tests ===

    #[test]
    fn validate_empty_content_warns() {
        let adapter = AntigravityAdapter::new();
        let output = OutputFile::new(".agent/rules/test.md", "", Target::Antigravity);

        let diags = adapter.validate(&output);

        assert!(!diags.is_empty());
        assert!(diags.iter().any(|d| d.message.contains("empty")));
    }

    #[test]
    fn validate_valid_content_no_warnings() {
        let adapter = AntigravityAdapter::new();
        let output = OutputFile::new(
            ".agent/rules/test.md",
            "---\ndescription: Test\n---\n\n# Content",
            Target::Antigravity,
        );

        let diags = adapter.validate(&output);

        assert!(diags.is_empty());
    }

    // === TDD: Security Baseline ===

    #[test]
    fn security_baseline_returns_empty() {
        let adapter = AntigravityAdapter::new();
        let config = crate::config::Config::default();

        let baseline = adapter.security_baseline(&config).unwrap();

        assert!(baseline.is_empty());
    }

    // === TDD: Trait Implementation ===

    #[test]
    fn adapter_target_is_antigravity() {
        let adapter = AntigravityAdapter::new();
        assert_eq!(adapter.target(), Target::Antigravity);
    }

    #[test]
    fn adapter_version_is_one() {
        let adapter = AntigravityAdapter::new();
        assert_eq!(adapter.version(), 1);
    }
}
