//! Watch Use Case implementation

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::application::{
    resolve_lockfile_path, AssetPipeline, DeployOutputOptions, DeployResult, DeployUseCase,
};
use crate::domain::policies::ScopePolicy;
use crate::domain::value_objects::Scope;
use crate::error::{CalvinError, CalvinResult};
use crate::infrastructure::adapters::all_adapters;
use crate::infrastructure::fs::LocalFs;
use crate::infrastructure::repositories::{FsAssetRepository, TomlLockfileRepository};

use super::cache::{compute_content_hash, IncrementalCache};
use super::event::{WatchEvent, WatchOptions, WatcherState};

/// Result of a single sync operation
#[derive(Debug, Clone, Default)]
pub struct SyncResult {
    /// Files written
    pub written: Vec<PathBuf>,
    /// Files skipped
    pub skipped: Vec<PathBuf>,
    /// Errors
    pub errors: Vec<String>,
}

impl SyncResult {
    /// Check if sync was successful (no errors)
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Watch Use Case
///
/// Orchestrates continuous file watching with auto-deploy.
/// This is the main entry point for the `calvin watch` command.
pub struct WatchUseCase {
    options: WatchOptions,
}

impl WatchUseCase {
    /// Create a new WatchUseCase
    pub fn new(options: WatchOptions) -> Self {
        Self { options }
    }

    /// Start watching (blocking)
    ///
    /// This method blocks until the running flag is set to false.
    /// Use the callback to receive events.
    pub fn start<F>(&self, running: Arc<AtomicBool>, on_event: F) -> CalvinResult<()>
    where
        F: Fn(WatchEvent),
    {
        // Emit start event
        on_event(WatchEvent::WatchStarted {
            source: self.options.source.display().to_string(),
        });

        // Create incremental cache for efficient reparsing
        let mut cache = IncrementalCache::new();

        // Do initial full sync (populates cache)
        self.do_sync_incremental(&[], &mut cache, &on_event)?;

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
        .map_err(|e| CalvinError::Io(std::io::Error::other(e.to_string())))?;

        watcher
            .watch(&self.options.source, RecursiveMode::Recursive)
            .map_err(|e| CalvinError::Io(std::io::Error::other(e.to_string())))?;

        // Watch loop with debouncing
        let mut state = WatcherState::new();
        // Track content hashes for change detection (separate from parse cache)
        // Pre-populate from initial cache to avoid spurious syncs on startup
        let mut content_hashes: HashMap<PathBuf, String> = cache.file_hashes();

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
                // Only watch .md files, ignore lockfiles and other non-md files
                if path.extension().map(|e| e == "md").unwrap_or(false) {
                    // Skip legacy lockfile (older versions wrote it into `.promptpack/`)
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
                    on_event(WatchEvent::FileChanged {
                        path: path.display().to_string(),
                    });
                }
                // Incremental sync - only reparse changed files
                self.do_sync_incremental(&changes, &mut cache, &on_event)?;
            }
        }

        on_event(WatchEvent::Shutdown);
        Ok(())
    }

    fn do_sync_incremental(
        &self,
        changed_files: &[PathBuf],
        cache: &mut IncrementalCache,
        callback: &impl Fn(WatchEvent),
    ) -> CalvinResult<()> {
        callback(WatchEvent::SyncStarted);

        let result = match self.perform_sync_incremental(changed_files, cache) {
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
        &self,
        changed_files: &[PathBuf],
        cache: &mut IncrementalCache,
    ) -> CalvinResult<DeployResult> {
        let scope_policy = if self.options.deploy_to_home() {
            ScopePolicy::ForceUser
        } else {
            ScopePolicy::Keep
        };

        // Convert domain targets to legacy targets for pipeline
        let legacy_targets: Vec<crate::models::Target> = self
            .options
            .targets
            .iter()
            .map(|t| match t {
                crate::domain::value_objects::Target::ClaudeCode => {
                    crate::models::Target::ClaudeCode
                }
                crate::domain::value_objects::Target::Cursor => crate::models::Target::Cursor,
                crate::domain::value_objects::Target::VSCode => crate::models::Target::VSCode,
                crate::domain::value_objects::Target::Antigravity => {
                    crate::models::Target::Antigravity
                }
                crate::domain::value_objects::Target::Codex => crate::models::Target::Codex,
                crate::domain::value_objects::Target::All => crate::models::Target::All,
            })
            .collect();

        let pipeline = AssetPipeline::new(self.options.source.clone(), self.options.config.clone())
            .with_scope_policy(scope_policy)
            .with_targets(legacy_targets);

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

        // Determine scope and lockfile path
        let scope = if self.options.deploy_to_home() {
            Scope::User
        } else {
            Scope::Project
        };

        let fs = LocalFs::new();
        let lockfile_repo = TomlLockfileRepository::new();
        let project_root = self.options.project_root.clone();

        let (lockfile_path, migration_note) =
            resolve_lockfile_path(&project_root, &self.options.source, &lockfile_repo);

        // Create DeployUseCase
        let asset_repo = FsAssetRepository::new();
        let adapters = all_adapters();
        let deploy = DeployUseCase::new(asset_repo, lockfile_repo, fs, adapters);

        // Deploy outputs directly
        let deploy_options = DeployOutputOptions::new(lockfile_path)
            .with_scope(scope)
            .with_clean_orphans(true);

        let mut result = deploy.deploy_outputs(outputs, &deploy_options);
        if let Some(note) = migration_note {
            result.add_warning(note);
        }

        Ok(result)
    }
}
