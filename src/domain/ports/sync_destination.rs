//! Sync Destination Port
//!
//! Abstracts the destination for deploying files (local project, home, remote).
//! This allows DeployUseCase to work with different destinations without
//! knowing the implementation details.

use crate::domain::entities::OutputFile;
use crate::domain::value_objects::Scope;
use std::path::PathBuf;

/// Error during sync operations
#[derive(Debug, Clone)]
pub enum SyncDestinationError {
    /// File system error
    IoError(String),
    /// Remote connection error
    ConnectionError(String),
    /// Command execution error
    CommandFailed(String),
    /// Feature not available
    NotAvailable(String),
}

impl std::fmt::Display for SyncDestinationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(msg) => write!(f, "I/O error: {}", msg),
            Self::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            Self::CommandFailed(msg) => write!(f, "Command failed: {}", msg),
            Self::NotAvailable(msg) => write!(f, "Not available: {}", msg),
        }
    }
}

impl std::error::Error for SyncDestinationError {}

/// Result of a sync operation
#[derive(Debug, Clone, Default)]
pub struct SyncResult {
    /// Files that were successfully written
    pub written: Vec<PathBuf>,
    /// Files that were skipped (up-to-date or conflicts)
    pub skipped: Vec<PathBuf>,
    /// Errors encountered
    pub errors: Vec<String>,
}

impl SyncResult {
    /// Check if sync was successful (no errors)
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }

    /// Check if there were any changes
    pub fn has_changes(&self) -> bool {
        !self.written.is_empty()
    }
}

/// Options for sync operations
#[derive(Debug, Clone, Default)]
pub struct SyncOptions {
    /// Force overwrite without conflict detection
    pub force: bool,
    /// Dry run mode (don't write files)
    pub dry_run: bool,
    /// Verbose output
    pub verbose: bool,
    /// JSON mode (suppress interactive output)
    pub json: bool,
}

/// Trait for sync destinations
///
/// This abstracts over different deployment targets:
/// - Local project directory
/// - User home directory
/// - Remote server via SSH/rsync
pub trait SyncDestination: Send + Sync {
    /// Get the scope of this destination
    fn scope(&self) -> Scope;

    /// Get a display name for this destination
    fn display_name(&self) -> String;

    /// Check if a file exists at the destination
    fn exists(&self, path: &std::path::Path) -> bool;

    /// Read file content from the destination
    fn read(&self, path: &std::path::Path) -> Result<String, SyncDestinationError>;

    /// Get the hash of a file at the destination
    fn hash(&self, path: &std::path::Path) -> Result<String, SyncDestinationError>;

    /// Write a single file to the destination
    fn write_file(&self, path: &std::path::Path, content: &str)
        -> Result<(), SyncDestinationError>;

    /// Delete a file from the destination
    fn delete_file(&self, path: &std::path::Path) -> Result<(), SyncDestinationError>;

    /// Batch sync multiple files to the destination
    ///
    /// This is the preferred method for syncing as it can be optimized
    /// (e.g., using rsync for remote destinations).
    fn sync_batch(
        &self,
        outputs: &[OutputFile],
        options: &SyncOptions,
    ) -> Result<SyncResult, SyncDestinationError>;

    /// Check if this destination supports batch sync
    fn supports_batch_sync(&self) -> bool {
        true
    }

    /// Resolve a path for this destination
    ///
    /// For local project: joins with project root
    /// For home: expands ~ prefix
    /// For remote: returns as-is (remote handles paths)
    fn resolve_path(&self, path: &std::path::Path) -> PathBuf;

    /// Get the lockfile path for this destination
    fn lockfile_path(&self, source: &std::path::Path) -> PathBuf;
}
