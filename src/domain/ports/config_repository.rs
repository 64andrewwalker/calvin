//! Config repository port - abstracts configuration loading and saving.
//!
//! This port uses a generic `C: DomainConfig` type parameter to avoid the domain layer
//! depending on the concrete `crate::config::Config` type. The infrastructure layer
//! implements this trait with the actual Config type.

use anyhow::Result;
use std::path::Path;

use crate::domain::value_objects::{ConfigWarning, DeployTarget, SecurityMode, Target};

/// Trait that defines what the domain layer needs from configuration.
///
/// This trait abstracts the configuration interface, allowing the domain layer
/// to work with configuration without depending on the concrete Config type.
pub trait DomainConfig: Clone + Default + Send + Sync {
    /// Get the security mode.
    fn security_mode(&self) -> SecurityMode;

    /// Get enabled targets.
    fn enabled_targets(&self) -> Vec<Target>;

    /// Get the deploy target setting.
    fn deploy_target(&self) -> DeployTarget;

    /// Get the format version.
    fn format_version(&self) -> &str;

    /// Check if atomic writes are enabled.
    fn atomic_writes(&self) -> bool;

    /// Check if lockfile should be respected.
    fn respect_lockfile(&self) -> bool;
}

/// Repository trait for configuration management.
///
/// Abstracts the underlying storage mechanism (TOML files, environment variables, etc.)
/// to allow for dependency injection and testing.
///
/// The generic type `C` must implement `DomainConfig`, which defines the interface
/// that the domain layer needs from configuration.
pub trait ConfigRepository<C: DomainConfig>: Send + Sync {
    /// Load configuration from a specific path.
    fn load(&self, path: &Path) -> Result<C>;

    /// Load configuration with warnings about unknown keys.
    fn load_with_warnings(&self, path: &Path) -> Result<(C, Vec<ConfigWarning>)>;

    /// Load configuration using the standard hierarchy:
    /// 1. Project config (.promptpack/config.toml)
    /// 2. User config (~/.config/calvin/config.toml)
    /// 3. Built-in defaults
    ///
    /// Then applies environment variable overrides.
    fn load_or_default(&self, project_root: Option<&Path>) -> C;

    /// Save the deploy target to the config file.
    ///
    /// Updates or creates the [deploy] section while preserving other settings.
    fn save_deploy_target(&self, config_path: &Path, target: DeployTarget) -> Result<()>;

    /// Check if a config file exists at the given path.
    fn exists(&self, path: &Path) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// A minimal test config that implements DomainConfig
    #[derive(Debug, Clone, Default)]
    struct TestConfig {
        security_mode: SecurityMode,
        deploy_target: DeployTarget,
        format_version: String,
        atomic_writes: bool,
        respect_lockfile: bool,
    }

    impl TestConfig {
        fn new() -> Self {
            Self {
                security_mode: SecurityMode::Balanced,
                deploy_target: DeployTarget::Unset,
                format_version: "1.0".to_string(),
                atomic_writes: true,
                respect_lockfile: true,
            }
        }

        fn with_security_mode(mut self, mode: SecurityMode) -> Self {
            self.security_mode = mode;
            self
        }
    }

    impl DomainConfig for TestConfig {
        fn security_mode(&self) -> SecurityMode {
            self.security_mode
        }

        fn enabled_targets(&self) -> Vec<Target> {
            vec![
                Target::ClaudeCode,
                Target::Cursor,
                Target::VSCode,
                Target::Antigravity,
                Target::Codex,
            ]
        }

        fn deploy_target(&self) -> DeployTarget {
            self.deploy_target
        }

        fn format_version(&self) -> &str {
            &self.format_version
        }

        fn atomic_writes(&self) -> bool {
            self.atomic_writes
        }

        fn respect_lockfile(&self) -> bool {
            self.respect_lockfile
        }
    }

    /// Mock implementation for testing
    struct MockConfigRepository {
        configs: Mutex<HashMap<String, TestConfig>>,
        default_config: TestConfig,
    }

    impl MockConfigRepository {
        fn new() -> Self {
            Self {
                configs: Mutex::new(HashMap::new()),
                default_config: TestConfig::new(),
            }
        }

        fn with_config(self, path: &str, config: TestConfig) -> Self {
            self.configs
                .lock()
                .unwrap()
                .insert(path.to_string(), config);
            self
        }
    }

    impl ConfigRepository<TestConfig> for MockConfigRepository {
        fn load(&self, path: &Path) -> Result<TestConfig> {
            let configs = self.configs.lock().unwrap();
            configs
                .get(path.to_string_lossy().as_ref())
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Config not found"))
        }

        fn load_with_warnings(&self, path: &Path) -> Result<(TestConfig, Vec<ConfigWarning>)> {
            let config = self.load(path)?;
            Ok((config, vec![]))
        }

        fn load_or_default(&self, project_root: Option<&Path>) -> TestConfig {
            if let Some(root) = project_root {
                // Use separate joins for cross-platform compatibility
                let project_config_path = root.join(".promptpack").join("config.toml");
                if let Ok(config) = self.load(&project_config_path) {
                    return config;
                }
            }
            self.default_config.clone()
        }

        fn save_deploy_target(&self, _config_path: &Path, _target: DeployTarget) -> Result<()> {
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
        assert_eq!(config.format_version(), "1.0");
    }

    #[test]
    fn mock_repository_returns_configured() {
        let custom = TestConfig::new().with_security_mode(SecurityMode::Strict);

        // Use platform-agnostic path construction for cross-platform compatibility
        // Build the same path that load_or_default will construct internally
        let project_root = Path::new("project");
        // Use PathBuf to ensure consistent path separators on all platforms
        let config_path = project_root.join(".promptpack").join("config.toml");
        let config_path_str = config_path.to_string_lossy().to_string();

        let repo = MockConfigRepository::new().with_config(&config_path_str, custom);

        let config = repo.load_or_default(Some(project_root));
        assert_eq!(config.security_mode(), SecurityMode::Strict);
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
            MockConfigRepository::new().with_config("/project/config.toml", TestConfig::new());

        assert!(repo.exists(Path::new("/project/config.toml")));
        assert!(!repo.exists(Path::new("/other/config.toml")));
    }

    #[test]
    fn domain_config_trait_methods() {
        let config = TestConfig::new();
        assert_eq!(config.security_mode(), SecurityMode::Balanced);
        assert_eq!(config.deploy_target(), DeployTarget::Unset);
        assert_eq!(config.format_version(), "1.0");
        assert!(config.atomic_writes());
        assert!(config.respect_lockfile());
        assert_eq!(config.enabled_targets().len(), 5);
    }
}
