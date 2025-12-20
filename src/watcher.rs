//! File watcher for continuous sync
//!
//! Implements the `watch` command with:
//! - Debouncing (100ms)
//! - Incremental compilation (only reparse changed files)
//! - Graceful Ctrl+C shutdown
//! - NDJSON output for CI

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::application::AssetPipeline;
use crate::domain::policies::ScopePolicy;
use crate::error::CalvinResult;
use crate::models::{PromptAsset, Target};
use crate::parser::{parse_directory, parse_file};
use crate::sync::SyncResult;

/// Debounce duration in milliseconds
const DEBOUNCE_MS: u64 = 100;

/// Cache for incremental compilation
/// Tracks file hashes and parsed assets to avoid unnecessary reparsing
#[derive(Debug, Default)]
pub struct IncrementalCache {
    /// Map of file path to content hash
    file_hashes: HashMap<PathBuf, String>,
    /// Cached parsed assets by file path
    cached_assets: HashMap<PathBuf, PromptAsset>,
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
fn compute_content_hash(content: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Watch options
#[derive(Debug, Clone)]
pub struct WatchOptions {
    /// Path to .promptpack directory
    pub source: PathBuf,
    /// Project root (parent of source)
    pub project_root: PathBuf,
    /// Enabled targets
    pub targets: Vec<Target>,
    /// Config
    pub config: crate::config::Config,
    /// Output as NDJSON
    pub json: bool,
    /// Deploy to home directory instead of project root
    pub deploy_to_home: bool,
}

/// Watch event types for NDJSON output
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum WatchEvent {
    WatchStarted {
        source: String,
    },
    FileChanged {
        path: String,
    },
    SyncStarted,
    SyncComplete {
        written: usize,
        skipped: usize,
        errors: usize,
    },
    Error {
        message: String,
    },
    Shutdown,
}

impl WatchEvent {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Watcher state for debouncing
struct WatcherState {
    pending_changes: HashSet<PathBuf>,
    last_change: Option<Instant>,
}

impl WatcherState {
    fn new() -> Self {
        Self {
            pending_changes: HashSet::new(),
            last_change: None,
        }
    }

    fn add_change(&mut self, path: PathBuf) {
        self.pending_changes.insert(path);
        self.last_change = Some(Instant::now());
    }

    fn should_sync(&self) -> bool {
        if let Some(last) = self.last_change {
            !self.pending_changes.is_empty() && last.elapsed() >= Duration::from_millis(DEBOUNCE_MS)
        } else {
            false
        }
    }

    fn take_changes(&mut self) -> Vec<PathBuf> {
        let changes: Vec<_> = self.pending_changes.drain().collect();
        self.last_change = None;
        changes
    }
}

/// Start watching for file changes
pub fn watch(
    options: WatchOptions,
    running: Arc<AtomicBool>,
    event_callback: impl Fn(WatchEvent),
) -> CalvinResult<()> {
    // Initial sync
    event_callback(WatchEvent::WatchStarted {
        source: options.source.display().to_string(),
    });

    // Create incremental cache for efficient reparsing
    let mut cache = IncrementalCache::new();

    // Do initial full sync (populates cache)
    do_sync_incremental(&options, &[], &mut cache, &event_callback)?;

    // Set up file watcher
    let (tx, rx) = channel();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                for path in event.paths {
                    let _ = tx.send(path);
                }
            }
        },
        Config::default(),
    )
    .map_err(|e| crate::error::CalvinError::Io(std::io::Error::other(e.to_string())))?;

    watcher
        .watch(&options.source, RecursiveMode::Recursive)
        .map_err(|e| crate::error::CalvinError::Io(std::io::Error::other(e.to_string())))?;

    // Watch loop with debouncing
    let mut state = WatcherState::new();
    // Track content hashes for change detection (separate from parse cache)
    // Pre-populate from initial cache to avoid spurious syncs on startup
    let mut content_hashes: HashMap<PathBuf, String> = cache.file_hashes.clone();

    // Startup cooldown: drain any initial events from notify (it sometimes sends
    // events for existing files when the watcher is first registered)
    let cooldown_end = Instant::now() + Duration::from_millis(500);
    while Instant::now() < cooldown_end {
        // Drain events but don't process them
        let _ = rx.recv_timeout(Duration::from_millis(50));
    }

