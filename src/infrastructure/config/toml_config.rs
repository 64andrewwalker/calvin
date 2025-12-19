//! TOML-based configuration repository implementation.

use std::path::Path;

use anyhow::Result;

use crate::config::{Config, ConfigWarning, DeployTargetConfig};
use crate::domain::ports::ConfigRepository;

/// TOML configuration repository implementation.
///
/// Delegates to the existing Config methods for file loading/saving,
/// providing a trait-based abstraction for dependency injection.
#[derive(Debug, Clone, Copy, Default)]
pub struct TomlConfigRepository;

impl TomlConfigRepository {
    pub fn new() -> Self {
        Self
    }
}

impl ConfigRepository for TomlConfigRepository {
    fn load(&self, path: &Path) -> Result<Config> {
        Config::load(path).map_err(Into::into)
    }

    fn load_with_warnings(&self, path: &Path) -> Result<(Config, Vec<ConfigWarning>)> {
        Config::load_with_warnings(path).map_err(Into::into)
    }

    fn load_or_default(&self, project_root: Option<&Path>) -> Config {
        Config::load_or_default(project_root)
    }

    fn save_deploy_target(&self, config_path: &Path, target: DeployTargetConfig) -> Result<()> {
        Config::save_deploy_target(config_path, target).map_err(Into::into)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SecurityMode;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn load_valid_config() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
[format]
version = "1.0"

[security]
mode = "strict"
"#,
        )
        .unwrap();

        let repo = TomlConfigRepository::new();
        let config = repo.load(&config_path).unwrap();

        assert_eq!(config.format.version, "1.0");
        assert_eq!(config.security.mode, SecurityMode::Strict);
    }

    #[test]
    fn load_missing_config_errors() {
        let repo = TomlConfigRepository::new();
        let result = repo.load(Path::new("/nonexistent/path/config.toml"));

        assert!(result.is_err());
    }

    #[test]
    fn load_or_default_returns_default_when_no_config() {
        let repo = TomlConfigRepository::new();
        let config = repo.load_or_default(None);

        assert_eq!(config.format.version, "1.0");
        assert_eq!(config.security.mode, SecurityMode::Balanced);
    }

    #[test]
    fn load_or_default_uses_project_config() {
        let dir = tempdir().unwrap();
        let promptpack_dir = dir.path().join(".promptpack");
        fs::create_dir_all(&promptpack_dir).unwrap();

        let config_path = promptpack_dir.join("config.toml");
        fs::write(
            &config_path,
            r#"
[security]
mode = "yolo"
"#,
        )
        .unwrap();

        let repo = TomlConfigRepository::new();
        let config = repo.load_or_default(Some(dir.path()));

        assert_eq!(config.security.mode, SecurityMode::Yolo);
    }

    #[test]
    fn load_with_warnings_detects_unknown_keys() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
[security]
mode = "balanced"
unknown_key = "value"
"#,
        )
        .unwrap();

        let repo = TomlConfigRepository::new();
        let (config, warnings) = repo.load_with_warnings(&config_path).unwrap();

        assert_eq!(config.security.mode, SecurityMode::Balanced);
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.key == "unknown_key"));
    }

    #[test]
    fn save_deploy_target_creates_section() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        // Start with empty config
        fs::write(&config_path, "[format]\nversion = \"1.0\"\n").unwrap();

        let repo = TomlConfigRepository::new();
        repo.save_deploy_target(&config_path, DeployTargetConfig::Home)
            .unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("[deploy]"));
        assert!(content.contains("target = \"home\""));
    }

    #[test]
    fn save_deploy_target_updates_existing() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        // Start with existing deploy section
        fs::write(
            &config_path,
            r#"
[format]
version = "1.0"

[deploy]
target = "home"
"#,
        )
        .unwrap();

        let repo = TomlConfigRepository::new();
        repo.save_deploy_target(&config_path, DeployTargetConfig::Project)
            .unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("target = \"project\""));
        assert!(!content.contains("target = \"home\""));
    }

    #[test]
    fn exists_returns_true_for_existing_file() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(&config_path, "").unwrap();

        let repo = TomlConfigRepository::new();
        assert!(repo.exists(&config_path));
    }

    #[test]
    fn exists_returns_false_for_missing_file() {
        let repo = TomlConfigRepository::new();
        assert!(!repo.exists(Path::new("/nonexistent/config.toml")));
    }

    #[test]
    fn save_deploy_target_unset_does_nothing() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        let original = "[format]\nversion = \"1.0\"\n";
        fs::write(&config_path, original).unwrap();

        let repo = TomlConfigRepository::new();
        repo.save_deploy_target(&config_path, DeployTargetConfig::Unset)
            .unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        assert_eq!(content, original);
    }
}
