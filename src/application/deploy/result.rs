//! Deploy Result
//!
//! Result types for deploy operations.

use std::path::PathBuf;

/// Result of a deploy operation
#[derive(Debug, Clone)]
pub struct DeployResult {
    /// Files that were written
    pub written: Vec<PathBuf>,
    /// Files that were skipped (up-to-date)
    pub skipped: Vec<PathBuf>,
    /// Files that were deleted (orphans)
    pub deleted: Vec<PathBuf>,
    /// Errors encountered
    pub errors: Vec<String>,
    /// Total asset count
    pub asset_count: usize,
    /// Total output count
    pub output_count: usize,
}

impl DeployResult {
    pub fn new() -> Self {
        Self {
            written: Vec::new(),
            skipped: Vec::new(),
            deleted: Vec::new(),
            errors: Vec::new(),
            asset_count: 0,
            output_count: 0,
        }
    }

    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn has_changes(&self) -> bool {
        !self.written.is_empty() || !self.deleted.is_empty()
    }
}

impl Default for DeployResult {
    fn default() -> Self {
        Self::new()
    }
}
