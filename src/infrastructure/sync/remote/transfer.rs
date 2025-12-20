//! Remote Transfer Strategy
//!
//! Defines the interface for batch file transfers to remote servers.

use crate::domain::ports::{SyncDestinationError, SyncOptions, SyncResult};
use std::path::{Path, PathBuf};

/// Strategy for transferring files to a remote server
pub trait TransferStrategy: Send + Sync {
    /// Get the name of this transfer method (for logging)
    fn name(&self) -> &'static str;

    /// Check if this transfer method is available on the system
    fn is_available(&self) -> bool;

    /// Perform a batch transfer of files from staging directory to remote
    ///
    /// # Arguments
    /// * `staging_root` - Local directory containing staged files
    /// * `remote_host` - Remote host (e.g., "user@host")
    /// * `remote_path` - Remote base path (e.g., "/home/user/project")
    /// * `staged_files` - List of file paths that were staged (for result reporting)
    /// * `options` - Sync options (dry_run, verbose, json)
    fn transfer(
        &self,
        staging_root: &Path,
        remote_host: &str,
        remote_path: &str,
        staged_files: &[PathBuf],
        options: &SyncOptions,
    ) -> Result<SyncResult, SyncDestinationError>;
}

/// Detect and return the best available transfer strategy
pub fn detect_strategy() -> Option<Box<dyn TransferStrategy>> {
    // Try rsync first (preferred)
    let rsync = super::rsync::RsyncTransfer;
    if rsync.is_available() {
        return Some(Box::new(rsync));
    }

    // Fallback to scp (common on Windows)
    let scp = super::scp::ScpTransfer;
    if scp.is_available() {
        return Some(Box::new(scp));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_strategy_does_not_panic() {
        // Just verify it doesn't panic, actual result depends on system
        let _ = detect_strategy();
    }
}
