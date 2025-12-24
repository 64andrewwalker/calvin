//! Remote Sync Destination
//!
//! Implements SyncDestination for remote servers via SSH.
//! Uses a pluggable transfer strategy (rsync preferred, scp fallback).

mod rsync;
mod scp;
mod transfer;

pub use rsync::RsyncTransfer;
pub use scp::ScpTransfer;
pub use transfer::{detect_strategy, TransferStrategy};

use crate::domain::entities::OutputFile;
use crate::domain::ports::{SyncDestination, SyncDestinationError, SyncOptions, SyncResult};
use crate::domain::value_objects::Scope;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

/// Sync destination for remote servers
///
/// Uses SSH for single file operations and a configurable transfer
/// strategy (rsync or scp) for batch operations.
pub struct RemoteDestination {
    /// Remote host (e.g., "ubuntu-server" or "user@host")
    host: String,
    /// Remote base path (e.g., "~" or "/home/user/project")
    remote_path: String,
    /// Source directory (for lockfile path)
    source: PathBuf,
    /// Cached remote $HOME value (for `~` expansion)
    cached_home: Mutex<Option<String>>,
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
            cached_home: Mutex::new(None),
        }
    }

    /// Build the remote destination string
    fn remote_dest(&self) -> String {
        format!("{}:{}", self.host, self.remote_path)
    }

    fn shell_quote(s: &str) -> String {
        format!("'{}'", s.replace('\'', "'\\''"))
    }

    fn remote_home(&self) -> Option<String> {
        if let Ok(cache) = self.cached_home.lock() {
            if let Some(home) = cache.clone() {
                return Some(home);
            }
        }

        let output = Command::new("ssh")
            .arg(&self.host)
            .arg("echo $HOME")
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let home = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if home.is_empty() {
            return None;
        }
        if let Ok(mut cache) = self.cached_home.lock() {
            *cache = Some(home.clone());
        }
        Some(home)
    }

    fn resolved_remote_path(&self) -> Result<String, SyncDestinationError> {
        let p = self.remote_path.as_str();
        if p != "~" && !p.starts_with("~/") {
            return Ok(self.remote_path.clone());
        }
        let home = self.remote_home().ok_or_else(|| {
            SyncDestinationError::ConnectionError(
                "Failed to resolve remote $HOME for '~' expansion".to_string(),
            )
        })?;
        Ok(if p == "~" {
            home
        } else {
            format!(
                "{}/{}",
                home.trim_end_matches('/'),
                p.trim_start_matches("~/")
            )
        })
    }

    fn remote_file(&self, path: &Path) -> Result<String, SyncDestinationError> {
        let base = self.resolved_remote_path()?;
        Ok(format!("{}/{}", base.trim_end_matches('/'), path.display()))
    }

    /// Stage output files to a temporary directory
    fn stage_files(
        staging_root: &Path,
        outputs: &[OutputFile],
    ) -> Result<Vec<PathBuf>, SyncDestinationError> {
        let mut staged_files = Vec::new();

        for output in outputs {
            let target_path = staging_root.join(output.path());

            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| SyncDestinationError::IoError(e.to_string()))?;
            }

            std::fs::write(&target_path, output.content())
                .map_err(|e| SyncDestinationError::IoError(e.to_string()))?;

            staged_files.push(output.path().clone());
        }

        Ok(staged_files)
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
        let Ok(remote_file) = self.remote_file(path) else {
            return false;
        };
        Command::new("ssh")
            .arg(&self.host)
            .arg(format!("test -f {}", Self::shell_quote(&remote_file)))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn read(&self, path: &Path) -> Result<String, SyncDestinationError> {
        let remote_file = self.remote_file(path)?;
        let output = Command::new("ssh")
            .arg(&self.host)
            .arg(format!("cat {}", Self::shell_quote(&remote_file)))
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
        let remote_file = self.remote_file(path)?;
        let output = Command::new("ssh")
            .arg(&self.host)
            .arg(format!("sha256sum {}", Self::shell_quote(&remote_file)))
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
        let remote_file = self.remote_file(path)?;
        let remote_dir = Path::new(&remote_file)
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        let mut child = Command::new("ssh")
            .arg(&self.host)
            .arg(format!(
                "mkdir -p {} && cat > {}",
                Self::shell_quote(&remote_dir),
                Self::shell_quote(&remote_file)
            ))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| SyncDestinationError::ConnectionError(e.to_string()))?;

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
        let remote_file = self.remote_file(path)?;
        let status = Command::new("ssh")
            .arg(&self.host)
            .arg(format!("rm -f {}", Self::shell_quote(&remote_file)))
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
        // Detect available transfer strategy
        let strategy = detect_strategy().ok_or_else(|| {
            SyncDestinationError::NotAvailable(
                "No transfer method available. Install rsync (preferred) or ensure scp is in PATH."
                    .to_string(),
            )
        })?;

        // Create staging directory
        let temp_dir =
            tempfile::tempdir().map_err(|e| SyncDestinationError::IoError(e.to_string()))?;
        let staging_root = temp_dir.path();

        // Stage all files
        let staged_files = Self::stage_files(staging_root, outputs)?;

        if options.dry_run {
            return Ok(SyncResult {
                written: staged_files,
                skipped: vec![],
                errors: vec![],
            });
        }

        // Execute transfer using detected strategy
        let remote_path = self.resolved_remote_path()?;
        strategy.transfer(
            staging_root,
            &self.host,
            &remote_path,
            &staged_files,
            options,
        )
    }

    fn resolve_path(&self, path: &Path) -> PathBuf {
        path.to_path_buf()
    }

    fn lockfile_path(&self, _source: &Path) -> PathBuf {
        let source_is_promptpack = self
            .source
            .file_name()
            .map(|n| n == ".promptpack")
            .unwrap_or(false);

        let project_root = if source_is_promptpack {
            self.source
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."))
        } else {
            self.source.as_path()
        };

        project_root.join("calvin.lock")
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
        assert_eq!(lockfile, PathBuf::from("/local/project/calvin.lock"));
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
}
