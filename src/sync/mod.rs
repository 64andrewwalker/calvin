//! Sync engine for writing compiled outputs
//!
//! Implements:
//! - TD-14: Atomic writes via tempfile + rename
//! - TD-15: Lockfile system for change detection

pub mod writer;
pub mod lockfile;

use std::path::{Path, PathBuf};

use crate::adapters::{all_adapters, OutputFile};
use crate::error::{CalvinError, CalvinResult};
use crate::models::{PromptAsset, Target};
use crate::parser::parse_directory;

pub use lockfile::Lockfile;
pub use writer::atomic_write;

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
pub struct SyncOptions {
    /// Force overwrite of modified files
    pub force: bool,
    /// Dry run - don't actually write
    pub dry_run: bool,
    /// Enabled targets (empty = all)
    pub targets: Vec<Target>,
}

impl Default for SyncOptions {
    fn default() -> Self {
        Self {
            force: false,
            dry_run: false,
            targets: Vec::new(),
        }
    }
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

/// Compile all assets to target platform outputs
pub fn compile_assets(
    assets: &[PromptAsset],
    targets: &[Target],
) -> CalvinResult<Vec<OutputFile>> {
    let mut outputs = Vec::new();
    
    let adapters = all_adapters();
    
    for asset in assets {
        // Get effective targets for this asset
        let effective_targets = asset.frontmatter.effective_targets();
        
        for adapter in &adapters {
            let adapter_target = adapter.target();
            
            // Skip if target not enabled for this asset
            if !effective_targets.contains(&adapter_target) {
                continue;
            }
            
            // Skip if not in requested targets list (if specified)
            if !targets.is_empty() && !targets.contains(&adapter_target) {
                continue;
            }
            
            // Compile asset with this adapter
            let files = adapter.compile(asset)?;
            outputs.extend(files);
        }
    }
    
    // Sort for deterministic output
    outputs.sort_by(|a, b| a.path.cmp(&b.path));
    
    Ok(outputs)
}

/// Sync compiled outputs to the filesystem
pub fn sync_outputs(
    project_root: &Path,
    outputs: &[OutputFile],
    options: &SyncOptions,
) -> CalvinResult<SyncResult> {
    let mut result = SyncResult::new();
    
    // Load or create lockfile
    let lockfile_path = project_root.join(".promptpack/.calvin.lock");
    let mut lockfile = Lockfile::load_or_new(&lockfile_path);
    
    for output in outputs {
        // Validate path safety (prevent path traversal attacks)
        validate_path_safety(&output.path, project_root)?;
        
        // Expand home directory if needed
        let output_path = expand_home_dir(&output.path);
        let target_path = if output_path.starts_with("~") || output_path.is_absolute() {
            output_path.clone()
        } else {
            project_root.join(&output_path)
        };
        let path_str = output.path.display().to_string();
        
        // Check if file exists and was modified
        if target_path.exists() {
            if let Some(recorded_hash) = lockfile.get_hash(&path_str) {
                // Check current file hash
                let current_hash = writer::hash_file(&target_path)?;
                
                if current_hash != recorded_hash && !options.force {
                    // File was modified by user, skip
                    result.skipped.push(path_str.clone());
                    continue;
                }
            } else {
                // File exists but not in lockfile - user created it
                if !options.force {
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
            std::fs::create_dir_all(parent)?;
        }
        
        // Write file atomically
        match writer::atomic_write(&target_path, output.content.as_bytes()) {
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
        lockfile.save(&lockfile_path)?;
    }
    
    Ok(result)
}

/// Full sync pipeline: parse → compile → write
pub fn sync(
    source_dir: &Path,
    project_root: &Path,
    options: &SyncOptions,
) -> CalvinResult<SyncResult> {
    // Parse source directory
    let assets = parse_directory(source_dir)?;
    
    // Compile to all targets
    let outputs = compile_assets(&assets, &options.targets)?;
    
    // Write outputs
    sync_outputs(project_root, &outputs, options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Frontmatter;

    #[test]
    fn test_sync_options_default() {
        let opts = SyncOptions::default();
        assert!(!opts.force);
        assert!(!opts.dry_run);
        assert!(opts.targets.is_empty());
    }

    #[test]
    fn test_sync_result_new() {
        let result = SyncResult::new();
        assert!(result.written.is_empty());
        assert!(result.skipped.is_empty());
        assert!(result.errors.is_empty());
        assert!(result.is_success());
    }

    #[test]
    fn test_compile_assets_single() {
        let fm = Frontmatter::new("Test asset");
        let asset = PromptAsset::new("test", "test.md", fm, "Content");
        
        let outputs = compile_assets(&[asset], &[]).unwrap();
        
        // Should generate output for all 5 adapters
        assert!(!outputs.is_empty());
    }

    #[test]
    fn test_compile_assets_target_filter() {
        let fm = Frontmatter::new("Test asset");
        let asset = PromptAsset::new("test", "test.md", fm, "Content");
        
        let outputs = compile_assets(&[asset], &[Target::ClaudeCode]).unwrap();
        
        // Should only generate Claude Code output
        assert!(outputs.iter().all(|o| o.path.starts_with(".claude")));
    }

    #[test]
    fn test_validate_path_safety_normal() {
        let root = Path::new("/project");
        assert!(validate_path_safety(Path::new(".claude/settings.json"), root).is_ok());
        assert!(validate_path_safety(Path::new(".cursor/rules/test/RULE.md"), root).is_ok());
    }

    #[test]
    fn test_validate_path_safety_user_paths() {
        let root = Path::new("/project");
        // User-level paths starting with ~ are always allowed
        assert!(validate_path_safety(Path::new("~/.codex/prompts/test.md"), root).is_ok());
        assert!(validate_path_safety(Path::new("~/.claude/commands/test.md"), root).is_ok());
    }

    #[test]
    fn test_validate_path_safety_traversal() {
        let root = Path::new("/project");
        // Path traversal should be blocked
        assert!(validate_path_safety(Path::new("../etc/passwd"), root).is_err());
        assert!(validate_path_safety(Path::new("../../malicious"), root).is_err());
    }

    #[test]
    fn test_expand_home_dir() {
        // Test that ~ is expanded (if HOME is set)
        if std::env::var("HOME").is_ok() {
            let expanded = expand_home_dir(Path::new("~/.codex/prompts"));
            assert!(!expanded.to_string_lossy().starts_with("~"));
            assert!(expanded.to_string_lossy().contains(".codex/prompts"));
        }
        
        // Non-home paths should pass through unchanged
        let unchanged = expand_home_dir(Path::new(".claude/settings.json"));
        assert_eq!(unchanged, Path::new(".claude/settings.json"));
    }
}
