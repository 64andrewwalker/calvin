//! Rsync Transfer Strategy
//!
//! Uses rsync for efficient incremental file transfers.
//! This is the preferred method on Unix systems.

use super::transfer::TransferStrategy;
use crate::domain::ports::{SyncDestinationError, SyncOptions, SyncResult};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Transfer strategy using rsync
///
/// Rsync provides efficient incremental transfers by only
/// sending changed portions of files.
pub struct RsyncTransfer;

impl RsyncTransfer {
    /// Check if rsync is installed and available
    pub fn check_available() -> bool {
        Command::new("rsync")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

impl TransferStrategy for RsyncTransfer {
    fn name(&self) -> &'static str {
        "rsync"
    }

    fn is_available(&self) -> bool {
        Self::check_available()
    }

    fn transfer(
        &self,
        staging_root: &Path,
        remote_host: &str,
        remote_path: &str,
        staged_files: &[PathBuf],
        options: &SyncOptions,
    ) -> Result<SyncResult, SyncDestinationError> {
        let remote_dest = format!("{}:{}", remote_host, remote_path);

        let mut cmd = Command::new("rsync");
        cmd.arg("-avz")
            .arg("--progress")
            .arg("-e")
            .arg("ssh")
            .arg(format!("{}/", staging_root.display())) // trailing slash = copy contents
            .arg(&remote_dest)
            .stdin(Stdio::inherit()); // Allow password input

        if options.json {
            cmd.stdout(Stdio::null()).stderr(Stdio::piped());
        } else {
            cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        }

        let status = cmd
            .status()
            .map_err(|e| SyncDestinationError::CommandFailed(e.to_string()))?;

        if !status.success() {
            return Err(SyncDestinationError::CommandFailed(format!(
                "rsync failed with exit code: {:?}",
                status.code()
            )));
        }

        Ok(SyncResult {
            written: staged_files.to_vec(),
            skipped: vec![],
            errors: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rsync_transfer_name() {
        let transfer = RsyncTransfer;
        assert_eq!(transfer.name(), "rsync");
    }

    #[test]
    fn check_available_does_not_panic() {
        let _ = RsyncTransfer::check_available();
    }
}
