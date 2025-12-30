//! Remote File System Implementation
//!
//! Implements the FileSystem port for remote operations via SSH.

use crate::domain::ports::file_system::{FileSystem, FsError, FsResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Remote file system implementation using SSH
///
/// Provides file operations on remote hosts via SSH commands.
/// Caches the remote $HOME directory for efficiency.
pub struct RemoteFs {
    /// SSH destination (user@host or host)
    destination: String,
    /// Cached remote $HOME value
    cached_home: Mutex<Option<String>>,
}

impl RemoteFs {
    /// Create a new RemoteFs for the given SSH destination
    pub fn new(destination: impl Into<String>) -> Self {
        Self {
            destination: destination.into(),
            cached_home: Mutex::new(None),
        }
    }

    /// Get the SSH destination
    pub fn destination(&self) -> &str {
        &self.destination
    }

    /// Get the remote $HOME directory (cached)
    fn get_remote_home(&self) -> Option<String> {
        // Check cache first
        {
            let cache = self.cached_home.lock().unwrap();
            if let Some(ref home) = *cache {
                return Some(home.clone());
            }
        }

        // Fetch from remote via `echo $HOME`
        if let Ok(home) = self.run_command("echo $HOME", None) {
            let home = home.trim().to_string();
            if !home.is_empty() {
                let mut cache = self.cached_home.lock().unwrap();
                *cache = Some(home.clone());
                return Some(home);
            }
        }

        None
    }

    /// Run a command on the remote host via SSH
    fn run_command(&self, command: &str, input: Option<&str>) -> FsResult<String> {
        self.run_command_bytes(command, input.map(|s| s.as_bytes()))
    }

    /// Run a command on the remote host via SSH with binary input
    fn run_command_bytes(&self, command: &str, input: Option<&[u8]>) -> FsResult<String> {
        use std::io::Write;
        use std::process::{Command, Stdio};

        let mut child = Command::new("ssh")
            .arg(&self.destination)
            .arg(command)
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        if let Some(inp) = input {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(inp)?;
            }
        }

        let output = child.wait_with_output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(FsError::Other(format!("SSH error: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Quote a path for safe use in shell commands
    fn quote_path(path: &Path) -> String {
        format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
    }

    /// Batch check multiple files for existence and SHA-256 hash in a single SSH call
    ///
    /// Returns a map of path -> (exists, hash_if_exists)
    /// Hash format: `sha256:<64 hex digits>` (same as lockfile format)
    pub fn batch_check_files(
        &self,
        paths: &[PathBuf],
    ) -> FsResult<HashMap<PathBuf, (bool, Option<String>)>> {
        if paths.is_empty() {
            return Ok(HashMap::new());
        }

        // Build a script that checks all files in one SSH call
        // Output format: one line per file with either:
        //   0 (not exists)
        //   1 <sha256hash> (exists with hash)
        let mut script = String::from("#!/bin/sh\n");
        for path in paths {
            let p = Self::quote_path(path);
            // Check existence, then compute hash if exists
            // Use sha256sum on Linux, shasum on macOS
            script.push_str(&format!(
                "if [ -e {} ]; then h=$(sha256sum {} 2>/dev/null | cut -d' ' -f1 || shasum -a 256 {} 2>/dev/null | cut -d' ' -f1); echo \"1 $h\"; else echo 0; fi\n",
                p, p, p
            ));
        }

        let output = self.run_command("sh", Some(&script))?;

        let mut result = HashMap::new();
        let lines: Vec<&str> = output.lines().collect();

        for (i, path) in paths.iter().enumerate() {
            if let Some(line) = lines.get(i) {
                let trimmed = line.trim();
                if trimmed == "0" {
                    result.insert(path.clone(), (false, None));
                } else if trimmed.starts_with("1 ") {
                    let hash_hex = trimmed.strip_prefix("1 ").unwrap_or("").trim();
                    // Convert to lockfile format: sha256:<hex>
                    let hash = if !hash_hex.is_empty() {
                        Some(format!("sha256:{}", hash_hex))
                    } else {
                        None
                    };
                    result.insert(path.clone(), (true, hash));
                } else {
                    // Unexpected format, assume exists but no hash
                    result.insert(path.clone(), (true, None));
                }
            } else {
                // No output for this file, assume doesn't exist
                result.insert(path.clone(), (false, None));
            }
        }

        Ok(result)
    }
}

impl FileSystem for RemoteFs {
    fn read(&self, path: &Path) -> FsResult<String> {
        self.run_command(&format!("cat {}", Self::quote_path(path)), None)
    }

    fn write(&self, path: &Path, content: &str) -> FsResult<()> {
        let p = Self::quote_path(path);
        let tmp = format!("{}.tmp", p);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            self.create_dir_all(parent)?;
        }

        // Write to temp file then atomically rename
        self.run_command(&format!("cat > {}", tmp), Some(content))?;
        self.run_command(&format!("mv -f {} {}", tmp, p), None)?;
        Ok(())
    }

    fn write_binary(&self, path: &Path, content: &[u8]) -> FsResult<()> {
        let p = Self::quote_path(path);
        let tmp = format!("{}.tmp", p);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            self.create_dir_all(parent)?;
        }

        // Write binary content to temp file via stdin, then atomically rename
        // SSH passes stdin through as binary, so this works for any content
        self.run_command_bytes(&format!("cat > {}", tmp), Some(content))?;
        self.run_command(&format!("mv -f {} {}", tmp, p), None)?;
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        self.run_command(&format!("test -e {}", Self::quote_path(path)), None)
            .is_ok()
    }

    fn remove(&self, path: &Path) -> FsResult<()> {
        let p = Self::quote_path(path);
        if self.run_command(&format!("rm -f {}", p), None).is_ok() {
            return Ok(());
        }
        // Best-effort: allow removing empty directories without introducing `rm -rf`.
        self.run_command(&format!("rmdir {}", p), None)?;
        Ok(())
    }

    fn create_dir_all(&self, path: &Path) -> FsResult<()> {
        self.run_command(&format!("mkdir -p {}", Self::quote_path(path)), None)?;
        Ok(())
    }

    fn hash(&self, path: &Path) -> FsResult<String> {
        // Try sha256sum (Linux), fall back to shasum (macOS)
        let p = Self::quote_path(path);
        let cmd = format!(
            "sha256sum {} 2>/dev/null || shasum -a 256 {} 2>/dev/null",
            p, p
        );
        let out = self.run_command(&cmd, None)?;
        let hash_hex = out.split_whitespace().next().unwrap_or("");
        Ok(format!("sha256:{}", hash_hex))
    }

    fn expand_home(&self, path: &Path) -> PathBuf {
        self.expand_home_internal(path)
    }
}

impl RemoteFs {
    /// Internal expand_home to avoid trait method ambiguity
    fn expand_home_internal(&self, path: &Path) -> PathBuf {
        let p = path.to_string_lossy();

        if p.starts_with("~/") || p == "~" {
            // Get remote $HOME via cached SSH call
            if let Some(home) = self.get_remote_home() {
                if p == "~" {
                    return PathBuf::from(home);
                } else {
                    // ~/foo -> /home/user/foo
                    return PathBuf::from(home).join(p.strip_prefix("~/").unwrap());
                }
            }
            // Fallback: if we can't get home, return as-is
            path.to_path_buf()
        } else {
            path.to_path_buf()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_fs_quote_path_simple() {
        let quoted = RemoteFs::quote_path(Path::new("/home/user/file.txt"));
        assert_eq!(quoted, "'/home/user/file.txt'");
    }

    #[test]
    fn remote_fs_quote_path_with_space() {
        let quoted = RemoteFs::quote_path(Path::new("/home/user/my file.txt"));
        assert_eq!(quoted, "'/home/user/my file.txt'");
    }

    #[test]
    fn remote_fs_quote_path_with_single_quote() {
        let quoted = RemoteFs::quote_path(Path::new("/home/user/it's.txt"));
        assert_eq!(quoted, "'/home/user/it'\\''s.txt'");
    }

    #[test]
    fn remote_fs_new_stores_destination() {
        let fs = RemoteFs::new("user@host");
        assert_eq!(fs.destination(), "user@host");
    }

    #[test]
    fn remote_fs_expand_home_non_tilde() {
        let fs = RemoteFs::new("user@host");
        let path = Path::new("/absolute/path");
        assert_eq!(fs.expand_home(path), PathBuf::from("/absolute/path"));
    }

    // Note: Tests that require actual SSH connections are not included here.
    // Those should be integration tests or require mocking.

    /// Test that write_binary uses binary stdin correctly
    ///
    /// This test is ignored by default because it requires SSH access to localhost.
    /// Run with: cargo test --lib remote_fs_write_binary_localhost -- --ignored
    ///
    /// Prerequisites:
    /// - SSH server running on localhost
    /// - Current user can SSH to localhost without password (ssh-agent or key)
    #[test]
    #[ignore = "requires SSH access to localhost"]
    fn remote_fs_write_binary_localhost() {
        use crate::domain::ports::file_system::FileSystem;
        use tempfile::tempdir;

        let fs = RemoteFs::new("localhost");

        // Create a temp directory to write to
        let dir = tempdir().unwrap();
        let test_path = dir.path().join("test_binary.bin");

        // Binary content with NUL bytes
        let binary_content: [u8; 16] = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52,
        ];

        // Write binary content via SSH
        let result = fs.write_binary(&test_path, &binary_content);
        assert!(result.is_ok(), "write_binary failed: {:?}", result);

        // Verify content was written correctly
        let written_content = std::fs::read(&test_path).expect("Failed to read written file");
        assert_eq!(
            written_content.as_slice(),
            &binary_content[..],
            "Binary content should match exactly"
        );
    }
}
