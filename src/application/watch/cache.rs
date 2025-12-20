//! Incremental cache for efficient reparsing

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use crate::error::CalvinResult;
use crate::models::PromptAsset;
use crate::parser::{parse_directory, parse_file};

/// Cache for incremental compilation
/// Tracks file hashes and parsed assets to avoid unnecessary reparsing
#[derive(Debug, Default)]
pub struct IncrementalCache {
    /// Map of file path to content hash
    pub(crate) file_hashes: HashMap<PathBuf, String>,
    /// Cached parsed assets by file path
    pub(crate) cached_assets: HashMap<PathBuf, PromptAsset>,
}

impl IncrementalCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.file_hashes.is_empty()
    }

    /// Check if a file needs to be reparsed based on content hash
    pub fn needs_reparse(&self, path: &Path, current_hash: &str) -> bool {
        match self.file_hashes.get(path) {
            Some(cached_hash) => cached_hash != current_hash,
            None => true, // New file, needs parsing
        }
    }

    /// Update the cache with a new hash for a file
    pub fn update(&mut self, path: &Path, hash: &str) {
        self.file_hashes
            .insert(path.to_path_buf(), hash.to_string());
    }

    /// Update cache with a parsed asset
    pub fn update_asset(&mut self, path: &Path, hash: &str, asset: PromptAsset) {
        self.file_hashes
            .insert(path.to_path_buf(), hash.to_string());
        self.cached_assets.insert(path.to_path_buf(), asset);
    }

    /// Invalidate cache for a specific file
    pub fn invalidate(&mut self, path: &Path) {
        self.file_hashes.remove(path);
        self.cached_assets.remove(path);
    }

    /// Get all cached assets
    pub fn get_all_assets(&self) -> Vec<&PromptAsset> {
        self.cached_assets.values().collect()
    }

    /// Get cached asset for a file
    pub fn get_asset(&self, path: &Path) -> Option<&PromptAsset> {
        self.cached_assets.get(path)
    }

    /// Get a clone of the file hashes map
    pub fn file_hashes(&self) -> HashMap<PathBuf, String> {
        self.file_hashes.clone()
    }
}

/// Parse files incrementally - only reparse changed files
///
/// If `changed_files` is empty, performs a full parse and populates cache.
/// Otherwise, only reparses files in `changed_files` and returns cached + reparsed assets.
pub fn parse_incremental(
    source_dir: &Path,
    changed_files: &[PathBuf],
    cache: &mut IncrementalCache,
) -> CalvinResult<Vec<PromptAsset>> {
    if changed_files.is_empty() || cache.is_empty() {
        // Full parse - populate cache
        // Clear cache first to avoid stale entries
        cache.file_hashes.clear();
        cache.cached_assets.clear();

        let assets = parse_directory(source_dir)?;
        for asset in &assets {
            // Use absolute path for cache key for consistency
            let abs_path = source_dir.join(&asset.source_path);
            // Canonicalize for consistent path matching with notify events
            let canonical_path = abs_path.canonicalize().unwrap_or(abs_path.clone());

            // IMPORTANT: Use raw file content hash (not parsed content) to match watch loop
            let raw_content = std::fs::read_to_string(&canonical_path).unwrap_or_default();
            let hash = compute_content_hash(&raw_content);
            cache.update_asset(&canonical_path, &hash, asset.clone());
        }
        return Ok(assets);
    }

    // Incremental parse - only reparse changed files
    for path in changed_files {
        if path.extension().map(|e| e == "md").unwrap_or(false) {
            // Skip README.md files (consistent with parse_directory)
            if path.file_name() == Some(std::ffi::OsStr::new("README.md")) {
                continue;
            }

            // Check if file still exists
            if path.exists() {
                if let Ok(mut asset) = parse_file(path) {
                    // Make source_path relative for consistency
                    if let Ok(relative) = path.strip_prefix(source_dir) {
                        asset.source_path = relative.to_path_buf();
                    }
                    // Canonicalize path for consistent cache key
                    let canonical_path = path.canonicalize().unwrap_or(path.clone());
                    // Use raw file content hash (not parsed content) to match watch loop
                    let raw_content = std::fs::read_to_string(&canonical_path).unwrap_or_default();
                    let hash = compute_content_hash(&raw_content);
                    cache.update_asset(&canonical_path, &hash, asset);
                }
            } else {
                // File was deleted - canonicalize for cache key consistency
                let canonical_path = path.canonicalize().unwrap_or(path.clone());
                cache.invalidate(&canonical_path);
            }
        }
    }

    // Return all cached assets, sorted by ID for deterministic output
    // (HashMap iteration order is not stable, which would cause AGENTS.md etc. to have different hashes)
    let mut assets: Vec<_> = cache.cached_assets.values().cloned().collect();
    assets.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(assets)
}

/// Compute a simple hash of content for change detection
pub fn compute_content_hash(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
