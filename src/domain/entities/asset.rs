//! Asset entity - a source file from .promptpack/
//!
//! Assets are the "source code" of Calvin - markdown files with YAML frontmatter
//! that define policies, actions, and agents.

use crate::domain::value_objects::{Scope, Target};
use std::collections::HashMap;
use std::path::PathBuf;

/// Kind of prompt asset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

/// A prompt asset from .promptpack/
///
/// This is the core domain entity representing a source file.
#[derive(Debug, Clone, PartialEq)]
pub struct Asset {
    /// Unique identifier (derived from filename, kebab-case)
    id: String,
    /// Source file path relative to .promptpack/
    source_path: PathBuf,
    /// Description of the asset (from frontmatter)
    description: String,
    /// Kind of asset
    kind: AssetKind,
    /// Where to deploy (project or user)
    scope: Scope,
    /// Target platforms
    targets: Vec<Target>,
    /// Content body (after frontmatter)
    content: String,
    /// Optional apply glob pattern
    apply: Option<String>,

    /// Skill supplemental files (path relative to skill root → content)
    ///
    /// Empty for non-skill assets.
    supplementals: HashMap<PathBuf, String>,

    /// Skill binary supplemental files (path relative to skill root → binary content)
    ///
    /// For files like images, PDFs, etc. that cannot be stored as UTF-8 strings.
    /// Empty for non-skill assets.
    binary_supplementals: HashMap<PathBuf, Vec<u8>>,

    /// Skill allowed tools list (from `allowed-tools` frontmatter)
    ///
    /// Empty for non-skill assets.
    allowed_tools: Vec<String>,

    agent_name: Option<String>,
    agent_tools: Vec<String>,
    agent_model: Option<String>,
    agent_permission_mode: Option<String>,
    agent_skills: Vec<String>,

    /// Warnings generated during asset loading (e.g., skipped binary files)
    ///
    /// These are non-fatal issues that should be surfaced to the user.
    warnings: Vec<String>,
}

impl Asset {
    /// Create a new Asset
    ///
    /// # Arguments
    /// - `id` - Unique identifier (kebab-case)
    /// - `source_path` - Path relative to .promptpack/
    /// - `description` - Human-readable description
    /// - `content` - Content body (after frontmatter)
    pub fn new(
        id: impl Into<String>,
        source_path: impl Into<PathBuf>,
        description: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            source_path: source_path.into(),
            description: description.into(),
            kind: AssetKind::default(),
            scope: Scope::default(),
            targets: Vec::new(),
            content: content.into(),
            apply: None,
            supplementals: HashMap::new(),
            binary_supplementals: HashMap::new(),
            allowed_tools: Vec::new(),
            agent_name: None,
            agent_tools: Vec::new(),
            agent_model: None,
            agent_permission_mode: None,
            agent_skills: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Builder: set the kind
    pub fn with_kind(mut self, kind: AssetKind) -> Self {
        self.kind = kind;
        self
    }

    /// Builder: set the scope
    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    /// Builder: set the targets
    pub fn with_targets(mut self, targets: Vec<Target>) -> Self {
        self.targets = targets;
        self
    }

    /// Builder: set the apply pattern
    pub fn with_apply(mut self, apply: impl Into<String>) -> Self {
        self.apply = Some(apply.into());
        self
    }

    /// Builder: set supplemental files (skill-only)
    pub fn with_supplementals(mut self, supplementals: HashMap<PathBuf, String>) -> Self {
        self.supplementals = supplementals;
        self
    }

    /// Builder: set binary supplemental files (skill-only)
    pub fn with_binary_supplementals(
        mut self,
        binary_supplementals: HashMap<PathBuf, Vec<u8>>,
    ) -> Self {
        self.binary_supplementals = binary_supplementals;
        self
    }

    /// Builder: set allowed tools (skill-only)
    pub fn with_allowed_tools(mut self, allowed_tools: Vec<String>) -> Self {
        self.allowed_tools = allowed_tools;
        self
    }

    /// Builder: set warnings encountered during loading
    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings = warnings;
        self
    }

    pub fn with_agent_name(mut self, name: Option<String>) -> Self {
        self.agent_name = name;
        self
    }

    pub fn with_agent_tools(mut self, tools: Vec<String>) -> Self {
        self.agent_tools = tools;
        self
    }

