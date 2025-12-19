//! Config repository port - abstracts configuration loading and saving.

use anyhow::Result;
use std::path::Path;

use crate::config::{Config, ConfigWarning, DeployTargetConfig};

/// Repository trait for configuration management.
///
/// Abstracts the underlying storage mechanism (TOML files, environment variables, etc.)
/// to allow for dependency injection and testing.
pub trait ConfigRepository: Send + Sync {
    /// Load configuration from a specific path.
    fn load(&self, path: &Path) -> Result<Config>;

    /// Load configuration with warnings about unknown keys.
    fn load_with_warnings(&self, path: &Path) -> Result<(Config, Vec<ConfigWarning>)>;

    /// Load configuration using the standard hierarchy:
    /// 1. Project config (.promptpack/config.toml)
    /// 2. User config (~/.config/calvin/config.toml)
    /// 3. Built-in defaults
    ///
    /// Then applies environment variable overrides.
    fn load_or_default(&self, project_root: Option<&Path>) -> Config;

    /// Save the deploy target to the config file.
    ///
    /// Updates or creates the [deploy] section while preserving other settings.
    fn save_deploy_target(&self, config_path: &Path, target: DeployTargetConfig) -> Result<()>;

    /// Check if a config file exists at the given path.
    fn exists(&self, path: &Path) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Mock implementation for testing
    struct MockConfigRepository {
        configs: Mutex<HashMap<String, Config>>,
        default_config: Config,
    }

    impl MockConfigRepository {
        fn new() -> Self {
            Self {
                configs: Mutex::new(HashMap::new()),
                default_config: Config::default(),
            }
        }

        fn with_config(mut self, path: &str, config: Config) -> Self {
            self.configs
                .lock()
                .unwrap()
                .insert(path.to_string(), config);
            self
        }
    }

    impl ConfigRepository for MockConfigRepository {
        fn load(&self, path: &Path) -> Result<Config> {
            let configs = self.configs.lock().unwrap();
            configs
                .get(path.to_string_lossy().as_ref())
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Config not found"))
        }

        fn load_with_warnings(&self, path: &Path) -> Result<(Config, Vec<ConfigWarning>)> {
            let config = self.load(path)?;
            Ok((config, vec![]))
        }

        fn load_or_default(&self, project_root: Option<&Path>) -> Config {
            if let Some(root) = project_root {
                let project_config_path = root.join(".promptpack/config.toml");
                if let Ok(config) = self.load(&project_config_path) {
                    return config;
                }
            }
            self.default_config.clone()
        }

        fn save_deploy_target(
            &self,
            _config_path: &Path,
            _target: DeployTargetConfig,
        ) -> Result<()> {
            Ok(()) // Mock does nothing
        }

        fn exists(&self, path: &Path) -> bool {
            self.configs
                .lock()
                .unwrap()
                .contains_key(path.to_string_lossy().as_ref())
        }
    }

    #[test]
    fn mock_repository_returns_default() {
        let repo = MockConfigRepository::new();
        let config = repo.load_or_default(None);
        assert_eq!(config.format.version, "1.0");
    }

    #[test]
    fn mock_repository_returns_configured() {
        use crate::config::SecurityMode;

        let mut custom = Config::default();
        custom.security.mode = SecurityMode::Strict;

        let repo =
            MockConfigRepository::new().with_config("/project/.promptpack/config.toml", custom);

        let config = repo.load_or_default(Some(Path::new("/project")));
        assert_eq!(config.security.mode, SecurityMode::Strict);
    }

    #[test]
    fn mock_repository_load_fails_for_missing() {
        let repo = MockConfigRepository::new();
        let result = repo.load(Path::new("/nonexistent/config.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn mock_repository_exists_check() {
        let repo =
            MockConfigRepository::new().with_config("/project/config.toml", Config::default());

        assert!(repo.exists(Path::new("/project/config.toml")));
        assert!(!repo.exists(Path::new("/other/config.toml")));
    }
}
