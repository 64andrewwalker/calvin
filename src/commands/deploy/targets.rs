//! Deploy target types and configuration

use calvin::sync::SyncDestination;
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

    /// Check if this is a local target
    pub fn is_local(&self) -> bool {
        !matches!(self, DeployTarget::Remote(_))
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
        assert!(
            matches!(dest, SyncDestination::Remote { host, path } if host == "user@host" && path == PathBuf::from("/path"))
        );
    }

    #[test]
    fn remote_without_path() {
        let target = DeployTarget::Remote("server".to_string());
        let dest = target.to_sync_destination();
        assert!(
            matches!(dest, SyncDestination::Remote { host, path } if host == "server" && path == PathBuf::from("."))
        );
    }
}
