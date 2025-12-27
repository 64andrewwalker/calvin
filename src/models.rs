//! Core data models for Calvin
//!
//! Defines the fundamental data structures used throughout Calvin:
//! - `Frontmatter`: YAML metadata extracted from source files
//! - `PromptAsset`: A parsed source file with frontmatter and content
//! - Supporting enums: `AssetKind`, `Scope`, `Target`

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Kind of prompt asset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AssetKind {
    /// Long-term rules (code style, security, testing policies)
    Policy,
    /// Triggered commands/workflows (generate tests, PR review)
    #[default]
    Action,
    /// Specialized sub-agents/roles
    Agent,
    /// Directory-based skills (SKILL.md + supplementals)
    Skill,
}

/// Scope of the asset (where it should be installed)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// Project-level (in repository directories)
    #[default]
    Project,
    /// User-level (in home directory)
    User,
}

// Re-export Target from domain layer for backward compatibility
pub use crate::domain::value_objects::Target;

/// YAML frontmatter extracted from source files
///
/// Only `description` is required. All other fields have sensible defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frontmatter {
    /// Description of the asset (REQUIRED)
    pub description: String,

    /// Kind of asset (policy, action, agent)
    #[serde(default)]
    pub kind: AssetKind,

    /// Where to install (project or user level)
    #[serde(default)]
    pub scope: Scope,

    /// Target platforms (defaults to all if not specified)
    #[serde(default)]
    pub targets: Vec<Target>,

    /// File glob pattern for conditional application (e.g., "*.rs")
    #[serde(default)]
    pub apply: Option<String>,

    /// Skill-only: allowed tools list (validated at compile/check time)
    ///
    /// Ignored for non-skill assets.
    #[serde(default, rename = "allowed-tools")]
    pub allowed_tools: Vec<String>,
}

impl Frontmatter {
    /// Create a new frontmatter with only the required description
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            kind: AssetKind::default(),
            scope: Scope::default(),
            targets: Vec::new(),
            apply: None,
            allowed_tools: Vec::new(),
        }
    }

    /// Get effective targets (returns all if targets is empty or contains All)
    pub fn effective_targets(&self) -> Vec<Target> {
        if self.targets.is_empty() || self.targets.contains(&Target::All) {
            vec![
                Target::ClaudeCode,
                Target::Cursor,
                Target::VSCode,
                Target::Antigravity,
                Target::Codex,
            ]
        } else {
            self.targets.clone()
        }
    }
}

/// A parsed prompt asset with frontmatter and content
#[derive(Debug, Clone, PartialEq)]
pub struct PromptAsset {
    /// Unique identifier (derived from filename, kebab-case)
    pub id: String,

    /// Source file path relative to .promptpack/
    pub source_path: PathBuf,

    /// Parsed frontmatter
    pub frontmatter: Frontmatter,

    /// Content body (after frontmatter)
    pub content: String,
}