    pub fn with_agent_model(mut self, model: Option<String>) -> Self {
        self.agent_model = model;
        self
    }

    pub fn with_agent_permission_mode(mut self, mode: Option<String>) -> Self {
        self.agent_permission_mode = mode;
        self
    }

    pub fn with_agent_skills(mut self, skills: Vec<String>) -> Self {
        self.agent_skills = skills;
        self
    }

    // --- Getters ---

    /// Get the asset ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the source path
    pub fn source_path(&self) -> &PathBuf {
        &self.source_path
    }

    /// Get the source path as a normalized string (always uses `/` separator)
    ///
    /// This is useful for generating consistent output across platforms,
    /// such as in footer comments like "Source: policies/test.md".
    pub fn source_path_normalized(&self) -> String {
        // Replace backslashes with forward slashes for cross-platform consistency
        self.source_path.display().to_string().replace('\\', "/")
    }

    /// Get the description
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the kind
    pub fn kind(&self) -> AssetKind {
        self.kind
    }

    /// Get the scope
    pub fn scope(&self) -> Scope {
        self.scope
    }

    /// Get the raw targets (may be empty)
    pub fn targets(&self) -> &[Target] {
        &self.targets
    }

    /// Get effective targets (expands empty/All to all platforms)
    pub fn effective_targets(&self) -> Vec<Target> {
        if self.targets.is_empty() || self.targets.iter().any(|t| t.is_all()) {
            Target::ALL_CONCRETE.to_vec()
        } else {
            self.targets.clone()
        }
    }

    /// Get the content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the apply pattern
    pub fn apply(&self) -> Option<&str> {
        self.apply.as_deref()
    }

    /// Get skill supplemental files
    pub fn supplementals(&self) -> &HashMap<PathBuf, String> {
        &self.supplementals
    }

    /// Get skill binary supplemental files
    pub fn binary_supplementals(&self) -> &HashMap<PathBuf, Vec<u8>> {
        &self.binary_supplementals
    }

    /// Get allowed tools list (skill-only)
    pub fn allowed_tools(&self) -> &[String] {
        &self.allowed_tools
    }

    /// Get warnings encountered during asset loading
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    pub fn agent_name(&self) -> Option<&str> {
        self.agent_name.as_deref()
    }

    pub fn agent_tools(&self) -> &[String] {
        &self.agent_tools
    }

    pub fn agent_model(&self) -> Option<&str> {
        self.agent_model.as_deref()
    }

    pub fn agent_permission_mode(&self) -> Option<&str> {
        self.agent_permission_mode.as_deref()
    }

    pub fn agent_skills(&self) -> &[String] {
        &self.agent_skills
    }
}

// === From implementations ===

