//! Sync engine for writing compiled outputs
//!
//! Implements:
//! - TD-14: Atomic writes via tempfile + rename
//! - TD-15: Lockfile system for change detection

pub mod writer;
pub mod lockfile;

use std::path::Path;

use crate::adapters::{all_adapters, OutputFile};
use crate::error::CalvinResult;
use crate::models::{PromptAsset, Target};
use crate::parser::parse_directory;

pub use lockfile::Lockfile;
pub use writer::atomic_write;

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
        let target_path = project_root.join(&output.path);
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
}
