//! Scope Policy
//!
//! Determines how asset scope is handled during deployment.
//! This is a pure policy - it operates on generic types via traits.

use crate::domain::value_objects::Scope;
use crate::models::PromptAsset;

/// Deployment target type (where outputs should be written).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentTarget {
    /// Project directory.
    Project,
    /// User home directory.
    Home,
    /// Deploy to both (based on asset scope).
    Both,
}

impl DeploymentTarget {
    /// Check if this target deploys to project
    pub fn includes_project(&self) -> bool {
        matches!(self, Self::Project | Self::Both)
    }

    /// Check if this target deploys to home
    pub fn includes_home(&self) -> bool {
        matches!(self, Self::Home | Self::Both)
    }
}

/// Policy for handling asset scope during compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScopePolicy {
    /// Keep the original scope from the asset frontmatter.
    #[default]
    Keep,
    /// Keep only `scope: project` assets.
    ProjectOnly,
    /// Keep only `scope: user` assets.
    UserOnly,
    /// Force all assets to `scope: user`.
    ForceUser,
    /// Force all assets to `scope: project`.
    ForceProject,
}

impl ScopePolicy {
    /// Determine scope policy from a deployment target.
    pub fn from_target(target: DeploymentTarget) -> Self {
        match target {
            DeploymentTarget::Project => ScopePolicy::Keep,
            DeploymentTarget::Home => ScopePolicy::ForceUser,
            DeploymentTarget::Both => ScopePolicy::Keep,
        }
    }

    /// Check if an asset with the given scope should be included
    pub fn should_include(&self, scope: Scope) -> bool {
        match self {
            ScopePolicy::Keep => true,
            ScopePolicy::ProjectOnly => scope == Scope::Project,
            ScopePolicy::UserOnly => scope == Scope::User,
            ScopePolicy::ForceUser => true,
            ScopePolicy::ForceProject => true,
        }
    }

    /// Transform scope according to policy
    pub fn transform_scope(&self, scope: Scope) -> Scope {
        match self {
            ScopePolicy::Keep => scope,
            ScopePolicy::ProjectOnly => scope,
            ScopePolicy::UserOnly => scope,
            ScopePolicy::ForceUser => Scope::User,
            ScopePolicy::ForceProject => Scope::Project,
        }
    }

    /// Is this a filtering policy (may reduce assets)?
    pub fn is_filter(&self) -> bool {
        matches!(self, ScopePolicy::ProjectOnly | ScopePolicy::UserOnly)
    }

    /// Is this a transform policy (changes scope)?
    pub fn is_transform(&self) -> bool {
        matches!(self, ScopePolicy::ForceUser | ScopePolicy::ForceProject)
    }
}

/// Extension trait for applying ScopePolicy to PromptAsset vectors
///
/// This trait bridges the domain policy with the legacy PromptAsset model.
/// It provides the `apply` method that filters and transforms assets based on scope.
pub trait ScopePolicyExt {
    /// Apply the policy to a list of assets.
    fn apply(&self, assets: Vec<PromptAsset>) -> Vec<PromptAsset>;
}