    while running.load(Ordering::SeqCst) {
        // Check for file changes (non-blocking with timeout)
        if let Ok(path) = rx.recv_timeout(Duration::from_millis(50)) {
            // Only watch .md files, ignore .calvin.lock and other non-md files
            if path.extension().map(|e| e == "md").unwrap_or(false) {
                // Skip lockfile (it's in .promptpack/ and sync updates it)
                if path
                    .file_name()
                    .map(|n| n == ".calvin.lock")
                    .unwrap_or(false)
                {
                    continue;
                }

                // Canonicalize path to ensure consistent matching with cache
                let canonical_path = path.canonicalize().unwrap_or(path);

                // Check if content actually changed (filter out IDE auto-save noise)
                if let Ok(content) = std::fs::read_to_string(&canonical_path) {
                    let new_hash = compute_content_hash(&content);

                    // Check against our local content tracker
                    let cache_hit = content_hashes.get(&canonical_path);

                    if let Some(old_hash) = cache_hit {
                        if old_hash == &new_hash {
                            // Content unchanged, skip
                            continue;
                        }
                    }

                    // Content changed (or new file), update tracker and queue for sync
                    content_hashes.insert(canonical_path.clone(), new_hash);
                    state.add_change(canonical_path);
                }
            }
        }

        // Check if we should sync (debounced)
        if state.should_sync() {
            let changes = state.take_changes();
            // Report unique changed files after deduplication
            for path in &changes {
                event_callback(WatchEvent::FileChanged {
                    path: path.display().to_string(),
                });
            }
            // Incremental sync - only reparse changed files
            do_sync_incremental(&options, &changes, &mut cache, &event_callback)?;
        }
    }

    event_callback(WatchEvent::Shutdown);
    Ok(())
}

fn do_sync_incremental(
    options: &WatchOptions,
    changed_files: &[PathBuf],
    cache: &mut IncrementalCache,
    callback: &impl Fn(WatchEvent),
) -> CalvinResult<()> {
    callback(WatchEvent::SyncStarted);

    let result = match perform_sync_incremental(options, changed_files, cache) {
        Ok(result) => result,
        Err(e) => {
            callback(WatchEvent::Error {
                message: e.to_string(),
            });
            return Err(e);
        }
    };

    callback(WatchEvent::SyncComplete {
        written: result.written.len(),
        skipped: result.skipped.len(),
        errors: result.errors.len(),
    });

    Ok(())
}

