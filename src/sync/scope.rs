use crate::models::{PromptAsset, Scope};

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

/// Policy for handling asset scope during compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopePolicy {
    /// Keep the original scope from the asset frontmatter.
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

impl Default for ScopePolicy {
    fn default() -> Self {
        Self::Keep
    }
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

    /// Apply the policy to a list of assets.
    pub fn apply(&self, assets: Vec<PromptAsset>) -> Vec<PromptAsset> {
        match self {
            ScopePolicy::Keep => assets,
            ScopePolicy::ProjectOnly => assets
                .into_iter()
                .filter(|a| a.frontmatter.scope == Scope::Project)
                .collect(),
            ScopePolicy::UserOnly => assets
                .into_iter()
                .filter(|a| a.frontmatter.scope == Scope::User)
                .collect(),
            ScopePolicy::ForceUser => assets
                .into_iter()
                .map(|mut a| {
                    a.frontmatter.scope = Scope::User;
                    a
                })
                .collect(),
            ScopePolicy::ForceProject => assets
                .into_iter()
                .map(|mut a| {
                    a.frontmatter.scope = Scope::Project;
                    a
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Frontmatter;

    fn make_asset(id: &str, scope: Scope) -> PromptAsset {
        let mut fm = Frontmatter::new(format!("asset {id}"));
        fm.scope = scope;
        PromptAsset::new(id, format!("{id}.md"), fm, "Content")
    }

    #[test]
    fn from_target_maps_home_to_force_user() {
        assert_eq!(ScopePolicy::from_target(DeploymentTarget::Home), ScopePolicy::ForceUser);
    }

    #[test]
    fn apply_keep_keeps_all_assets() {
        let assets = vec![make_asset("a", Scope::Project), make_asset("b", Scope::User)];
        let out = ScopePolicy::Keep.apply(assets);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].frontmatter.scope, Scope::Project);
        assert_eq!(out[1].frontmatter.scope, Scope::User);
    }

    #[test]
    fn apply_project_only_filters_user_assets() {
        let assets = vec![make_asset("a", Scope::Project), make_asset("b", Scope::User)];
        let out = ScopePolicy::ProjectOnly.apply(assets);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "a");
        assert_eq!(out[0].frontmatter.scope, Scope::Project);
    }

    #[test]
    fn apply_user_only_filters_project_assets() {
        let assets = vec![make_asset("a", Scope::Project), make_asset("b", Scope::User)];
        let out = ScopePolicy::UserOnly.apply(assets);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "b");
        assert_eq!(out[0].frontmatter.scope, Scope::User);
    }

    #[test]
    fn apply_force_user_rewrites_scope() {
        let assets = vec![make_asset("a", Scope::Project), make_asset("b", Scope::User)];
        let out = ScopePolicy::ForceUser.apply(assets);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|a| a.frontmatter.scope == Scope::User));
    }

    #[test]
    fn apply_force_project_rewrites_scope() {
        let assets = vec![make_asset("a", Scope::Project), make_asset("b", Scope::User)];
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

