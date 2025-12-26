//! RegistryRepository port
//!
//! Persists the global registry at `~/.calvin/registry.toml`.

use crate::domain::entities::{ProjectEntry, Registry};
use std::path::PathBuf;

pub trait RegistryRepository: Send + Sync {
    fn load(&self) -> Result<Registry, RegistryError>;
    fn save(&self, registry: &Registry) -> Result<(), RegistryError>;
    fn update_project(&self, entry: ProjectEntry) -> Result<(), RegistryError>;
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RegistryError {
    #[error("Failed to access registry: {message}")]
    AccessError { message: String },

    #[error("Failed to serialize registry: {message}")]
    SerializationError { message: String },

    #[error(
        "registry file corrupted: {path}\n  → Fix: Delete and rebuild registry\n  → Run: rm {path} && calvin deploy\n  → Details: {message}"
    )]
    Corrupted { path: PathBuf, message: String },
}
