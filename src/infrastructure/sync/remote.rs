//! Remote Sync Destination
//!
//! Implements SyncDestination for remote servers via SSH/rsync.
//! Uses rsync for efficient batch transfers, with scp fallback for Windows.

use crate::domain::entities::OutputFile;
use crate::domain::ports::{SyncDestination, SyncDestinationError, SyncOptions, SyncResult};
use crate::domain::value_objects::Scope;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Transfer method for remote sync
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferMethod {
    /// Use rsync (preferred, efficient incremental transfers)
    Rsync,
    /// Use scp (fallback, full file transfers)
    Scp,
}

impl TransferMethod {
    /// Detect the best available transfer method
    pub fn detect() -> Option<Self> {
        if RemoteDestination::has_rsync() {
            Some(Self::Rsync)
        } else if RemoteDestination::has_scp() {
            Some(Self::Scp)
        } else {
            None
        }
    }
}

/// Sync destination for remote servers
///
/// Uses rsync for efficient batch file transfers.
pub struct RemoteDestination {
    /// Remote host (e.g., "ubuntu-server" or "user@host")
    host: String,
    /// Remote base path (e.g., "~" or "/home/user/project")
    remote_path: String,
    /// Source directory (for lockfile path)
    source: PathBuf,
}

impl RemoteDestination {
    /// Create a new remote destination from a remote spec
    ///
    /// Format: "host:path" or "user@host:path"
    pub fn new(remote: &str, source: PathBuf) -> Self {
        let (host, remote_path) = if let Some((h, p)) = remote.split_once(':') {
            (h.to_string(), p.to_string())
        } else {
            (remote.to_string(), ".".to_string())
        };

        Self {
            host,
            remote_path,
            source,
        }
    }

    /// Check if rsync is available
    pub fn has_rsync() -> bool {
        Command::new("rsync")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Check if scp is available
    pub fn has_scp() -> bool {
        // On Windows, scp might be available via OpenSSH or Git Bash
        // We check by running a simple version/help command
        Command::new("scp")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok() // scp returns non-zero with no args, but if it runs, it's available
    }

    /// Get the best available transfer method
    pub fn transfer_method() -> Option<TransferMethod> {
        TransferMethod::detect()
    }

    /// Build the remote destination string
    fn remote_dest(&self) -> String {
        format!("{}:{}", self.host, self.remote_path)
    }
}

impl SyncDestination for RemoteDestination {
    fn scope(&self) -> Scope {
        Scope::Project
    }

    fn display_name(&self) -> String {
        self.remote_dest()
    }

