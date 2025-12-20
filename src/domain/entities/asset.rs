//! Asset entity - a source file from .promptpack/
//!
//! Assets are the "source code" of Calvin - markdown files with YAML frontmatter
//! that define policies, actions, and agents.

use crate::domain::value_objects::{Scope, Target};
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

    // --- Getters ---

    /// Get the asset ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the source path
    pub fn source_path(&self) -> &PathBuf {
        &self.source_path
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
            kind: ModelKind::Policy,
            scope: ModelScope::User,
            targets: vec![crate::models::Target::Cursor],
            apply: Some("*.rs".to_string()),
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
}

// === From implementations ===

impl From<crate::models::PromptAsset> for Asset {
    fn from(pa: crate::models::PromptAsset) -> Self {
        // Convert AssetKind
        let kind = match pa.frontmatter.kind {
            crate::models::AssetKind::Policy => AssetKind::Policy,
            crate::models::AssetKind::Action => AssetKind::Action,
            crate::models::AssetKind::Agent => AssetKind::Agent,
        };

        // Convert Scope
        let scope = match pa.frontmatter.scope {
            crate::models::Scope::Project => Scope::Project,
            crate::models::Scope::User => Scope::User,
        };

        // Convert Targets
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

        asset
    }
}
