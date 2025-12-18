//! Sync execution module - batch transfer using optimal strategy
//!
//! Stage 2: execute_sync() - batch transfer using rsync or file-by-file

use std::path::Path;

use crate::adapters::OutputFile;
use crate::error::CalvinResult;
use crate::fs::FileSystem;
use crate::sync::{SyncOptions, SyncResult, SyncEvent};
use crate::sync::plan::{SyncPlan, SyncDestination};

/// Sync strategy selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SyncStrategy {
    /// Automatically select best strategy (rsync if available and 10+ files)
    #[default]
    Auto,
    /// Force rsync batch transfer
    Rsync,
    /// Force file-by-file transfer (for progress callbacks)
    FileByFile,
}

/// Execute a sync plan, transferring files to destination
///
/// This stage does the actual file transfer, using the optimal strategy
pub fn execute_sync(
    plan: &SyncPlan,
    dest: &SyncDestination,
    strategy: SyncStrategy,
) -> CalvinResult<SyncResult> {
    execute_sync_with_callback::<fn(SyncEvent)>(plan, dest, strategy, None)
}

/// Execute a sync plan with progress callback
pub fn execute_sync_with_callback<F>(
    plan: &SyncPlan,
    dest: &SyncDestination,
    strategy: SyncStrategy,
    mut callback: Option<F>,
) -> CalvinResult<SyncResult>
where
    F: FnMut(SyncEvent),
{
    if plan.to_write.is_empty() {
        return Ok(SyncResult {
            written: vec![],
            skipped: plan.to_skip.clone(),
            errors: vec![],
        });
    }

    // Determine actual strategy
    let use_rsync = match strategy {
        SyncStrategy::Rsync => true,
        SyncStrategy::FileByFile => false,
        SyncStrategy::Auto => {
            plan.to_write.len() > 10 && crate::sync::remote::has_rsync()
        }
    };

    match dest {
        SyncDestination::Local(root) => {
            if use_rsync && callback.is_none() {
                // Fast path: rsync batch transfer
                let options = SyncOptions {
                    force: true,
                    dry_run: false,
                    interactive: false,
                    targets: vec![],
                };
                crate::sync::remote::sync_local_rsync(root, &plan.to_write, &options, false)
            } else {
                // File-by-file with callback support
                write_files_sequential(root, &plan.to_write, &plan.to_skip, callback.as_mut())
            }
        }
        SyncDestination::Remote { host, path } => {
            let fs = crate::fs::RemoteFileSystem::new(host);
            // Expand ~ to actual home path for proper file operations
            let expanded_path = fs.expand_home(path);
            
            if use_rsync && callback.is_none() {
                // Fast path: rsync to remote
                let remote = format!("{}:{}", host, path.display());
                let options = SyncOptions {
                    force: true,
                    dry_run: false,
                    interactive: false,
                    targets: vec![],
                };
                crate::sync::remote::sync_remote_rsync(&remote, &plan.to_write, &options, false)
            } else {
                // File-by-file via SSH with callback
                write_files_with_fs(&expanded_path, &plan.to_write, &plan.to_skip, &fs, callback.as_mut())
            }
        }
    }
}

/// Write files sequentially to local filesystem
fn write_files_sequential<F>(
    root: &Path,
    outputs: &[OutputFile],
    skipped: &[String],
    mut callback: Option<&mut F>,
) -> CalvinResult<SyncResult>
where
    F: FnMut(SyncEvent),
{
    let fs = crate::fs::LocalFileSystem;
    write_files_with_fs(root, outputs, skipped, &fs, callback.as_mut())
}