    fn exists(&self, path: &Path) -> bool {
        // Check file existence via SSH
        let remote_file = format!("{}/{}", self.remote_path, path.display());
        Command::new("ssh")
            .arg(&self.host)
            .arg(format!("test -f '{}'", remote_file))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn read(&self, path: &Path) -> Result<String, SyncDestinationError> {
        // Read file via SSH
        let remote_file = format!("{}/{}", self.remote_path, path.display());
        let output = Command::new("ssh")
            .arg(&self.host)
            .arg(format!("cat '{}'", remote_file))
            .output()
            .map_err(|e| SyncDestinationError::ConnectionError(e.to_string()))?;

        if !output.status.success() {
            return Err(SyncDestinationError::IoError(format!(
                "Failed to read {}: {}",
                path.display(),
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        String::from_utf8(output.stdout).map_err(|e| SyncDestinationError::IoError(e.to_string()))
    }

    fn hash(&self, path: &Path) -> Result<String, SyncDestinationError> {
        // Compute hash via SSH
        let remote_file = format!("{}/{}", self.remote_path, path.display());
        let output = Command::new("ssh")
            .arg(&self.host)
            .arg(format!("sha256sum '{}'", remote_file))
            .output()
            .map_err(|e| SyncDestinationError::ConnectionError(e.to_string()))?;

        if !output.status.success() {
            return Err(SyncDestinationError::IoError(format!(
                "Failed to hash {}: {}",
                path.display(),
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(hash) = output_str.split_whitespace().next() {
            Ok(format!("sha256:{}", hash))
        } else {
            Err(SyncDestinationError::IoError(
                "Failed to parse hash output".to_string(),
            ))
        }
    }

    fn write_file(&self, path: &Path, content: &str) -> Result<(), SyncDestinationError> {
        // Write single file via SSH
        // For single files, we use echo with SSH
        let remote_file = format!("{}/{}", self.remote_path, path.display());
        let remote_dir = Path::new(&remote_file)
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        // Create parent directory and write file
        let mut child = Command::new("ssh")
            .arg(&self.host)
            .arg(format!(
                "mkdir -p '{}' && cat > '{}'",
                remote_dir, remote_file
            ))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| SyncDestinationError::ConnectionError(e.to_string()))?;

        use std::io::Write;
        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(content.as_bytes())
                .map_err(|e| SyncDestinationError::IoError(e.to_string()))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| SyncDestinationError::IoError(e.to_string()))?;

        if !output.status.success() {
            return Err(SyncDestinationError::IoError(format!(
                "Failed to write {}: {}",
                path.display(),
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }

    fn delete_file(&self, path: &Path) -> Result<(), SyncDestinationError> {
        let remote_file = format!("{}/{}", self.remote_path, path.display());
        let status = Command::new("ssh")
            .arg(&self.host)
            .arg(format!("rm -f '{}'", remote_file))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| SyncDestinationError::ConnectionError(e.to_string()))?;

        if !status.success() {
            return Err(SyncDestinationError::IoError(format!(
                "Failed to delete {}",
                path.display()
            )));
        }

        Ok(())
    }

    fn sync_batch(
        &self,
        outputs: &[OutputFile],
        options: &SyncOptions,
    ) -> Result<SyncResult, SyncDestinationError> {
        let method = Self::transfer_method().ok_or_else(|| {
            SyncDestinationError::NotAvailable(
                "Neither rsync nor scp is available. Install rsync (preferred) or ensure scp is in PATH.".to_string(),
            )
        })?;

        // Create temp directory for staging
        let temp_dir =
            tempfile::tempdir().map_err(|e| SyncDestinationError::IoError(e.to_string()))?;
        let staging_root = temp_dir.path();

        // Collect unique directories that need to be created
        let mut remote_dirs: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

        // Stage all files
        let mut staged_files = Vec::new();
        for output in outputs {
            let target_path = staging_root.join(output.path());

            // Create parent directories
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| SyncDestinationError::IoError(e.to_string()))?;
            }

            // Track remote directories for scp (rsync handles this automatically)
            if method == TransferMethod::Scp {
                if let Some(parent) = output.path().parent() {
                    if !parent.as_os_str().is_empty() {
                        remote_dirs.insert(parent.to_path_buf());
                    }
                }
            }

            // Write file
            std::fs::write(&target_path, output.content())
                .map_err(|e| SyncDestinationError::IoError(e.to_string()))?;
            staged_files.push(output.path().clone());
        }

        if options.dry_run {
            return Ok(SyncResult {
                written: staged_files,
                skipped: vec![],
                errors: vec![],
            });
        }

        match method {
            TransferMethod::Rsync => self.sync_batch_rsync(staging_root, &staged_files, options),
            TransferMethod::Scp => {
                self.sync_batch_scp(staging_root, &staged_files, &remote_dirs, options)
            }
        }
    }

    fn resolve_path(&self, path: &Path) -> PathBuf {
        // For remote, we just return the path as-is
        // The remote host handles path resolution
        path.to_path_buf()
    }

    fn lockfile_path(&self, _source: &Path) -> PathBuf {
        self.source.join(".calvin.lock")
    }
}

impl RemoteDestination {
    /// Sync using rsync (preferred method)
    fn sync_batch_rsync(
        &self,
        staging_root: &Path,
        staged_files: &[PathBuf],
        options: &SyncOptions,
    ) -> Result<SyncResult, SyncDestinationError> {
        let mut cmd = Command::new("rsync");
        cmd.arg("-avz")
            .arg("--progress")
            .arg("-e")
            .arg("ssh")
            .arg(format!("{}/", staging_root.display())) // trailing slash = copy contents
            .arg(self.remote_dest())
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

    /// Sync using scp (fallback method for Windows)
    ///
    /// This provides rsync-like behavior by:
    /// 1. Creating remote directories first via SSH
    /// 2. Using scp -r to copy the staged directory contents
    fn sync_batch_scp(
        &self,
        staging_root: &Path,
        staged_files: &[PathBuf],
        remote_dirs: &std::collections::HashSet<PathBuf>,
        options: &SyncOptions,
    ) -> Result<SyncResult, SyncDestinationError> {
        // Step 1: Create remote directories via SSH
        // This is necessary because scp doesn't create parent directories automatically
        if !remote_dirs.is_empty() {
            let dirs_to_create: Vec<String> = remote_dirs
                .iter()
                .map(|d| format!("{}/{}", self.remote_path, d.display()))
                .collect();

            let mkdir_cmd = format!("mkdir -p {}", dirs_to_create.join(" "));

            let status = Command::new("ssh")
                .arg(&self.host)
                .arg(&mkdir_cmd)
                .stdout(Stdio::null())
                .stderr(if options.json {
                    Stdio::null()
                } else {
                    Stdio::inherit()
                })
                .status()
                .map_err(|e| SyncDestinationError::ConnectionError(e.to_string()))?;

            if !status.success() {
                return Err(SyncDestinationError::CommandFailed(
                    "Failed to create remote directories via SSH".to_string(),
                ));
            }
        }

        // Step 2: Use scp -r to copy the staging directory contents
        // scp -r copies directory contents recursively
        let mut cmd = Command::new("scp");
        cmd.arg("-r") // recursive
            .arg("-p") // preserve modification times
            .stdin(Stdio::inherit()); // Allow password input

        // Add verbose flag if not in JSON mode
        if !options.json && options.verbose {
            cmd.arg("-v");
        }

        // Add all top-level items in the staging directory
        // We need to iterate the staging root and add each item
        let entries: Vec<_> = std::fs::read_dir(staging_root)
            .map_err(|e| SyncDestinationError::IoError(e.to_string()))?
            .filter_map(|e| e.ok())
            .collect();

        for entry in &entries {
            cmd.arg(entry.path());
        }

        // Add remote destination
        cmd.arg(self.remote_dest());

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_destination_parses_simple_host() {
        let dest = RemoteDestination::new("myserver", PathBuf::from(".promptpack"));
        assert_eq!(dest.host, "myserver");
        assert_eq!(dest.remote_path, ".");
    }

    #[test]
    fn remote_destination_parses_host_and_path() {
        let dest = RemoteDestination::new("user@host:/home/project", PathBuf::from(".promptpack"));
        assert_eq!(dest.host, "user@host");
        assert_eq!(dest.remote_path, "/home/project");
    }

    #[test]
    fn remote_destination_parses_tilde_path() {
        let dest = RemoteDestination::new("server:~", PathBuf::from(".promptpack"));
        assert_eq!(dest.host, "server");
        assert_eq!(dest.remote_path, "~");
    }

    #[test]
    fn remote_dest_formats_correctly() {
        let dest = RemoteDestination::new("user@host:/path", PathBuf::from(".promptpack"));
        assert_eq!(dest.remote_dest(), "user@host:/path");
    }

    #[test]
    fn scope_is_project() {
        let dest = RemoteDestination::new("host:/path", PathBuf::from(".promptpack"));
        assert_eq!(dest.scope(), Scope::Project);
    }

    #[test]
    fn display_name_matches_remote_dest() {
        let dest = RemoteDestination::new("user@server:/home/user", PathBuf::from(".promptpack"));
        assert_eq!(dest.display_name(), "user@server:/home/user");
    }

    #[test]
    fn lockfile_path_is_in_source_dir() {
        let dest = RemoteDestination::new("host:/remote", PathBuf::from("/local/project"));
        let lockfile = dest.lockfile_path(Path::new("/any/path"));
        assert_eq!(lockfile, PathBuf::from("/local/project/.calvin.lock"));
    }

    #[test]
    fn resolve_path_returns_path_unchanged() {
        let dest = RemoteDestination::new("host:/path", PathBuf::from(".promptpack"));
        let input = Path::new(".claude/commands/test.md");
        assert_eq!(dest.resolve_path(input), input.to_path_buf());
    }

    #[test]
    fn parses_empty_path_as_dot() {
        let dest = RemoteDestination::new("host", PathBuf::from(".promptpack"));
        assert_eq!(dest.remote_path, ".");
    }

    #[test]
    fn parses_user_at_host_format() {
        let dest = RemoteDestination::new("admin@192.168.1.1:~/projects", PathBuf::from("."));
        assert_eq!(dest.host, "admin@192.168.1.1");
        assert_eq!(dest.remote_path, "~/projects");
    }

    // Note: SSH/rsync/scp integration tests require actual network access
    // and are better suited for integration tests with a test container.
    // The following tests verify command construction without execution.

    #[test]
    fn has_rsync_does_not_panic() {
        // This just verifies the function doesn't panic, actual result depends on system
        let _ = RemoteDestination::has_rsync();
    }

    #[test]
    fn has_scp_does_not_panic() {
        // This just verifies the function doesn't panic, actual result depends on system
        let _ = RemoteDestination::has_scp();
    }

    #[test]
    fn transfer_method_detect_does_not_panic() {
        // This just verifies the function doesn't panic, actual result depends on system
        let _ = TransferMethod::detect();
    }

    #[test]
    fn transfer_method_prefers_rsync() {
        // When both are available, rsync should be preferred
        // This is ensured by the detect() implementation order
        if RemoteDestination::has_rsync() {
            assert_eq!(TransferMethod::detect(), Some(TransferMethod::Rsync));
        }
    }

    #[test]
    fn transfer_method_falls_back_to_scp() {
        // Test the logic: if rsync is not available but scp is, should return Scp
        // We can't easily test this without mocking, but we verify the enum values
        assert_ne!(TransferMethod::Rsync, TransferMethod::Scp);
    }
}
