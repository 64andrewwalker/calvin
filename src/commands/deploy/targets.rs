//! Deploy target types and configuration

use std::path::PathBuf;
use calvin::sync::SyncDestination;

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
    /// Get target name for display
    #[allow(dead_code)]
    pub fn display_name(&self) -> &'static str {
        match self {
            DeployTarget::Project(_) => "project",
            DeployTarget::Home => "home",
            DeployTarget::Remote(_) => "remote",
        }
    }

    /// Get destination path for header display
    pub fn destination_display(&self) -> Option<String> {
        match self {
            DeployTarget::Project(p) => Some(p.display().to_string()),
            DeployTarget::Home => Some("~/".to_string()),
            DeployTarget::Remote(r) => Some(r.clone()),
        }
    }

    /// Convert to SyncDestination for two-stage sync
    pub fn to_sync_destination(&self) -> SyncDestination {
        match self {
            DeployTarget::Project(root) => SyncDestination::Local(root.clone()),
            DeployTarget::Home => {
                let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
                SyncDestination::Local(home)
            }
            DeployTarget::Remote(remote) => {
                let (host, path) = if let Some((h, p)) = remote.split_once(':') {
                    (h.to_string(), PathBuf::from(p))
                } else {
                    (remote.clone(), PathBuf::from("."))
                };
                SyncDestination::Remote { host, path }
            }
        }
    }

    /// Get remote string if this is a remote target
    #[allow(dead_code)]
    pub fn remote_str(&self) -> Option<&str> {
        match self {
            DeployTarget::Remote(r) => Some(r.as_str()),
            _ => None,
        }
    }

    /// Check if this is a local target
    pub fn is_local(&self) -> bool {
        !matches!(self, DeployTarget::Remote(_))
    }
}

/// Scope policy for asset deployment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ScopePolicy {
    /// Keep original scope from asset definition
    Keep,
    /// Only deploy user-scoped assets
    UserOnly,
    /// Force all assets to user scope
    ForceUser,
}

impl Default for ScopePolicy {
    fn default() -> Self {
        ScopePolicy::Keep
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_to_sync_destination() {
        let target = DeployTarget::Project(PathBuf::from("/project"));
        let dest = target.to_sync_destination();
        assert!(matches!(dest, SyncDestination::Local(p) if p == PathBuf::from("/project")));
    }

    #[test]
    fn remote_to_sync_destination() {
        let target = DeployTarget::Remote("user@host:/path".to_string());
        let dest = target.to_sync_destination();
        assert!(matches!(dest, SyncDestination::Remote { host, path } 
            if host == "user@host" && path == PathBuf::from("/path")));
    }

    #[test]
    fn remote_without_path() {
        let target = DeployTarget::Remote("server".to_string());
        let dest = target.to_sync_destination();
        assert!(matches!(dest, SyncDestination::Remote { host, path } 
            if host == "server" && path == PathBuf::from(".")));
    }
}
