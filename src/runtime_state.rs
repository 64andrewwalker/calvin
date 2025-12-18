use std::path::Path;
use std::fs;

use serde::{Deserialize, Serialize};

/// Runtime state persisted between runs
/// Stored in .promptpack/.calvin-state.json
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeState {
    /// Last deploy target (project, home, or remote)
    #[serde(default)]
    pub last_deploy_target: DeployTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DeployTarget {
    #[default]
    Project,
    Home,
    Remote,
}

const STATE_FILE: &str = ".calvin-state.json";

impl RuntimeState {
    /// Load state from .promptpack/.calvin-state.json
    pub fn load(promptpack_dir: &Path) -> Self {
        let state_file = promptpack_dir.join(STATE_FILE);
        if state_file.exists() {
            if let Ok(content) = fs::read_to_string(&state_file) {
                if let Ok(state) = serde_json::from_str(&content) {
                    return state;
                }
            }
        }
        Self::default()
    }

    /// Save state to .promptpack/.calvin-state.json
    pub fn save(&self, promptpack_dir: &Path) -> std::io::Result<()> {
        let state_file = promptpack_dir.join(STATE_FILE);
        let content = serde_json::to_string_pretty(self)?;
        fs::write(state_file, content)
    }

    /// Update last deploy target and save
    pub fn set_deploy_target(&mut self, target: DeployTarget, promptpack_dir: &Path) {
        self.last_deploy_target = target;
        let _ = self.save(promptpack_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_state_default() {
        let state = RuntimeState::default();
        assert_eq!(state.last_deploy_target, DeployTarget::Project);
    }

    #[test]
    fn test_state_save_and_load() {
        let dir = tempdir().unwrap();
        let mut state = RuntimeState::default();
        state.last_deploy_target = DeployTarget::Home;
        state.save(dir.path()).unwrap();

        let loaded = RuntimeState::load(dir.path());
        assert_eq!(loaded.last_deploy_target, DeployTarget::Home);
    }

    #[test]
    fn test_state_load_missing_file() {
        let dir = tempdir().unwrap();
        let state = RuntimeState::load(dir.path());
        assert_eq!(state.last_deploy_target, DeployTarget::Project);
    }
}
