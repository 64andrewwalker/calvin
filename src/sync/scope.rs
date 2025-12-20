//! Scope Policy for sync operations
//!
//! **Migration Note**: `ScopePolicy` and `DeploymentTarget` are now defined in
//! `domain::policies` and re-exported here for backward compatibility.
//! The `apply` method on `ScopePolicy` is implemented here as an extension
//! for legacy code using `PromptAsset`.

use crate::models::PromptAsset;

// Re-export domain types for backward compatibility
pub use crate::domain::policies::{DeploymentTarget, ScopePolicy};

/// Extension trait for applying ScopePolicy to PromptAsset vectors
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
    use crate::models::{Frontmatter, Scope};

    fn make_asset(id: &str, scope: Scope) -> PromptAsset {
        let mut fm = Frontmatter::new(format!("asset {id}"));
        fm.scope = scope;
        PromptAsset::new(id, format!("{id}.md"), fm, "Content")
    }

    // Re-exported domain types work correctly
    #[test]
    fn from_target_maps_home_to_force_user() {
        assert_eq!(
            ScopePolicy::from_target(DeploymentTarget::Home),
            ScopePolicy::ForceUser
        );
    }

    // Extension trait tests for apply() on PromptAsset
    #[test]
    fn apply_keep_keeps_all_assets() {
        let assets = vec![
            make_asset("a", Scope::Project),
            make_asset("b", Scope::User),
        ];
        let out = ScopePolicy::Keep.apply(assets);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].frontmatter.scope, Scope::Project);
        assert_eq!(out[1].frontmatter.scope, Scope::User);
    }

    #[test]
    fn apply_project_only_filters_user_assets() {
        let assets = vec![
            make_asset("a", Scope::Project),
            make_asset("b", Scope::User),
        ];
        let out = ScopePolicy::ProjectOnly.apply(assets);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "a");
        assert_eq!(out[0].frontmatter.scope, Scope::Project);
    }

    #[test]
    fn apply_user_only_filters_project_assets() {
        let assets = vec![
            make_asset("a", Scope::Project),
            make_asset("b", Scope::User),
        ];
        let out = ScopePolicy::UserOnly.apply(assets);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "b");
        assert_eq!(out[0].frontmatter.scope, Scope::User);
    }

    #[test]
    fn apply_force_user_rewrites_scope() {
        let assets = vec![
            make_asset("a", Scope::Project),
            make_asset("b", Scope::User),
        ];
        let out = ScopePolicy::ForceUser.apply(assets);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|a| a.frontmatter.scope == Scope::User));
    }

    #[test]
    fn apply_force_project_rewrites_scope() {
        let assets = vec![
            make_asset("a", Scope::Project),
            make_asset("b", Scope::User),
        ];
        let out = ScopePolicy::ForceProject.apply(assets);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|a| a.frontmatter.scope == Scope::Project));
    }

    // --- Variants ---

    #[test]
    fn apply__empty_input_returns_empty() {
        let assets = vec![];
        let out = ScopePolicy::ForceUser.apply(assets);
        assert!(out.is_empty());
    }

    #[test]
    fn apply__project_only_on_user_only_input__returns_empty() {
        let assets = vec![make_asset("a", Scope::User), make_asset("b", Scope::User)];
        let out = ScopePolicy::ProjectOnly.apply(assets);
        assert!(out.is_empty());
    }
}
