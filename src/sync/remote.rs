//! Optimized remote sync using rsync
//!
//! Instead of multiple SSH calls per file, we:
//! 1. Write all files to a temp directory
//! 2. rsync the temp directory to remote
//! 3. Clean up

use std::process::{Command, Stdio};

use crate::adapters::OutputFile;
use crate::error::{CalvinError, CalvinResult};
use crate::sync::{SyncOptions, SyncResult};

/// Sync outputs to a remote destination using rsync
/// 
/// This is much faster than individual SSH calls because:
/// - rsync uses a single connection with multiplexing
/// - Files are transferred in a batch
/// - Only changed files are transferred (delta sync)
pub fn sync_remote_rsync(
    remote: &str,  // e.g., "ubuntu-server:~" or "user@host:/path"
    outputs: &[OutputFile],
    options: &SyncOptions,
    json: bool,
) -> CalvinResult<SyncResult> {
    // Parse remote destination
    let (host, remote_path) = if let Some((h, p)) = remote.split_once(':') {
        (h, p)
    } else {
        (remote, ".")
    };

    // Create temp directory for staging files
    let temp_dir = tempfile::tempdir()
        .map_err(|e| CalvinError::Io(std::io::Error::other(e)))?;
    
    let staging_root = temp_dir.path();
    
    // Write all files to staging directory
    let mut staged_files = Vec::new();
    for output in outputs {
        let target_path = staging_root.join(&output.path);
        
        // Create parent directories
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Write file
        std::fs::write(&target_path, &output.content)?;
        staged_files.push(output.path.display().to_string());
    }

    if options.dry_run {
        // For dry run, just report what would be synced
        return Ok(SyncResult {
            written: staged_files,
            skipped: vec![],
            errors: vec![],
        });
    }

    // Expand ~ in remote path
    let remote_dest = if remote_path == "~" || remote_path.starts_with("~/") {
        format!("{}:{}", host, remote_path)
    } else if remote_path == "." {
        format!("{}:.", host)
    } else {
        format!("{}:{}", host, remote_path)
    };

    // Build rsync command
    // -a: archive mode (recursive, preserve permissions, etc.)
    // -v: verbose (show progress)
    // -z: compress during transfer
    // --progress: show progress
    // -e ssh: use ssh
    let mut cmd = Command::new("rsync");
    cmd.arg("-avz")
        .arg("--progress")
        .arg("-e").arg("ssh")
        .arg(format!("{}/", staging_root.display())) // trailing slash = copy contents
        .arg(&remote_dest)
        .stdin(Stdio::inherit())  // Allow password input
        .stdout(if json { Stdio::null() } else { Stdio::inherit() })
        .stderr(if json { Stdio::piped() } else { Stdio::inherit() });

    let status = cmd.status()?;

    if !status.success() {
        return Err(CalvinError::Io(std::io::Error::other(format!(
            "rsync failed with exit code: {:?}",
            status.code()
        ))));
    }

    Ok(SyncResult {
        written: staged_files,
        skipped: vec![],
        errors: vec![],
    })
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

/// Sync outputs to a local destination using rsync (fast batch mode)
/// 
/// This is ~10x faster than individual writes when syncing 100+ files
pub fn sync_local_rsync(
    project_root: &std::path::Path,
    outputs: &[OutputFile],
    options: &SyncOptions,
    json: bool,
) -> CalvinResult<SyncResult> {
    // Create temp directory for staging files
    let temp_dir = tempfile::tempdir()
        .map_err(|e| CalvinError::Io(std::io::Error::other(e)))?;
    
    let staging_root = temp_dir.path();
    
    // Write all files to staging directory
    let mut staged_files = Vec::new();
    for output in outputs {
        // Handle paths that start with ~ (user home)
        let output_path_str = output.path.display().to_string();
        let target_path = if output_path_str.starts_with("~") {
            // Skip ~ prefixed paths - they go to different locations
            continue;
        } else {
            staging_root.join(&output.path)
        };
        
        // Create parent directories
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Write file
        std::fs::write(&target_path, &output.content)?;
        staged_files.push(output.path.display().to_string());
    }

    if options.dry_run {
        return Ok(SyncResult {
            written: staged_files,
            skipped: vec![],
            errors: vec![],
        });
    }

    // Early return if no files to transfer
    if staged_files.is_empty() {
        return Ok(SyncResult {
            written: vec![],
            skipped: vec![],
            errors: vec![],
        });
    }

    // Build rsync command for local sync
    // -a: archive mode (recursive, preserve permissions)
    // -v: verbose
    // --delete: remove files in dest not in source (optional, not used here)
    let mut cmd = Command::new("rsync");
    cmd.arg("-av")
        .arg(format!("{}/", staging_root.display())) // trailing slash = copy contents
        .arg(project_root)
        .stdout(if json { Stdio::null() } else { Stdio::inherit() })
        .stderr(if json { Stdio::piped() } else { Stdio::inherit() });

    let status = cmd.status()?;

    if !status.success() {
        return Err(CalvinError::Io(std::io::Error::other(format!(
            "rsync failed with exit code: {:?}",
            status.code()
        ))));
    }

    Ok(SyncResult {
        written: staged_files,
        skipped: vec![],
        errors: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_rsync() {
        // rsync should be available on most Unix systems
        #[cfg(unix)]
        assert!(has_rsync());
    }
}