impl ScopePolicyExt for ScopePolicy {
    fn apply(&self, assets: Vec<PromptAsset>) -> Vec<PromptAsset> {
        assets
            .into_iter()
            .filter(|a| self.should_include(a.frontmatter.scope.into()))
            .map(|mut a| {
                a.frontmatter.scope = self.transform_scope(a.frontmatter.scope.into()).into();
                a
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === TDD: DeploymentTarget ===

    #[test]
    fn deployment_target_includes_project() {
        assert!(DeploymentTarget::Project.includes_project());
        assert!(!DeploymentTarget::Home.includes_project());
        assert!(DeploymentTarget::Both.includes_project());
    }

    #[test]
    fn deployment_target_includes_home() {
        assert!(!DeploymentTarget::Project.includes_home());
        assert!(DeploymentTarget::Home.includes_home());
        assert!(DeploymentTarget::Both.includes_home());
    }

    // === TDD: ScopePolicy ===

    #[test]
    fn scope_policy_default_is_keep() {
        assert_eq!(ScopePolicy::default(), ScopePolicy::Keep);
    }

    #[test]
    fn scope_policy_from_target() {
        assert_eq!(
            ScopePolicy::from_target(DeploymentTarget::Project),
            ScopePolicy::Keep
        );
        assert_eq!(
            ScopePolicy::from_target(DeploymentTarget::Home),
            ScopePolicy::ForceUser
        );
        assert_eq!(
            ScopePolicy::from_target(DeploymentTarget::Both),
            ScopePolicy::Keep
        );
    }

    #[test]
    fn scope_policy_should_include_keep() {
        assert!(ScopePolicy::Keep.should_include(Scope::Project));
        assert!(ScopePolicy::Keep.should_include(Scope::User));
    }

    #[test]
    fn scope_policy_should_include_project_only() {
        assert!(ScopePolicy::ProjectOnly.should_include(Scope::Project));
        assert!(!ScopePolicy::ProjectOnly.should_include(Scope::User));
    }

    #[test]
    fn scope_policy_should_include_user_only() {
        assert!(!ScopePolicy::UserOnly.should_include(Scope::Project));
        assert!(ScopePolicy::UserOnly.should_include(Scope::User));
    }

    #[test]
    fn scope_policy_should_include_force_user() {
        // Force policies include everything (then transform)
        assert!(ScopePolicy::ForceUser.should_include(Scope::Project));
        assert!(ScopePolicy::ForceUser.should_include(Scope::User));
    }

    #[test]
    fn scope_policy_should_include_force_project() {
        assert!(ScopePolicy::ForceProject.should_include(Scope::Project));
        assert!(ScopePolicy::ForceProject.should_include(Scope::User));
    }

    #[test]
    fn scope_policy_transform_keep() {
        assert_eq!(
            ScopePolicy::Keep.transform_scope(Scope::Project),
            Scope::Project
        );
        assert_eq!(ScopePolicy::Keep.transform_scope(Scope::User), Scope::User);
    }

    #[test]
    fn scope_policy_transform_force_user() {
        assert_eq!(
            ScopePolicy::ForceUser.transform_scope(Scope::Project),
            Scope::User
        );
        assert_eq!(
            ScopePolicy::ForceUser.transform_scope(Scope::User),
            Scope::User
        );
    }

    #[test]
    fn scope_policy_transform_force_project() {
        assert_eq!(
            ScopePolicy::ForceProject.transform_scope(Scope::Project),
            Scope::Project
        );
        assert_eq!(
            ScopePolicy::ForceProject.transform_scope(Scope::User),
            Scope::Project
        );
    }

    #[test]
    fn scope_policy_is_filter() {
        assert!(!ScopePolicy::Keep.is_filter());
        assert!(ScopePolicy::ProjectOnly.is_filter());
        assert!(ScopePolicy::UserOnly.is_filter());
        assert!(!ScopePolicy::ForceUser.is_filter());
        assert!(!ScopePolicy::ForceProject.is_filter());
    }

    #[test]
    fn scope_policy_is_transform() {
        assert!(!ScopePolicy::Keep.is_transform());
        assert!(!ScopePolicy::ProjectOnly.is_transform());
        assert!(!ScopePolicy::UserOnly.is_transform());
        assert!(ScopePolicy::ForceUser.is_transform());
        assert!(ScopePolicy::ForceProject.is_transform());
    }

    // === TDD: ScopePolicyExt trait ===

    use crate::models::Frontmatter;

    fn make_asset(id: &str, scope: crate::models::Scope) -> PromptAsset {
        let mut fm = Frontmatter::new(format!("asset {id}"));
        fm.scope = scope;
        PromptAsset::new(id, format!("{id}.md"), fm, "Content")
    }

    #[test]
    fn apply_keep_keeps_all_assets() {
        use crate::models::Scope as ModelScope;
        let assets = vec![
            make_asset("a", ModelScope::Project),
            make_asset("b", ModelScope::User),
        ];
        let out = ScopePolicy::Keep.apply(assets);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].frontmatter.scope, ModelScope::Project);
        assert_eq!(out[1].frontmatter.scope, ModelScope::User);
    }

    #[test]
    fn apply_project_only_filters_user_assets() {
        use crate::models::Scope as ModelScope;
        let assets = vec![
            make_asset("a", ModelScope::Project),
            make_asset("b", ModelScope::User),
        ];
        let out = ScopePolicy::ProjectOnly.apply(assets);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "a");
        assert_eq!(out[0].frontmatter.scope, ModelScope::Project);
    }

    #[test]
    fn apply_user_only_filters_project_assets() {
        use crate::models::Scope as ModelScope;
        let assets = vec![
            make_asset("a", ModelScope::Project),
            make_asset("b", ModelScope::User),
        ];
        let out = ScopePolicy::UserOnly.apply(assets);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "b");
        assert_eq!(out[0].frontmatter.scope, ModelScope::User);
    }

    #[test]
    fn apply_force_user_rewrites_scope() {
        use crate::models::Scope as ModelScope;
        let assets = vec![
            make_asset("a", ModelScope::Project),
            make_asset("b", ModelScope::User),
        ];
        let out = ScopePolicy::ForceUser.apply(assets);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|a| a.frontmatter.scope == ModelScope::User));
    }

    #[test]
    fn apply_force_project_rewrites_scope() {
        use crate::models::Scope as ModelScope;
        let assets = vec![
            make_asset("a", ModelScope::Project),
            make_asset("b", ModelScope::User),
        ];
        let out = ScopePolicy::ForceProject.apply(assets);
        assert_eq!(out.len(), 2);
        assert!(out
            .iter()
            .all(|a| a.frontmatter.scope == ModelScope::Project));
    }

    #[test]
    fn apply_empty_input_returns_empty() {
        let assets = vec![];
        let out = ScopePolicy::ForceUser.apply(assets);
        assert!(out.is_empty());
    }

    #[test]
    fn apply_project_only_on_user_only_input_returns_empty() {
        use crate::models::Scope as ModelScope;
        let assets = vec![
            make_asset("a", ModelScope::User),
            make_asset("b", ModelScope::User),
        ];
        let out = ScopePolicy::ProjectOnly.apply(assets);
        assert!(out.is_empty());
    }
}
