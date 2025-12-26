//! TOML Registry Repository
//!
//! Persists the global registry at `~/.calvin/registry.toml`.

use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};

use crate::domain::entities::{ProjectEntry, Registry};
use crate::domain::ports::{RegistryError, RegistryRepository};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TomlProjectEntry {
    path: PathBuf,
    lockfile: PathBuf,
    last_deployed: DateTime<Utc>,
    asset_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TomlRegistry {
    version: u32,
    #[serde(default)]
    projects: Vec<TomlProjectEntry>,
}

pub struct TomlRegistryRepository {
    path: PathBuf,
}

impl TomlRegistryRepository {
    pub fn new() -> Self {
        Self {
            path: default_registry_path(),
        }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    fn lock_path(&self) -> PathBuf {
        self.path.with_extension("lock")
    }

    fn load_from_disk(&self) -> Result<Registry, RegistryError> {
        if !self.path.exists() {
            return Ok(Registry::new());
        }

        let content = fs::read_to_string(&self.path).map_err(|e| RegistryError::AccessError {
            message: e.to_string(),
        })?;

        let toml_reg: TomlRegistry =
            toml::from_str(&content).map_err(|e| RegistryError::Corrupted {
                path: self.path.clone(),
                message: e.to_string(),
            })?;

        Ok(from_toml(toml_reg))
    }

    fn save_to_disk(&self, registry: &Registry) -> Result<(), RegistryError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| RegistryError::AccessError {
                message: e.to_string(),
            })?;
        }

        let toml_reg = to_toml(registry);
        let content =
            toml::to_string_pretty(&toml_reg).map_err(|e| RegistryError::SerializationError {
                message: e.to_string(),
            })?;

        fs::write(&self.path, content).map_err(|e| RegistryError::AccessError {
            message: e.to_string(),
        })?;

        Ok(())
    }
}

impl Default for TomlRegistryRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryRepository for TomlRegistryRepository {
    fn load(&self) -> Result<Registry, RegistryError> {
        self.load_from_disk()
    }

    fn save(&self, registry: &Registry) -> Result<(), RegistryError> {
        let lock_path = self.lock_path();
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent).map_err(|e| RegistryError::AccessError {
                message: e.to_string(),
            })?;
        }

        let lock_file = fs::File::create(&lock_path).map_err(|e| RegistryError::AccessError {
            message: e.to_string(),
        })?;
        lock_file
            .lock_exclusive()
            .map_err(|e| RegistryError::AccessError {
                message: e.to_string(),
            })?;

        let result = self.save_to_disk(registry);

        let _ = lock_file.unlock();
        result
    }

    fn update_project(&self, entry: ProjectEntry) -> Result<(), RegistryError> {
        let lock_path = self.lock_path();
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent).map_err(|e| RegistryError::AccessError {
                message: e.to_string(),
            })?;
        }

        let lock_file = fs::File::create(&lock_path).map_err(|e| RegistryError::AccessError {
            message: e.to_string(),
        })?;
        lock_file
            .lock_exclusive()
            .map_err(|e| RegistryError::AccessError {
                message: e.to_string(),
            })?;

        let mut registry = self.load_from_disk()?;
        registry.upsert(entry);
        let result = self.save_to_disk(&registry);

        let _ = lock_file.unlock();
        result
    }
}

fn default_registry_path() -> PathBuf {
    // Allow override for testing (especially on Windows where dirs::home_dir
    // uses system API and cannot be overridden via environment variables)
    if let Ok(path) = std::env::var("CALVIN_REGISTRY_PATH") {
        return PathBuf::from(path);
    }
    dirs::home_dir()
        .map(|h| h.join(".calvin/registry.toml"))
        .unwrap_or_else(|| PathBuf::from("~/.calvin/registry.toml"))
}

fn from_toml(toml_registry: TomlRegistry) -> Registry {
    let mut registry = Registry::new();
    registry.version = toml_registry.version;
    registry.projects = toml_registry
        .projects
        .into_iter()
        .map(|p| ProjectEntry {
            path: p.path,
            lockfile: p.lockfile,
            last_deployed: p.last_deployed,
            asset_count: p.asset_count,
        })
        .collect();
    registry
}

fn to_toml(registry: &Registry) -> TomlRegistry {
    TomlRegistry {
        version: registry.version,
        projects: registry
            .projects
            .iter()
            .cloned()
            .map(|p| TomlProjectEntry {
                path: p.path,
                lockfile: p.lockfile,
                last_deployed: p.last_deployed,
                asset_count: p.asset_count,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_missing_returns_empty() {
        let dir = tempdir().unwrap();
        let repo = TomlRegistryRepository::with_path(dir.path().join("registry.toml"));
        let reg = repo.load().unwrap();
        assert!(reg.projects.is_empty());
        assert_eq!(reg.version, 1);
    }

    #[test]
    fn load_corrupted_returns_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.toml");
        fs::write(&path, "this is not toml = = =").unwrap();

        let repo = TomlRegistryRepository::with_path(path.clone());
        let err = repo.load().unwrap_err();
        assert!(matches!(err, RegistryError::Corrupted { .. }));

        let msg = err.to_string();
        assert!(msg.contains("registry file corrupted"));
        assert!(msg.contains(&path.display().to_string()));
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let repo = TomlRegistryRepository::with_path(dir.path().join("registry.toml"));

        let mut reg = Registry::new();
        reg.upsert(ProjectEntry {
            path: PathBuf::from("/project"),
            lockfile: PathBuf::from("/project/calvin.lock"),
            last_deployed: Utc::now(),
            asset_count: 7,
        });

        repo.save(&reg).unwrap();
        let loaded = repo.load().unwrap();
        assert_eq!(loaded.projects.len(), 1);
        assert_eq!(loaded.projects[0].asset_count, 7);
    }

    #[test]
    fn update_project_is_upsert() {
        let dir = tempdir().unwrap();
        let repo = TomlRegistryRepository::with_path(dir.path().join("registry.toml"));

        let entry = ProjectEntry {
            path: PathBuf::from("/project"),
            lockfile: PathBuf::from("/project/calvin.lock"),
            last_deployed: Utc::now(),
            asset_count: 1,
        };
        repo.update_project(entry.clone()).unwrap();
        repo.update_project(ProjectEntry {
            asset_count: 2,
            ..entry
        })
        .unwrap();

        let loaded = repo.load().unwrap();
        assert_eq!(loaded.projects.len(), 1);
        assert_eq!(loaded.projects[0].asset_count, 2);
    }
}
