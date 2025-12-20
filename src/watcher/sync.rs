//! Watch and sync functionality

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::application::{AssetPipeline, DeployResult};
use crate::domain::policies::ScopePolicy;
use crate::error::CalvinResult;

use super::cache::{compute_content_hash, IncrementalCache};
use super::event::{WatchEvent, WatchOptions, WatcherState};

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
) -> CalvinResult<DeployResult> {
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

    Ok(result)
}
