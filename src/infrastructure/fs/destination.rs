//! Destination File System Adapter
//!
//! Adapts a SyncDestination to the FileSystem trait.
//! This allows DeployUseCase to work with any SyncDestination.

use crate::domain::ports::file_system::{FileSystem, FsError, FsResult};
use crate::domain::ports::{SyncDestination, SyncDestinationError};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Adapts a SyncDestination to implement FileSystem.
///
/// This allows DeployUseCase to work with remote destinations
/// without changing its internal logic.
pub struct DestinationFs<D: SyncDestination> {
    destination: Arc<D>,
}

impl<D: SyncDestination> DestinationFs<D> {
    /// Create a new DestinationFs from a SyncDestination
    pub fn new(destination: Arc<D>) -> Self {
        Self { destination }
    }
}

impl<D: SyncDestination + 'static> FileSystem for DestinationFs<D> {
    fn read(&self, path: &Path) -> FsResult<String> {
        self.destination.read(path).map_err(convert_error)
    }

    fn write(&self, path: &Path, content: &str) -> FsResult<()> {
        self.destination
            .write_file(path, content)
            .map_err(convert_error)
    }

    fn exists(&self, path: &Path) -> bool {
        self.destination.exists(path)
    }

    fn remove(&self, path: &Path) -> FsResult<()> {
        self.destination.delete_file(path).map_err(convert_error)
    }

    fn create_dir_all(&self, _path: &Path) -> FsResult<()> {
        // SyncDestination handles directory creation in write_file
        Ok(())
    }

    fn hash(&self, path: &Path) -> FsResult<String> {
        self.destination.hash(path).map_err(convert_error)
    }

    fn expand_home(&self, path: &Path) -> PathBuf {
        self.destination.resolve_path(path)
    }
}

fn convert_error(e: SyncDestinationError) -> FsError {
    match e {
        SyncDestinationError::IoError(msg) => FsError::Other(msg),
        SyncDestinationError::ConnectionError(msg) => FsError::Other(msg),
        SyncDestinationError::CommandFailed(msg) => FsError::Other(msg),
        SyncDestinationError::NotAvailable(msg) => FsError::Other(msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::OutputFile;
    use crate::domain::ports::{SyncOptions, SyncResult as DestSyncResult};
    use crate::domain::value_objects::Scope;

    /// Mock SyncDestination for testing
    struct MockDestination {
        files: std::sync::Mutex<std::collections::HashMap<PathBuf, String>>,
    }

    impl MockDestination {
        fn new() -> Self {
            Self {
                files: std::sync::Mutex::new(std::collections::HashMap::new()),
            }
        }
    }

    impl SyncDestination for MockDestination {
        fn scope(&self) -> Scope {
            Scope::Project
        }

        fn display_name(&self) -> String {
            "mock".to_string()
        }

        fn exists(&self, path: &Path) -> bool {
            self.files.lock().unwrap().contains_key(path)
        }

        fn read(&self, path: &Path) -> Result<String, SyncDestinationError> {
            self.files
                .lock()
                .unwrap()
                .get(path)
                .cloned()
                .ok_or_else(|| SyncDestinationError::IoError("not found".to_string()))
        }

        fn hash(&self, path: &Path) -> Result<String, SyncDestinationError> {
            self.files
                .lock()
                .unwrap()
                .get(path)
                .map(|content| {
                    use sha2::{Digest, Sha256};
                    let mut hasher = Sha256::new();
                    hasher.update(content.as_bytes());
                    format!("sha256:{:x}", hasher.finalize())
                })
                .ok_or_else(|| SyncDestinationError::IoError("not found".to_string()))
        }

        fn write_file(&self, path: &Path, content: &str) -> Result<(), SyncDestinationError> {
            self.files
                .lock()
                .unwrap()
                .insert(path.to_path_buf(), content.to_string());
            Ok(())
        }

        fn delete_file(&self, path: &Path) -> Result<(), SyncDestinationError> {
            self.files.lock().unwrap().remove(path);
            Ok(())
        }

        fn sync_batch(
            &self,
            outputs: &[OutputFile],
            _options: &SyncOptions,
        ) -> Result<DestSyncResult, SyncDestinationError> {
            let mut written = Vec::new();
            for output in outputs {
                self.write_file(output.path(), output.content())?;
                written.push(output.path().clone());
            }
            Ok(DestSyncResult {
                written,
                skipped: vec![],
                errors: vec![],
            })
        }

        fn resolve_path(&self, path: &Path) -> PathBuf {
            path.to_path_buf()
        }

        fn lockfile_path(&self, source: &Path) -> PathBuf {
            source.join(".calvin.lock")
        }
    }

    #[test]
    fn destination_fs_write_and_read() {
        let dest = Arc::new(MockDestination::new());
        let fs = DestinationFs::new(dest);

        let path = Path::new("test.md");
        fs.write(path, "hello").unwrap();

        let content = fs.read(path).unwrap();
        assert_eq!(content, "hello");
    }

    #[test]
    fn destination_fs_exists() {
        let dest = Arc::new(MockDestination::new());
        let fs = DestinationFs::new(dest);

        let path = Path::new("test.md");
        assert!(!fs.exists(path));

        fs.write(path, "hello").unwrap();
        assert!(fs.exists(path));
    }

    #[test]
    fn destination_fs_remove() {
        let dest = Arc::new(MockDestination::new());
        let fs = DestinationFs::new(dest);

        let path = Path::new("test.md");
        fs.write(path, "hello").unwrap();
        assert!(fs.exists(path));

        fs.remove(path).unwrap();
        assert!(!fs.exists(path));
    }
}