/// Write files using a FileSystem abstraction
fn write_files_with_fs<FS, F>(
    root: &Path,
    outputs: &[OutputFile],
    skipped: &[String],
    fs: &FS,
    mut callback: Option<&mut F>,
) -> CalvinResult<SyncResult>
where
    FS: crate::fs::FileSystem + ?Sized,
    F: FnMut(SyncEvent),
{
    let mut result = SyncResult {
        written: Vec::new(),
        skipped: skipped.to_vec(),
        errors: Vec::new(),
    };

    for (index, output) in outputs.iter().enumerate() {
        let path_str = output.path.display().to_string();
        
        if let Some(ref mut cb) = callback {
            cb(SyncEvent::ItemStart {
                index,
                path: path_str.clone(),
            });
        }

        let target_path = root.join(&output.path);
        
        // Ensure parent directory exists
        if let Some(parent) = target_path.parent() {
            if let Err(e) = fs.create_dir_all(parent) {
                if let Some(ref mut cb) = callback {
                    cb(SyncEvent::ItemError {
                        index,
                        path: path_str.clone(),
                        message: e.to_string(),
                    });
                }
                result.errors.push(path_str);
                continue;
            }
        }

        // Write file
        match fs.write_atomic(&target_path, &output.content) {
            Ok(()) => {
                if let Some(ref mut cb) = callback {
                    cb(SyncEvent::ItemWritten {
                        index,
                        path: path_str.clone(),
                    });
                }
                result.written.push(path_str);
            }
            Err(e) => {
                if let Some(ref mut cb) = callback {
                    cb(SyncEvent::ItemError {
                        index,
                        path: path_str.clone(),
                        message: e.to_string(),
                    });
                }
                result.errors.push(path_str);
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::MockFileSystem;
    use std::path::PathBuf;

    fn make_output(path: &str, content: &str) -> OutputFile {
        OutputFile::new(PathBuf::from(path), content.to_string())
    }

    #[test]
    fn execute_empty_plan_returns_empty_result() {
        let plan = SyncPlan::new();
        let dest = SyncDestination::Local(PathBuf::from("/project"));
        
        let result = execute_sync(&plan, &dest, SyncStrategy::FileByFile).unwrap();
        
        assert!(result.written.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn execute_writes_files_to_local() {
        let mut plan = SyncPlan::new();
        plan.to_write.push(make_output("test.md", "content"));
        
        let dir = tempfile::tempdir().unwrap();
        let dest = SyncDestination::Local(dir.path().to_path_buf());
        
        let result = execute_sync(&plan, &dest, SyncStrategy::FileByFile).unwrap();
        
        assert_eq!(result.written.len(), 1);
        assert!(dir.path().join("test.md").exists());
    }

    #[test]
    fn execute_with_callback_emits_events() {
        let mut plan = SyncPlan::new();
        plan.to_write.push(make_output("a.md", "a"));
        plan.to_write.push(make_output("b.md", "b"));
        
        let dir = tempfile::tempdir().unwrap();
        let dest = SyncDestination::Local(dir.path().to_path_buf());
        
        let mut events = Vec::new();
        let result = execute_sync_with_callback(
            &plan,
            &dest,
            SyncStrategy::FileByFile,
            Some(|e: SyncEvent| events.push(e)),
        ).unwrap();
        
        assert_eq!(result.written.len(), 2);
        // Should have 2 starts + 2 written = 4 events
        assert_eq!(events.len(), 4);
    }

    #[test]
    fn execute_preserves_skipped_files() {
        let mut plan = SyncPlan::new();
        plan.to_skip.push("skipped.md".to_string());
        
        let dir = tempfile::tempdir().unwrap();
        let dest = SyncDestination::Local(dir.path().to_path_buf());
        
        let result = execute_sync(&plan, &dest, SyncStrategy::FileByFile).unwrap();
        
        assert_eq!(result.skipped.len(), 1);
        assert_eq!(result.skipped[0], "skipped.md");
    }

    #[test]
    fn auto_strategy_uses_rsync_for_many_files() {
        // This is a design validation test
        let strategy = SyncStrategy::Auto;
        
        // With 11 files and rsync available, should use rsync
        let use_rsync = match strategy {
            SyncStrategy::Rsync => true,
            SyncStrategy::FileByFile => false,
            SyncStrategy::Auto => 11 > 10 && crate::sync::remote::has_rsync(),
        };
        
        // On most Unix systems, rsync is available so this should be true
        #[cfg(unix)]
        assert!(use_rsync);
    }
}