impl PromptAsset {
    /// Create a new PromptAsset
    pub fn new(
        id: impl Into<String>,
        source_path: impl Into<PathBuf>,
        frontmatter: Frontmatter,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            source_path: source_path.into(),
            frontmatter,
            content: content.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === TDD Cycle 1: Core Data Structures ===

    #[test]
    fn test_frontmatter_deserialize_minimal() {
        let yaml = "description: Test action";
        let fm: Frontmatter = serde_yaml_ng::from_str(yaml).unwrap();

        assert_eq!(fm.description, "Test action");
        assert_eq!(fm.kind, AssetKind::Action); // default
        assert_eq!(fm.scope, Scope::Project); // default
        assert!(fm.targets.is_empty()); // default
        assert!(fm.apply.is_none()); // default
        assert!(fm.allowed_tools.is_empty()); // default
    }

    #[test]
    fn test_frontmatter_deserialize_full() {
        let yaml = r#"
description: Security rules
kind: policy
scope: project
targets:
  - claude-code
  - cursor
apply: "*.rs"
allowed-tools:
  - git
"#;
        let fm: Frontmatter = serde_yaml_ng::from_str(yaml).unwrap();

        assert_eq!(fm.description, "Security rules");
        assert_eq!(fm.kind, AssetKind::Policy);
        assert_eq!(fm.scope, Scope::Project);
        assert_eq!(fm.targets, vec![Target::ClaudeCode, Target::Cursor]);
        assert_eq!(fm.apply, Some("*.rs".to_string()));
        assert_eq!(fm.allowed_tools, vec!["git".to_string()]);
    }

    #[test]
    fn test_frontmatter_deserialize_action() {
        let yaml = r#"
description: Generate tests
kind: action
scope: user
"#;
        let fm: Frontmatter = serde_yaml_ng::from_str(yaml).unwrap();

        assert_eq!(fm.description, "Generate tests");
        assert_eq!(fm.kind, AssetKind::Action);
        assert_eq!(fm.scope, Scope::User);
    }

    #[test]
    fn test_frontmatter_deserialize_agent() {
        let yaml = r#"
description: Code reviewer agent
kind: agent
"#;
        let fm: Frontmatter = serde_yaml_ng::from_str(yaml).unwrap();

        assert_eq!(fm.kind, AssetKind::Agent);
    }

    #[test]
    fn test_frontmatter_skill_kind_parses() {
        let yaml = r#"
description: A skill
kind: skill
"#;
        let fm: Frontmatter = serde_yaml_ng::from_str(yaml).unwrap();

        assert_eq!(fm.kind, AssetKind::Skill);
    }

    #[test]
    fn test_frontmatter_allowed_tools_parses() {
        let yaml = r#"
description: A skill
kind: skill
allowed-tools:
  - git
  - cat
"#;
        let fm: Frontmatter = serde_yaml_ng::from_str(yaml).unwrap();

        assert_eq!(fm.allowed_tools, vec!["git".to_string(), "cat".to_string()]);
    }

    #[test]
    fn test_frontmatter_missing_description_fails() {
        let yaml = "kind: policy";
        let result: Result<Frontmatter, _> = serde_yaml_ng::from_str(yaml);

        assert!(result.is_err());
    }

    #[test]
    fn test_effective_targets_empty_returns_all() {
        let fm = Frontmatter::new("Test");
        let targets = fm.effective_targets();

        assert_eq!(targets.len(), 5);
        assert!(targets.contains(&Target::ClaudeCode));
        assert!(targets.contains(&Target::Cursor));
        assert!(targets.contains(&Target::VSCode));
        assert!(targets.contains(&Target::Antigravity));
        assert!(targets.contains(&Target::Codex));
    }

    #[test]
    fn test_effective_targets_all_expands() {
        let mut fm = Frontmatter::new("Test");
        fm.targets = vec![Target::All];
        let targets = fm.effective_targets();

        assert_eq!(targets.len(), 5);
    }

    #[test]
    fn test_effective_targets_specific() {
        let mut fm = Frontmatter::new("Test");
        fm.targets = vec![Target::ClaudeCode, Target::Cursor];
        let targets = fm.effective_targets();

        assert_eq!(targets.len(), 2);
        assert_eq!(targets, vec![Target::ClaudeCode, Target::Cursor]);
    }

    #[test]
    fn test_prompt_asset_construction() {
        let fm = Frontmatter::new("Test policy");
        let asset = PromptAsset::new(
            "security-policy",
            "policies/security.md",
            fm,
            "# Security Policy\n\nContent here",
        );

        assert_eq!(asset.id, "security-policy");
        assert_eq!(asset.source_path, PathBuf::from("policies/security.md"));
        assert_eq!(asset.frontmatter.description, "Test policy");
        assert!(asset.content.contains("Security Policy"));
    }

    #[test]
    fn test_target_serde_kebab_case() {
        // Deserialize kebab-case
        let yaml = "claude-code";
        let target: Target = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(target, Target::ClaudeCode);

        // vscode alias works
        let yaml = "vscode";
        let target: Target = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(target, Target::VSCode);
    }

    #[test]
    fn test_asset_kind_serde() {
        let yaml = "policy";
        let kind: AssetKind = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(kind, AssetKind::Policy);

        let yaml = "action";
        let kind: AssetKind = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(kind, AssetKind::Action);

        let yaml = "agent";
        let kind: AssetKind = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(kind, AssetKind::Agent);

        let yaml = "skill";
        let kind: AssetKind = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(kind, AssetKind::Skill);
    }

    #[test]
    fn test_scope_serde() {
        let yaml = "project";
        let scope: Scope = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(scope, Scope::Project);

        let yaml = "user";
        let scope: Scope = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(scope, Scope::User);
    }
}
