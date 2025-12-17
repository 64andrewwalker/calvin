use std::path::{Path, PathBuf};
use crate::error::CalvinResult;

/// Abstract file system interface
pub trait FileSystem {
    /// Read file content
    fn read_to_string(&self, path: &Path) -> CalvinResult<String>;
    
    /// Write file content atomically
    fn write_atomic(&self, path: &Path, content: &str) -> CalvinResult<()>;
    
    /// Check if file exists
    fn exists(&self, path: &Path) -> bool;
    
    /// Compute SHA256 hash of file content
    fn hash_file(&self, path: &Path) -> CalvinResult<String>;
    
    /// Create directory and parents
    fn create_dir_all(&self, path: &Path) -> CalvinResult<()>;
    
    /// Expand ~ to home directory
    fn expand_home(&self, path: &Path) -> PathBuf;
}

/// Local file system implementation
pub struct LocalFileSystem;

impl FileSystem for LocalFileSystem {
    fn read_to_string(&self, path: &Path) -> CalvinResult<String> {
        Ok(std::fs::read_to_string(path)?)
    }
    
    fn write_atomic(&self, path: &Path, content: &str) -> CalvinResult<()> {
        crate::sync::writer::atomic_write(path, content.as_bytes())
    }
    
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
    
    fn hash_file(&self, path: &Path) -> CalvinResult<String> {
        crate::sync::writer::hash_file(path)
    }
    
    fn create_dir_all(&self, path: &Path) -> CalvinResult<()> {
        Ok(std::fs::create_dir_all(path)?)
    }

    fn expand_home(&self, path: &Path) -> PathBuf {
        crate::sync::expand_home_dir(path)
    }
}

/// Remote file system implementation using SSH
pub struct RemoteFileSystem {
    destination: String,
    /// Cached remote $HOME value
    cached_home: std::sync::Mutex<Option<String>>,
}

impl RemoteFileSystem {
    pub fn new(destination: impl Into<String>) -> Self {
        Self { 
            destination: destination.into(),
            cached_home: std::sync::Mutex::new(None),
        }
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
        // This is safe because we're not passing user input to the shell
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

    fn run_command(&self, command: &str, input: Option<&str>) -> CalvinResult<String> {
        use std::process::{Command, Stdio};
        use std::io::Write;

        let mut child = Command::new("ssh")
            .arg(&self.destination)
            .arg(command)
            .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(crate::error::CalvinError::Io)?;

        if let Some(inp) = input {
            if let Some(mut stdin) = child.stdin.take() {
                if let Err(e) = stdin.write_all(inp.as_bytes()) {
                     return Err(crate::error::CalvinError::Io(e));
                }
            }
        }

        let output = child.wait_with_output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Ignore trivial errors or handle specific checks?
            // For 'test -e', non-zero exit is expected if file missing.
            // But we return error here. 
            // Caller (exists) handles it.
            return Err(crate::error::CalvinError::Io(
                std::io::Error::other(format!("SSH error: {}", stderr))
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
    
    fn quote_path(path: &Path) -> String {
        format!("'{}'", path.to_string_lossy().replace("'", "'\\''"))
    }
}

impl FileSystem for RemoteFileSystem {
    fn read_to_string(&self, path: &Path) -> CalvinResult<String> {
        self.run_command(&format!("cat {}", Self::quote_path(path)), None)
    }

    fn write_atomic(&self, path: &Path, content: &str) -> CalvinResult<()> {
        // Simple cat > file for now. 
        // For atomic, we should write to tmp and rename.
        // mv -f tmp target
        let p = Self::quote_path(path);
        let tmp = format!("{}.tmp", p);
        self.run_command(&format!("cat > {}", tmp), Some(content))?;
        self.run_command(&format!("mv -f {} {}", tmp, p), None)?;
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        self.run_command(&format!("test -e {}", Self::quote_path(path)), None).is_ok()
    }

    fn hash_file(&self, path: &Path) -> CalvinResult<String> {
        // Try sha256sum, fall back to shasum -a 256, fall back to openssl?
        // Using || chain
        let p = Self::quote_path(path);
        let cmd = format!("sha256sum {} 2>/dev/null || shasum -a 256 {} 2>/dev/null", p, p);
        let out = self.run_command(&cmd, None)?;
        Ok(out.split_whitespace().next().unwrap_or("").to_string())
    }

    fn create_dir_all(&self, path: &Path) -> CalvinResult<()> {
        self.run_command(&format!("mkdir -p {}", Self::quote_path(path)), None)?;
        Ok(())
    }

    fn expand_home(&self, path: &Path) -> PathBuf {
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
            // Fallback: if we can't get home, return as-is (will likely fail later)
            // This shouldn't happen in practice if SSH is working
            path.to_path_buf()
        } else {
            path.to_path_buf()
        }
    }
}

/// Mock file system for testing
#[cfg(test)]
pub struct MockFileSystem {
    pub files: std::sync::Mutex<std::collections::HashMap<PathBuf, String>>,
}

#[cfg(test)]
impl MockFileSystem {
    pub fn new() -> Self {
        Self {
            files: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[cfg(test)]
impl FileSystem for MockFileSystem {
    fn read_to_string(&self, path: &Path) -> CalvinResult<String> {
        let files = self.files.lock().unwrap();
        files.get(path)
            .cloned()
            .ok_or_else(|| crate::error::CalvinError::Io(
                std::io::Error::new(std::io::ErrorKind::NotFound, "File not found")
            ))
    }
    
    fn write_atomic(&self, path: &Path, content: &str) -> CalvinResult<()> {
        let mut files = self.files.lock().unwrap();
        files.insert(path.to_path_buf(), content.to_string());
        Ok(())
    }
    
    fn exists(&self, path: &Path) -> bool {
        let files = self.files.lock().unwrap();
        files.contains_key(path)
    }
    
    fn hash_file(&self, path: &Path) -> CalvinResult<String> {
        use sha2::{Sha256, Digest};
        let content = self.read_to_string(path)?;
        let mut hasher = Sha256::new();
        hasher.update(content);
        Ok(format!("{:x}", hasher.finalize()))
    }
    
    fn create_dir_all(&self, _path: &Path) -> CalvinResult<()> {
        Ok(())
    }

    fn expand_home(&self, path: &Path) -> PathBuf {
         let p = path.to_string_lossy();
         if p.starts_with("~/") {
             PathBuf::from("/mock/home").join(p.strip_prefix("~/").unwrap())
         } else if p == "~" {
             PathBuf::from("/mock/home")
         } else {
             path.to_path_buf()
         }
    }
}