impl From<crate::models::PromptAsset> for Asset {
    fn from(pa: crate::models::PromptAsset) -> Self {
        let kind = match pa.frontmatter.kind {
            crate::models::AssetKind::Policy => AssetKind::Policy,
            crate::models::AssetKind::Action => AssetKind::Action,
            crate::models::AssetKind::Agent => AssetKind::Agent,
            crate::models::AssetKind::Skill => AssetKind::Skill,
        };

        let scope = match pa.frontmatter.scope {
            crate::models::Scope::Project => Scope::Project,
            crate::models::Scope::User => Scope::User,
        };

        let targets: Vec<Target> = pa
            .frontmatter
            .targets
            .iter()
            .map(|t| match t {
                crate::models::Target::ClaudeCode => Target::ClaudeCode,
                crate::models::Target::Cursor => Target::Cursor,
                crate::models::Target::VSCode => Target::VSCode,
                crate::models::Target::Antigravity => Target::Antigravity,
                crate::models::Target::Codex => Target::Codex,
                crate::models::Target::All => Target::All,
            })
            .collect();

        let effective_tools = pa.frontmatter.effective_tools();
        let effective_skills = pa.frontmatter.effective_skills();
        let effective_perm = pa
            .frontmatter
            .effective_permission_mode()
            .map(|s| s.to_string());

        let mut asset = Asset::new(
            &pa.id,
            &pa.source_path,
            &pa.frontmatter.description,
            &pa.content,
        )
        .with_kind(kind)
        .with_scope(scope)
        .with_targets(targets);

        if let Some(apply) = pa.frontmatter.apply {
            asset = asset.with_apply(apply);
        }

        if !pa.frontmatter.allowed_tools.is_empty() {
            asset = asset.with_allowed_tools(pa.frontmatter.allowed_tools);
        }

        asset = asset.with_agent_name(pa.frontmatter.name);

        if !effective_tools.is_empty() {
            asset = asset.with_agent_tools(effective_tools);
        }
        if pa.frontmatter.model.is_some() {
            asset = asset.with_agent_model(pa.frontmatter.model);
        }
        if effective_perm.is_some() {
            asset = asset.with_agent_permission_mode(effective_perm);
        }
        if !effective_skills.is_empty() {
            asset = asset.with_agent_skills(effective_skills);
        }

        asset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === TDD: Asset Creation ===

    #[test]
    fn asset_new_creates_with_defaults() {
        let asset = Asset::new(
            "test-policy",
            "policies/test.md",
            "A test policy",
            "Content here",
        );

        assert_eq!(asset.id(), "test-policy");
        assert_eq!(asset.source_path(), &PathBuf::from("policies/test.md"));
        assert_eq!(asset.description(), "A test policy");
        assert_eq!(asset.kind(), AssetKind::Action); // default
        assert_eq!(asset.scope(), Scope::Project); // default
        assert!(asset.targets().is_empty());
        assert_eq!(asset.content(), "Content here");
        assert!(asset.apply().is_none());
        assert!(asset.supplementals().is_empty());
        assert!(asset.allowed_tools().is_empty());
    }

    #[test]
    fn asset_builder_sets_kind() {
        let asset = Asset::new("test", "test.md", "desc", "content").with_kind(AssetKind::Policy);

        assert_eq!(asset.kind(), AssetKind::Policy);
    }

    #[test]
    fn asset_builder_sets_scope() {
        let asset = Asset::new("test", "test.md", "desc", "content").with_scope(Scope::User);

        assert_eq!(asset.scope(), Scope::User);
    }

    #[test]
    fn asset_builder_sets_targets() {
        let asset = Asset::new("test", "test.md", "desc", "content")
            .with_targets(vec![Target::ClaudeCode, Target::Cursor]);

        assert_eq!(asset.targets().len(), 2);
    }

    #[test]
    fn asset_builder_sets_apply() {
        let asset = Asset::new("test", "test.md", "desc", "content").with_apply("*.rs");

        assert_eq!(asset.apply(), Some("*.rs"));
    }

    #[test]
    fn asset_builder_chain() {
        let asset = Asset::new(
            "security",
            "policies/security.md",
            "Security rules",
            "# Rules",
        )
        .with_kind(AssetKind::Policy)
        .with_scope(Scope::Project)
        .with_targets(vec![Target::ClaudeCode])
        .with_apply("*.rs");

        assert_eq!(asset.id(), "security");
        assert_eq!(asset.kind(), AssetKind::Policy);
        assert_eq!(asset.scope(), Scope::Project);
        assert_eq!(asset.targets(), &[Target::ClaudeCode]);
        assert_eq!(asset.apply(), Some("*.rs"));
    }

    // === TDD: Skills fields ===

    #[test]
    fn test_asset_kind_skill_exists() {
        assert_ne!(AssetKind::Skill, AssetKind::Action);
    }

    #[test]
    fn test_asset_supplementals_empty_by_default() {
        let asset = Asset::new("test", "test.md", "desc", "content");
        assert!(asset.supplementals().is_empty());
    }

    #[test]
    fn test_asset_with_supplementals_builder() {
        let mut supplementals = HashMap::new();
        supplementals.insert(PathBuf::from("reference.md"), "# Ref".to_string());
        supplementals.insert(
            PathBuf::from("scripts/validate.py"),
            "print('ok')".to_string(),
        );

        let asset = Asset::new("my-skill", "skills/my-skill/SKILL.md", "desc", "content")
            .with_kind(AssetKind::Skill)
            .with_supplementals(supplementals.clone());

        assert_eq!(asset.supplementals(), &supplementals);
    }

    #[test]
    fn test_asset_allowed_tools_empty_by_default() {
        let asset = Asset::new("test", "test.md", "desc", "content");
        assert!(asset.allowed_tools().is_empty());
    }

    #[test]
    fn test_asset_warnings_empty_by_default() {
        let asset = Asset::new("test", "test.md", "desc", "content");
        assert!(asset.warnings().is_empty());
    }

    #[test]
    fn test_asset_with_warnings_builder() {
        let warnings = vec![
            "Skipped binary file: image.png".to_string(),
            "Skipped binary file: data.bin".to_string(),
        ];

        let asset = Asset::new("my-skill", "skills/my-skill/SKILL.md", "desc", "content")
            .with_warnings(warnings.clone());

        assert_eq!(asset.warnings(), &warnings);
    }

    // === TDD: Effective Targets ===

    #[test]
    fn asset_effective_targets_empty_returns_all() {
        let asset = Asset::new("test", "test.md", "desc", "content");

        let targets = asset.effective_targets();
        assert_eq!(targets.len(), 5);
    }

    #[test]
    fn asset_effective_targets_with_all_expands() {
        let asset =
            Asset::new("test", "test.md", "desc", "content").with_targets(vec![Target::All]);

        let targets = asset.effective_targets();
        assert_eq!(targets.len(), 5);
    }

    #[test]
    fn asset_effective_targets_specific_unchanged() {
        let asset = Asset::new("test", "test.md", "desc", "content")
            .with_targets(vec![Target::Cursor, Target::VSCode]);

        let targets = asset.effective_targets();
        assert_eq!(targets.len(), 2);
        assert_eq!(targets, vec![Target::Cursor, Target::VSCode]);
    }

    // === TDD: AssetKind ===

    #[test]
    fn asset_kind_default_is_action() {
        assert_eq!(AssetKind::default(), AssetKind::Action);
    }

    #[test]
    fn asset_kind_equality() {
        assert_eq!(AssetKind::Policy, AssetKind::Policy);
        assert_ne!(AssetKind::Policy, AssetKind::Action);
    }

    // === TDD: From<PromptAsset> ===

    #[test]
    fn asset_from_prompt_asset() {
        use crate::models::{
            AssetKind as ModelKind, Frontmatter, PromptAsset, Scope as ModelScope,
        };

        let frontmatter = Frontmatter {
            description: "Test description".to_string(),
            name: None,
            kind: ModelKind::Policy,
            scope: ModelScope::User,
            targets: vec![crate::models::Target::Cursor],
            apply: Some("*.rs".to_string()),
            allowed_tools: vec![],
            tools: None,
            agent_tools: vec![],
            model: None,
            permission_mode_camel: None,
            permission_mode: None,
            skills: None,
            agent_skills: vec![],
        };
        let prompt_asset = PromptAsset::new("test-id", "test.md", frontmatter, "Test content");

        let asset = Asset::from(prompt_asset);

        assert_eq!(asset.id(), "test-id");
        assert_eq!(asset.description(), "Test description");
        assert_eq!(asset.kind(), AssetKind::Policy);
        assert_eq!(asset.scope(), Scope::User);
        assert_eq!(asset.targets(), &[Target::Cursor]);
        assert_eq!(asset.apply(), Some("*.rs"));
        assert_eq!(asset.content(), "Test content");
    }

    #[test]
    fn asset_from_prompt_asset_with_agent_fields() {
        use crate::models::{
            AssetKind as ModelKind, Frontmatter, PromptAsset, Scope as ModelScope,
        };

        let frontmatter = Frontmatter {
            description: "Agent description".to_string(),
            name: Some("test-agent".to_string()),
            kind: ModelKind::Agent,
            scope: ModelScope::Project,
            targets: vec![crate::models::Target::ClaudeCode],
            apply: None,
            allowed_tools: vec![],
            tools: None,
            agent_tools: vec!["Read".to_string(), "Grep".to_string()],
            model: Some("sonnet".to_string()),
            permission_mode_camel: Some("acceptEdits".to_string()),
            permission_mode: None,
            skills: None,
            agent_skills: vec!["skill-a".to_string()],
        };
        let prompt_asset =
            PromptAsset::new("test-agent", "agents/test.md", frontmatter, "Agent content");

        let asset = Asset::from(prompt_asset);

        assert_eq!(asset.id(), "test-agent");
        assert_eq!(asset.kind(), AssetKind::Agent);
        assert_eq!(asset.agent_tools(), &["Read", "Grep"]);
        assert_eq!(asset.agent_model(), Some("sonnet"));
        assert_eq!(asset.agent_permission_mode(), Some("acceptEdits"));
        assert_eq!(asset.agent_skills(), &["skill-a"]);
    }
}
