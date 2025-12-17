//! Sync engine for writing compiled outputs
//!
//! Implements:
//! - TD-14: Atomic writes via tempfile + rename
//! - TD-15: Lockfile system for change detection

pub mod writer;
pub mod lockfile;
mod compile;
mod conflict;

use std::path::{Path, PathBuf};

use crate::adapters::OutputFile;
use crate::error::{CalvinError, CalvinResult};
use crate::models::Target;
use crate::parser::parse_directory;

pub use lockfile::Lockfile;
pub use writer::atomic_write;
pub use compile::compile_assets;

use conflict::{ConflictChoice, ConflictReason, StdioPrompter, SyncPrompter};

/// Expand ~/ prefix to user home directory
pub fn expand_home_dir(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    if path_str.starts_with("~/") || path_str == "~" {
        if let Some(home) = dirs_home() {
            return home.join(path_str.strip_prefix("~/").unwrap_or(""));
        }
    }
    path.to_path_buf()
}

/// Get home directory (platform-independent)
fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
}

/// Check if a path is safe (doesn't escape project root)
/// 
/// Protects against path traversal attacks like `../../etc/passwd`
pub fn validate_path_safety(path: &Path, project_root: &Path) -> CalvinResult<()> {
    // Skip paths starting with ~ (user-level paths are intentionally outside project)
    let path_str = path.to_string_lossy();
    if path_str.starts_with("~") {
        return Ok(());
    }
    
    // Check for path traversal attempts
    if path_str.contains("..") {
        // Resolve to canonical form and check
        let resolved = project_root.join(path);
        if let Ok(canonical) = resolved.canonicalize() {
            let root_canonical = project_root.canonicalize().unwrap_or_else(|_| project_root.to_path_buf());
            if !canonical.starts_with(&root_canonical) {
                return Err(CalvinError::PathEscape {
                    path: path.to_path_buf(),
                    root: project_root.to_path_buf(),
                });
            }
        }
        // If we can't canonicalize, check for obvious escapes
        else if path_str.starts_with("..") {
            return Err(CalvinError::PathEscape {
                path: path.to_path_buf(),
                root: project_root.to_path_buf(),
            });
        }
    }
    
    Ok(())
}

/// Options for sync operations
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct SyncOptions {
    /// Force overwrite of modified files
    pub force: bool,
    /// Dry run - don't actually write
    pub dry_run: bool,
    /// Prompt for confirmation on conflicts
    pub interactive: bool,
    /// Enabled targets (empty = all)
    pub targets: Vec<Target>,
}


/// Result of a sync operation
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Files written
    pub written: Vec<String>,
    /// Files skipped (modified by user)
    pub skipped: Vec<String>,
    /// Errors encountered
    pub errors: Vec<String>,
}

