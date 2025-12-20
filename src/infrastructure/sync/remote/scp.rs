//! SCP Transfer Strategy
//!
//! Uses scp for file transfers to remote servers.
//! This is the fallback method when rsync is not available,
//! particularly on Windows systems with OpenSSH.

use super::transfer::TransferStrategy;
use crate::domain::ports::{SyncDestinationError, SyncOptions, SyncResult};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Transfer strategy using scp
///
/// SCP is widely available on Windows 10+ via OpenSSH,
/// making it a good fallback when rsync is not installed.
///
/// Unlike rsync, scp doesn't:
/// - Do incremental transfers (always copies full files)
/// - Create remote directories automatically
///
/// This implementation compensates by:
/// 1. Pre-creating directories via SSH mkdir -p
/// 2. Using scp -r for recursive directory transfer
pub struct ScpTransfer;

impl ScpTransfer {
    /// Check if scp is installed and available
    pub fn check_available() -> bool {
        // scp without args returns non-zero, but if we can spawn it, it's available
        Command::new("scp")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    }

    /// Create remote directories via SSH
    fn create_remote_dirs(
        remote_host: &str,
        remote_path: &str,
        dirs: &HashSet<PathBuf>,
        json_mode: bool,
    ) -> Result<(), SyncDestinationError> {
        if dirs.is_empty() {
            return Ok(());
        }

        let dirs_to_create: Vec<String> = dirs
            .iter()
            .map(|d| format!("{}/{}", remote_path, d.display()))
            .collect();

        let mkdir_cmd = format!("mkdir -p {}", shell_quote_paths(&dirs_to_create));

        let status = Command::new("ssh")
            .arg(remote_host)
            .arg(&mkdir_cmd)
            .stdout(Stdio::null())
            .stderr(if json_mode {
                Stdio::null()
            } else {
                Stdio::inherit()
            })
            .status()
            .map_err(|e| SyncDestinationError::ConnectionError(e.to_string()))?;

        if !status.success() {
            return Err(SyncDestinationError::CommandFailed(
                "Failed to create remote directories".to_string(),
            ));
        }

        Ok(())
    }

    /// Collect parent directories from staged files
    fn collect_parent_dirs(staged_files: &[PathBuf]) -> HashSet<PathBuf> {
        staged_files
            .iter()
            .filter_map(|p| p.parent())
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.to_path_buf())
            .collect()
    }
}

impl TransferStrategy for ScpTransfer {
    fn name(&self) -> &'static str {
        "scp"
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

        // Step 1: Create remote directories (scp doesn't do this automatically)
        let parent_dirs = Self::collect_parent_dirs(staged_files);
        Self::create_remote_dirs(remote_host, remote_path, &parent_dirs, options.json)?;

        // Step 2: Build scp command
        let mut cmd = Command::new("scp");
        cmd.arg("-r") // recursive
            .arg("-p") // preserve timestamps
            .stdin(Stdio::inherit()); // Allow password input

        if !options.json && options.verbose {
            cmd.arg("-v");
        }

        // Add all top-level items from staging directory
        let entries: Vec<_> = std::fs::read_dir(staging_root)
            .map_err(|e| SyncDestinationError::IoError(e.to_string()))?
            .filter_map(|e| e.ok())
            .collect();

        if entries.is_empty() {
            // Nothing to transfer
            return Ok(SyncResult {
                written: vec![],
                skipped: vec![],
                errors: vec![],
            });
        }

        for entry in &entries {
            cmd.arg(entry.path());
        }

        cmd.arg(&remote_dest);

        if options.json {
            cmd.stdout(Stdio::null()).stderr(Stdio::null());
        } else {
            cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        }

        let status = cmd
            .status()
            .map_err(|e| SyncDestinationError::CommandFailed(e.to_string()))?;

        if !status.success() {
            return Err(SyncDestinationError::CommandFailed(format!(
                "scp failed with exit code: {:?}",
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

/// Quote paths for shell command (simple escaping)
fn shell_quote_paths(paths: &[String]) -> String {
    paths
        .iter()
        .map(|p| format!("'{}'", p.replace('\'', "'\\''")))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scp_transfer_name() {
        let transfer = ScpTransfer;
        assert_eq!(transfer.name(), "scp");
    }

    #[test]
    fn check_available_does_not_panic() {
        let _ = ScpTransfer::check_available();
    }

    #[test]
    fn collect_parent_dirs_empty() {
        let files: Vec<PathBuf> = vec![];
        let dirs = ScpTransfer::collect_parent_dirs(&files);
        assert!(dirs.is_empty());
    }

    #[test]
    fn collect_parent_dirs_with_subdirs() {
        let files = vec![
            PathBuf::from(".claude/commands/test.md"),
            PathBuf::from(".cursor/rules/rule.mdc"),
            PathBuf::from("AGENTS.md"),
        ];
        let dirs = ScpTransfer::collect_parent_dirs(&files);
        assert!(dirs.contains(&PathBuf::from(".claude/commands")));
        assert!(dirs.contains(&PathBuf::from(".cursor/rules")));
        assert_eq!(dirs.len(), 2); // AGENTS.md has no parent dir (empty)
    }

    #[test]
    fn shell_quote_paths_simple() {
        let paths = vec!["dir/file.txt".to_string()];
        assert_eq!(shell_quote_paths(&paths), "'dir/file.txt'");
    }

    #[test]
    fn shell_quote_paths_with_quotes() {
        let paths = vec!["it's a file.txt".to_string()];
        assert_eq!(shell_quote_paths(&paths), "'it'\\''s a file.txt'");
    }
}
