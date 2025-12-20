//! Remote Sync Destination
//!
//! Implements SyncDestination for remote servers via SSH/rsync.
//! Uses rsync for efficient batch transfers.

use crate::domain::entities::OutputFile;
use crate::domain::ports::{SyncDestination, SyncDestinationError, SyncOptions, SyncResult};
use crate::domain::value_objects::Scope;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
        if !Self::has_rsync() {
            return Err(SyncDestinationError::NotAvailable(
                "rsync is not installed".to_string(),
            ));
        }

        // Create temp directory for staging
        let temp_dir =
            tempfile::tempdir().map_err(|e| SyncDestinationError::IoError(e.to_string()))?;
        let staging_root = temp_dir.path();

        // Stage all files
        let mut staged_files = Vec::new();
        for output in outputs {
            let target_path = staging_root.join(output.path());

            // Create parent directories
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| SyncDestinationError::IoError(e.to_string()))?;
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

        // Build rsync command
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
            written: staged_files,
            skipped: vec![],
            errors: vec![],
        })
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
}