impl SyncResult {
    pub fn new() -> Self {
        Self {
            written: Vec::new(),
            skipped: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}

impl Default for SyncResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Sync compiled outputs to the filesystem
/// Sync compiled outputs to the filesystem
pub fn sync_outputs(
    project_root: &Path,
    outputs: &[OutputFile],
    options: &SyncOptions,
) -> CalvinResult<SyncResult> {
    sync_with_fs(project_root, outputs, options, &crate::fs::LocalFileSystem)
}

/// Sync compiled outputs using provided file system
pub fn sync_with_fs<FS: crate::fs::FileSystem + ?Sized>(
    project_root: &Path,
    outputs: &[OutputFile],
    options: &SyncOptions,
    fs: &FS,
) -> CalvinResult<SyncResult> {
    let mut prompter = StdioPrompter::new();
    sync_with_fs_with_prompter(project_root, outputs, options, fs, &mut prompter)
}

fn sync_with_fs_with_prompter<FS: crate::fs::FileSystem + ?Sized, P: SyncPrompter>(
    project_root: &Path,
    outputs: &[OutputFile],
    options: &SyncOptions,
    fs: &FS,
    prompter: &mut P,
) -> CalvinResult<SyncResult> {
    let mut result = SyncResult::new();

    // Expand project_root if it contains ~ (for remote deployments)
    let project_root = fs.expand_home(project_root);

    // Load or create lockfile
    let lockfile_path = project_root.join(".promptpack/.calvin.lock");
    let mut lockfile = Lockfile::load_or_new(&lockfile_path, fs);

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ApplyAll {
        Overwrite,
        Skip,
    }
    let mut apply_all: Option<ApplyAll> = None;

    let mut aborted = false;

    'files: for output in outputs {
        // Validate path safety (prevent path traversal attacks)
        validate_path_safety(&output.path, &project_root)?;

        // Expand home directory if needed
        let output_path = fs.expand_home(&output.path);
        let target_path = if output_path.starts_with("~") || output_path.is_absolute() {
            output_path.clone()
        } else {
            project_root.join(&output_path)
        };
        let path_str = output.path.display().to_string();

        // Check if file exists and was modified
        let conflict_reason = if fs.exists(&target_path) {
            if let Some(recorded_hash) = lockfile.get_hash(&path_str) {
                let current_hash = fs.hash_file(&target_path)?;
                if current_hash != recorded_hash {
                    Some(ConflictReason::Modified)
                } else {
                    None
                }
            } else {
                Some(ConflictReason::Untracked)
            }
        } else {
            None
        };

        if let Some(reason) = conflict_reason {
            if !options.force {
                if options.interactive {
                    let mut choice = match apply_all {
                        Some(ApplyAll::Overwrite) => ConflictChoice::Overwrite,
                        Some(ApplyAll::Skip) => ConflictChoice::Skip,
                        None => prompter.prompt_conflict(&path_str, reason),
                    };

                    loop {
                        match choice {
                            ConflictChoice::Diff => {
                                let existing = fs.read_to_string(&target_path).unwrap_or_default();
                                let diff = conflict::unified_diff(&path_str, &existing, &output.content);
                                prompter.show_diff(&diff);
                                choice = prompter.prompt_conflict(&path_str, reason);
                                continue;
                            }
                            ConflictChoice::Skip => {
                                result.skipped.push(path_str.clone());
                                continue 'files;
                            }
                            ConflictChoice::Overwrite => break,
                            ConflictChoice::Abort => {
                                aborted = true;
                                break 'files;
                            }
                            ConflictChoice::OverwriteAll => {
                                apply_all = Some(ApplyAll::Overwrite);
                                break;
                            }
                            ConflictChoice::SkipAll => {
                                apply_all = Some(ApplyAll::Skip);
                                result.skipped.push(path_str.clone());
                                continue 'files;
                            }
                        }
                    }
                } else {
                    result.skipped.push(path_str.clone());
                    continue;
                }
            }
        }

        if options.dry_run {
            result.written.push(path_str.clone());
            continue;
        }

        // Create parent directories
        if let Some(parent) = target_path.parent() {
            fs.create_dir_all(parent)?;
        }

        // Write file atomically (using string content)
        match fs.write_atomic(&target_path, &output.content) {
            Ok(()) => {
                // Update lockfile with new hash
                let new_hash = writer::hash_content(output.content.as_bytes());
                lockfile.set_hash(&path_str, &new_hash);
                result.written.push(path_str);
            }
            Err(e) => {
                result.errors.push(format!("{}: {}", path_str, e));
            }
        }
    }

    // Save lockfile (unless dry run)
    if !options.dry_run {
        lockfile.save(&lockfile_path, fs)?;
    }

    if aborted {
        return Err(CalvinError::SyncAborted);
    }

    Ok(result)
}

/// Full sync pipeline: parse → compile → write
pub fn sync(
    source_dir: &Path,
    project_root: &Path,
    options: &SyncOptions,
    config: &crate::config::Config,
) -> CalvinResult<SyncResult> {
    // Parse source directory
    let assets = parse_directory(source_dir)?;
    
    // Compile to all targets
    let outputs = compile_assets(&assets, &options.targets, config)?;
    
    // Write outputs
    sync_outputs(project_root, &outputs, options)
}

#[cfg(test)]
mod tests;
