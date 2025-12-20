//! Deploy target types and configuration

use std::path::PathBuf;

/// Deployment target
#[derive(Debug, Clone)]
pub enum DeployTarget {
    /// Deploy to project directory
    Project(PathBuf),
    /// Deploy to user home directory
    Home,
    /// Deploy to remote server via SSH
    Remote(String),
}

impl DeployTarget {
    /// Get destination path for header display
    pub fn destination_display(&self) -> Option<String> {
        match self {
            DeployTarget::Project(p) => Some(p.display().to_string()),
            DeployTarget::Home => Some("~/".to_string()),
            DeployTarget::Remote(r) => Some(r.clone()),
        }
    }

    /// Check if this is a local target
    pub fn is_local(&self) -> bool {
        !matches!(self, DeployTarget::Remote(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_destination_display() {
        let target = DeployTarget::Project(PathBuf::from("/project"));
        assert_eq!(target.destination_display(), Some("/project".to_string()));
    }

    #[test]
    fn home_destination_display() {
        let target = DeployTarget::Home;
        assert_eq!(target.destination_display(), Some("~/".to_string()));
    }

    #[test]
    fn remote_destination_display() {
        let target = DeployTarget::Remote("user@host:/path".to_string());
        assert_eq!(
            target.destination_display(),
            Some("user@host:/path".to_string())
        );
    }

    #[test]
    fn is_local() {
        assert!(DeployTarget::Project(PathBuf::from("/")).is_local());
        assert!(DeployTarget::Home.is_local());
        assert!(!DeployTarget::Remote("host".to_string()).is_local());
    }
}
