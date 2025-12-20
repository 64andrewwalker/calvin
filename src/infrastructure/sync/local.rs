//! Local Sync Destinations
//!
//! Implements SyncDestination for local project and home directories.

use crate::domain::entities::OutputFile;
use crate::domain::ports::{SyncDestination, SyncDestinationError, SyncOptions, SyncResult};
use crate::domain::value_objects::Scope;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Sync destination for local project directory
pub struct LocalProjectDestination {
    /// Project root directory
    project_root: PathBuf,
}

impl LocalProjectDestination {
    /// Create a new local project destination
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }
}

impl SyncDestination for LocalProjectDestination {
    fn scope(&self) -> Scope {
        Scope::Project
    }

    fn display_name(&self) -> String {
        self.project_root.display().to_string()
    }

    fn exists(&self, path: &Path) -> bool {
        self.resolve_path(path).exists()
    }

    fn read(&self, path: &Path) -> Result<String, SyncDestinationError> {
        let resolved = self.resolve_path(path);
        std::fs::read_to_string(&resolved).map_err(|e| SyncDestinationError::IoError(e.to_string()))
    }

    fn hash(&self, path: &Path) -> Result<String, SyncDestinationError> {
        let content = self.read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        Ok(format!("sha256:{:x}", hasher.finalize()))
    }

    fn write_file(&self, path: &Path, content: &str) -> Result<(), SyncDestinationError> {
        let resolved = self.resolve_path(path);

        // Create parent directories
        if let Some(parent) = resolved.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| SyncDestinationError::IoError(e.to_string()))?;
        }

        std::fs::write(&resolved, content).map_err(|e| SyncDestinationError::IoError(e.to_string()))
    }

    fn delete_file(&self, path: &Path) -> Result<(), SyncDestinationError> {
        let resolved = self.resolve_path(path);
        if resolved.exists() {
            std::fs::remove_file(&resolved)
                .map_err(|e| SyncDestinationError::IoError(e.to_string()))?;
        }
        Ok(())
    }

    fn sync_batch(
        &self,
        outputs: &[OutputFile],
        options: &SyncOptions,
    ) -> Result<SyncResult, SyncDestinationError> {
        let mut result = SyncResult::default();

        for output in outputs {
            let path = output.path();
            let path_str = path.display().to_string();

            // Skip home paths in project destination
            if path_str.starts_with('~') {
                continue;
            }

            if options.dry_run {
                result.written.push(path.clone());
                continue;
            }

            match self.write_file(path, output.content()) {
                Ok(_) => result.written.push(path.clone()),
                Err(e) => result.errors.push(format!("{}: {}", path.display(), e)),
            }
        }

        Ok(result)
    }

    fn resolve_path(&self, path: &Path) -> PathBuf {
        self.project_root.join(path)
    }

    fn lockfile_path(&self, source: &Path) -> PathBuf {
        source.join(".calvin.lock")
    }
}

/// Sync destination for user home directory
pub struct LocalHomeDestination {
    /// Source directory (for lockfile path)
    source: PathBuf,
}

impl LocalHomeDestination {
    /// Create a new local home destination
    pub fn new(source: PathBuf) -> Self {
        Self { source }
    }

    /// Expand ~ to actual home directory
    fn expand_home(&self, path: &Path) -> PathBuf {
        let path_str = path.display().to_string();
        if let Some(stripped) = path_str.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(stripped);
            }
        } else if path_str == "~" {
            if let Some(home) = dirs::home_dir() {
                return home;
            }
        }
        path.to_path_buf()
    }
}

impl SyncDestination for LocalHomeDestination {
    fn scope(&self) -> Scope {
        Scope::User
    }

    fn display_name(&self) -> String {
        "~/".to_string()
    }

    fn exists(&self, path: &Path) -> bool {
        self.resolve_path(path).exists()
    }

    fn read(&self, path: &Path) -> Result<String, SyncDestinationError> {
        let resolved = self.resolve_path(path);
        std::fs::read_to_string(&resolved).map_err(|e| SyncDestinationError::IoError(e.to_string()))
    }

    fn hash(&self, path: &Path) -> Result<String, SyncDestinationError> {
        let content = self.read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        Ok(format!("sha256:{:x}", hasher.finalize()))
    }

    fn write_file(&self, path: &Path, content: &str) -> Result<(), SyncDestinationError> {
        let resolved = self.resolve_path(path);

        // Create parent directories
        if let Some(parent) = resolved.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| SyncDestinationError::IoError(e.to_string()))?;
        }

        std::fs::write(&resolved, content).map_err(|e| SyncDestinationError::IoError(e.to_string()))
    }

    fn delete_file(&self, path: &Path) -> Result<(), SyncDestinationError> {
        let resolved = self.resolve_path(path);
        if resolved.exists() {
            std::fs::remove_file(&resolved)
                .map_err(|e| SyncDestinationError::IoError(e.to_string()))?;
        }
        Ok(())
    }

    fn sync_batch(
        &self,
        outputs: &[OutputFile],
        options: &SyncOptions,
    ) -> Result<SyncResult, SyncDestinationError> {
        let mut result = SyncResult::default();

        for output in outputs {
            let path = output.path();

            if options.dry_run {
                result.written.push(path.clone());
                continue;
            }

            match self.write_file(path, output.content()) {
                Ok(_) => result.written.push(path.clone()),
                Err(e) => result.errors.push(format!("{}: {}", path.display(), e)),
            }
        }

        Ok(result)
    }

    fn resolve_path(&self, path: &Path) -> PathBuf {
        self.expand_home(path)
    }

    fn lockfile_path(&self, _source: &Path) -> PathBuf {
        self.source.join(".calvin.lock")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn local_project_scope() {
        let dest = LocalProjectDestination::new(PathBuf::from("/project"));
        assert_eq!(dest.scope(), Scope::Project);
    }

    #[test]
    fn local_project_resolve_path() {
        let dest = LocalProjectDestination::new(PathBuf::from("/project"));
        let resolved = dest.resolve_path(Path::new(".cursor/rules/test.md"));
        assert_eq!(resolved, PathBuf::from("/project/.cursor/rules/test.md"));
    }

    #[test]
    fn local_home_scope() {
        let dest = LocalHomeDestination::new(PathBuf::from(".promptpack"));
        assert_eq!(dest.scope(), Scope::User);
    }

    #[test]
    fn local_project_write_and_read() {
        let dir = tempdir().unwrap();
        let dest = LocalProjectDestination::new(dir.path().to_path_buf());

        // Write
        dest.write_file(Path::new("test.md"), "content").unwrap();

        // Check exists
        assert!(dest.exists(Path::new("test.md")));

        // Read
        let content = dest.read(Path::new("test.md")).unwrap();
        assert_eq!(content, "content");
    }

    #[test]
    fn local_project_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let dest = LocalProjectDestination::new(dir.path().to_path_buf());

        dest.write_file(Path::new("nested/deep/test.md"), "content")
            .unwrap();

        assert!(dest.exists(Path::new("nested/deep/test.md")));
    }
}
