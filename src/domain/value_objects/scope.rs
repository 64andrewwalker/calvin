//! Scope value object - defines where assets are deployed
//!
//! - `Project` scope: deployed to the project directory
//! - `User` scope: deployed to user's home directory

use serde::{Deserialize, Serialize};

/// Scope of an asset (where it should be installed)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// Project-level (in repository directories)
    #[default]
    Project,
    /// User-level (in home directory)
    User,
}

impl Scope {
    /// Returns true if this is a user-level scope
    pub fn is_user(&self) -> bool {
        matches!(self, Scope::User)
    }

    /// Returns true if this is a project-level scope
    pub fn is_project(&self) -> bool {
        matches!(self, Scope::Project)
    }

    /// Get the namespace prefix for lockfile keys
    pub fn namespace_prefix(&self) -> &'static str {
        match self {
            Scope::Project => "project:",
            Scope::User => "home:",
        }
    }
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scope::Project => write!(f, "project"),
            Scope::User => write!(f, "user"),
        }
    }
}

// Conversion from legacy models::Scope
impl From<crate::models::Scope> for Scope {
    fn from(scope: crate::models::Scope) -> Self {
        match scope {
            crate::models::Scope::Project => Scope::Project,
            crate::models::Scope::User => Scope::User,
        }
    }
}

// Conversion to legacy models::Scope
impl From<Scope> for crate::models::Scope {
    fn from(scope: Scope) -> Self {
        match scope {
            Scope::Project => crate::models::Scope::Project,
            Scope::User => crate::models::Scope::User,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_default_is_project() {
        assert_eq!(Scope::default(), Scope::Project);
    }

    #[test]
    fn scope_is_user() {
        assert!(Scope::User.is_user());
        assert!(!Scope::Project.is_user());
    }

    #[test]
    fn scope_is_project() {
        assert!(Scope::Project.is_project());
        assert!(!Scope::User.is_project());
    }

    #[test]
    fn scope_namespace_prefix() {
        assert_eq!(Scope::Project.namespace_prefix(), "project:");
        assert_eq!(Scope::User.namespace_prefix(), "home:");
    }

    #[test]
    fn scope_display() {
        assert_eq!(format!("{}", Scope::Project), "project");
        assert_eq!(format!("{}", Scope::User), "user");
    }

    #[test]
    fn scope_serde_roundtrip() {
        let scope = Scope::User;
        let json = serde_json::to_string(&scope).unwrap();
        let parsed: Scope = serde_json::from_str(&json).unwrap();
        assert_eq!(scope, parsed);
    }
}
