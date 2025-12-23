//! Clean result types

use std::path::PathBuf;

/// Reason why a file was skipped during clean
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    /// File was modified (hash mismatch)
    Modified,
    /// File is missing
    Missing,
    /// File doesn't have Calvin signature
    NoSignature,
    /// Permission denied
    PermissionDenied,
    /// Remote deployment (not supported yet)
    Remote,
}

impl std::fmt::Display for SkipReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkipReason::Modified => write!(f, "modified"),
            SkipReason::Missing => write!(f, "missing"),
            SkipReason::NoSignature => write!(f, "no signature"),
            SkipReason::PermissionDenied => write!(f, "permission denied"),
            SkipReason::Remote => write!(f, "remote deployment"),
        }
    }
}

/// A file that was skipped during clean
#[derive(Debug, Clone)]
pub struct SkippedFile {
    /// Path to the file
    pub path: PathBuf,
    /// Reason for skipping
    pub reason: SkipReason,
}

impl SkippedFile {
    pub fn new(path: PathBuf, reason: SkipReason) -> Self {
        Self { path, reason }
    }
}

/// Result of a clean operation
#[derive(Debug, Clone, Default)]
pub struct CleanResult {
    /// Files that were deleted (or would be deleted in dry run)
    pub deleted: Vec<PathBuf>,
    /// Files that were skipped
    pub skipped: Vec<SkippedFile>,
    /// Errors that occurred
    pub errors: Vec<String>,
}

impl CleanResult {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a deleted file
    pub fn add_deleted(&mut self, path: PathBuf) {
        self.deleted.push(path);
    }

    /// Add a skipped file
    pub fn add_skipped(&mut self, path: PathBuf, reason: SkipReason) {
        self.skipped.push(SkippedFile::new(path, reason));
    }

    /// Add an error
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// Get total count of files considered
    pub fn total_count(&self) -> usize {
        self.deleted.len() + self.skipped.len()
    }

    /// Check if operation was successful
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}