fn perform_sync_incremental(
    options: &WatchOptions,
    changed_files: &[PathBuf],
    cache: &mut IncrementalCache,
) -> CalvinResult<SyncResult> {
    use crate::application::{DeployOutputOptions, DeployUseCase};
    use crate::domain::value_objects::Scope;
    use crate::infrastructure::adapters::all_adapters;
    use crate::infrastructure::fs::LocalFs;
    use crate::infrastructure::repositories::{FsAssetRepository, TomlLockfileRepository};

    let scope_policy = if options.deploy_to_home {
        ScopePolicy::ForceUser
    } else {
        ScopePolicy::Keep
    };

    let pipeline = AssetPipeline::new(options.source.clone(), options.config.clone())
        .with_scope_policy(scope_policy)
        .with_targets(options.targets.clone());

    // Compile assets incrementally
    let legacy_outputs = pipeline.compile_incremental(changed_files, cache)?;

    // Convert legacy OutputFile to domain OutputFile
    let outputs: Vec<crate::domain::entities::OutputFile> = legacy_outputs
        .into_iter()
        .map(|o| {
            crate::domain::entities::OutputFile::new(
                o.path().to_path_buf(),
                o.content().to_string(),
                crate::domain::value_objects::Target::All, // Watch mode outputs to all targets
            )
        })
        .collect();

    // Create DeployUseCase
    let fs = LocalFs::new();
    let lockfile_repo = TomlLockfileRepository::new();
    let asset_repo = FsAssetRepository::new();
    let adapters = all_adapters();

    let deploy = DeployUseCase::new(asset_repo, lockfile_repo, fs, adapters);

    // Determine scope and lockfile path
    let scope = if options.deploy_to_home {
        Scope::User
    } else {
        Scope::Project
    };

    let lockfile_path = options.source.join(".calvin.lock");

    // Deploy outputs directly
    let deploy_options = DeployOutputOptions::new(lockfile_path)
        .with_scope(scope)
        .with_clean_orphans(true);

    let result = deploy.deploy_outputs(outputs, &deploy_options);

    // Convert DeployResult to SyncResult for backward compatibility
    Ok(result.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::tempdir;

    #[test]
    fn test_watch_event_to_json_started() {
        let event = WatchEvent::WatchStarted {
            source: ".promptpack".to_string(),
        };
        let json = event.to_json();
        assert!(json.contains("\"event\":\"watch_started\""));
        assert!(json.contains("\"source\":\".promptpack\""));
    }

    #[test]
    fn test_watch_event_to_json_file_changed() {
        let event = WatchEvent::FileChanged {
            path: "policies/test.md".to_string(),
        };
        let json = event.to_json();
        assert!(json.contains("\"event\":\"file_changed\""));
        assert!(json.contains("\"path\":\"policies/test.md\""));
    }

    #[test]
    fn test_watch_event_to_json_sync_complete() {
        let event = WatchEvent::SyncComplete {
            written: 5,
            skipped: 2,
            errors: 0,
        };
        let json = event.to_json();
        assert!(json.contains("\"event\":\"sync_complete\""));
        assert!(json.contains("\"written\":5"));
        assert!(json.contains("\"skipped\":2"));
        assert!(json.contains("\"errors\":0"));
    }

    #[test]
    fn test_watch_event_to_json_error() {
        let event = WatchEvent::Error {
            message: "Something \"failed\"".to_string(),
        };
        let json = event.to_json();
        assert!(json.contains("\"event\":\"error\""));
        assert!(json.contains("\\\"failed\\\""));
    }

    #[test]
    fn test_watcher_state_debouncing() {
        let mut state = WatcherState::new();

        // No changes yet
        assert!(!state.should_sync());

        // Add a change
        state.add_change(PathBuf::from("test.md"));

        // Should not sync immediately (debounce)
        assert!(!state.should_sync());

        // Wait for debounce period
        std::thread::sleep(Duration::from_millis(DEBOUNCE_MS + 10));

        // Now should sync
        assert!(state.should_sync());

        // Take changes
        let changes = state.take_changes();
        assert_eq!(changes.len(), 1);

        // No more pending
        assert!(!state.should_sync());
    }

    #[test]
    fn test_watcher_state_coalesce_changes() {
        let mut state = WatcherState::new();

        // Add multiple changes to same file
        state.add_change(PathBuf::from("test.md"));
        state.add_change(PathBuf::from("test.md"));
        state.add_change(PathBuf::from("test.md"));

        // Wait for debounce
        std::thread::sleep(Duration::from_millis(DEBOUNCE_MS + 10));

        // Should only have 1 unique change
        let changes = state.take_changes();
        assert_eq!(changes.len(), 1);
    }

    #[test]
    fn test_watcher_state_multiple_files() {
        let mut state = WatcherState::new();

        state.add_change(PathBuf::from("a.md"));
        state.add_change(PathBuf::from("b.md"));
        state.add_change(PathBuf::from("c.md"));

        std::thread::sleep(Duration::from_millis(DEBOUNCE_MS + 10));

        let changes = state.take_changes();
        assert_eq!(changes.len(), 3);
    }

    #[test]
    fn test_watch_initial_sync() {
        let dir = tempdir().unwrap();
        let source = dir.path().join(".promptpack");
        fs::create_dir_all(&source).unwrap();

        // Create a test file
        fs::write(
            source.join("test.md"),
            "---\ndescription: Test\n---\n# Content",
        )
        .unwrap();

        let options = WatchOptions {
            source: source.clone(),
            project_root: dir.path().to_path_buf(),
            targets: vec![],
            config: crate::config::Config::default(),
            json: false,
            deploy_to_home: false,
        };

        let events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();

        let running = Arc::new(AtomicBool::new(false)); // Stop immediately

        let _ = watch(options, running, |event| {
            events_clone.lock().unwrap().push(event.to_json());
        });

        let captured = events.lock().unwrap();
        assert!(!captured.is_empty());
        assert!(captured[0].contains("watch_started"));
    }

    // === TDD: Incremental Compilation Tests (P1 Fix) ===

    #[test]
    fn test_incremental_cache_new() {
        let cache = IncrementalCache::new();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_incremental_cache_update() {
        let mut cache = IncrementalCache::new();
        let path = PathBuf::from("test.md");

        // First update should return true (new file)
        assert!(cache.needs_reparse(&path, "content hash 1"));
        cache.update(&path, "content hash 1");

        // Same hash should not need reparse
        assert!(!cache.needs_reparse(&path, "content hash 1"));

        // Different hash should need reparse
        assert!(cache.needs_reparse(&path, "content hash 2"));
    }

    #[test]
    fn test_incremental_cache_invalidate() {
        let mut cache = IncrementalCache::new();
        let path = PathBuf::from("test.md");

        cache.update(&path, "hash1");
        assert!(!cache.needs_reparse(&path, "hash1"));

        cache.invalidate(&path);
        assert!(cache.needs_reparse(&path, "hash1"));
    }

    #[test]
    fn test_incremental_parse_changed_only() {
        let dir = tempdir().unwrap();
        let source = dir.path().join(".promptpack");
        fs::create_dir_all(&source).unwrap();

        // Create two test files
        fs::write(
            source.join("file1.md"),
            "---\ndescription: File 1\n---\n# Content 1",
        )
        .unwrap();
        fs::write(
            source.join("file2.md"),
            "---\ndescription: File 2\n---\n# Content 2",
        )
        .unwrap();

        let mut cache = IncrementalCache::new();

        // First parse - all files should be parsed
        let changed = vec![]; // Empty means full parse
        let result = parse_incremental(&source, &changed, &mut cache);
        assert!(result.is_ok());
        let assets = result.unwrap();
        assert_eq!(assets.len(), 2);

        // Now only file1 changed
        fs::write(
            source.join("file1.md"),
            "---\ndescription: File 1 Updated\n---\n# New Content",
        )
        .unwrap();

        let changed = vec![source.join("file1.md")];
        let result = parse_incremental(&source, &changed, &mut cache);
        assert!(result.is_ok());
        // Should still return all assets (cached + reparsed)
        let assets = result.unwrap();
        assert_eq!(assets.len(), 2);
    }
}
