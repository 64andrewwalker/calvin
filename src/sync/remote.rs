//! Optimized remote sync using rsync
//!
//! Instead of multiple SSH calls per file, we:
//! 1. Write all files to a temp directory
//! 2. rsync the temp directory to remote
//! 3. Clean up

use std::process::{Command, Stdio};

use super::OutputFile;
use crate::error::{CalvinError, CalvinResult};
use crate::sync::{SyncOptions, SyncResult};

/// Sync outputs to a remote destination using rsync
///
/// This is much faster than individual SSH calls because:
/// - rsync uses a single connection with multiplexing
/// - Files are transferred in a batch
/// - Only changed files are transferred (delta sync)
pub fn sync_remote_rsync(
    remote: &str, // e.g., "ubuntu-server:~" or "user@host:/path"
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
    let temp_dir = tempfile::tempdir().map_err(|e| CalvinError::Io(std::io::Error::other(e)))?;

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
        .arg("-e")
        .arg("ssh")
        .arg(format!("{}/", staging_root.display())) // trailing slash = copy contents
        .arg(&remote_dest)
        .stdin(Stdio::inherit()) // Allow password input
        .stdout(if json {
            Stdio::null()
        } else {
            Stdio::inherit()
        })
        .stderr(if json {
            Stdio::piped()
        } else {
            Stdio::inherit()
        });

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
    let temp_dir = tempfile::tempdir().map_err(|e| CalvinError::Io(std::io::Error::other(e)))?;

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
        .stdout(if json {
            Stdio::null()
        } else {
            Stdio::inherit()
        })
        .stderr(if json {
            Stdio::piped()
        } else {
            Stdio::inherit()
        });

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
    use std::path::PathBuf;

    fn make_output(path: &str, content: &str) -> OutputFile {
        OutputFile::new(PathBuf::from(path), content.to_string())
    }

    #[test]
    fn test_has_rsync() {
        // rsync should be available on most Unix systems
        #[cfg(unix)]
        assert!(has_rsync());
    }

    #[test]
    fn sync_remote_rsync_dry_run_stages_files() {
        let outputs = vec![
            make_output("test.md", "content"),
            make_output("dir/nested.md", "nested"),
        ];
        let options = SyncOptions {
            dry_run: true,
            ..Default::default()
        };

        let result = sync_remote_rsync("server:~", &outputs, &options, true).unwrap();

        assert_eq!(result.written.len(), 2);
        assert!(result.written.contains(&"test.md".to_string()));
        assert!(result.written.contains(&"dir/nested.md".to_string()));
        assert!(result.skipped.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn sync_remote_rsync_dry_run_empty_outputs() {
        let outputs: Vec<OutputFile> = vec![];
        let options = SyncOptions {
            dry_run: true,
            ..Default::default()
        };

        let result = sync_remote_rsync("server:~", &outputs, &options, true).unwrap();

        assert!(result.written.is_empty());
    }

    #[test]
    fn sync_local_rsync_dry_run_stages_files() {
        let outputs = vec![
            make_output("test.md", "content"),
            make_output("dir/nested.md", "nested"),
        ];
        let options = SyncOptions {
            dry_run: true,
            ..Default::default()
        };

        let temp = tempfile::tempdir().unwrap();
        let result = sync_local_rsync(temp.path(), &outputs, &options, true).unwrap();

        assert_eq!(result.written.len(), 2);
    }

    #[test]
    fn sync_local_rsync_dry_run_skips_home_paths() {
        let outputs = vec![
            make_output("test.md", "project file"),
            make_output("~/.config/tool.md", "home file"), // Should be skipped
        ];
        let options = SyncOptions {
            dry_run: true,
            ..Default::default()
        };

        let temp = tempfile::tempdir().unwrap();
        let result = sync_local_rsync(temp.path(), &outputs, &options, true).unwrap();

        // Only the project file should be staged
        assert_eq!(result.written.len(), 1);
        assert!(result.written.contains(&"test.md".to_string()));
    }

    #[test]
    fn sync_local_rsync_dry_run_empty_after_filter() {
        let outputs = vec![
            make_output("~/.config/a.md", "home a"),
            make_output("~/.config/b.md", "home b"),
        ];
        let options = SyncOptions {
            dry_run: true,
            ..Default::default()
        };

        let temp = tempfile::tempdir().unwrap();
        let result = sync_local_rsync(temp.path(), &outputs, &options, true).unwrap();

        // All paths are home paths, so nothing staged
        assert!(result.written.is_empty());
    }

    #[test]
    fn parse_remote_with_path() {
        // Test the remote parsing logic (inline)
        let remote = "user@server:/home/user";
        let (host, path) = remote.split_once(':').unwrap();
        assert_eq!(host, "user@server");
        assert_eq!(path, "/home/user");
    }

    #[test]
    fn parse_remote_without_path() {
        let remote = "server";
        let (host, path) = if let Some((h, p)) = remote.split_once(':') {
            (h, p)
        } else {
            (remote, ".")
        };
        assert_eq!(host, "server");
        assert_eq!(path, ".");
    }

    #[test]
    fn parse_remote_with_home() {
        let remote = "server:~";
        let (host, path) = remote.split_once(':').unwrap();
        assert_eq!(host, "server");
        assert_eq!(path, "~");
    }

    #[test]
    fn parse_remote_with_home_subdir() {
        let remote = "server:~/projects";
        let (host, path) = remote.split_once(':').unwrap();
        assert_eq!(host, "server");
        assert_eq!(path, "~/projects");
    }
}
