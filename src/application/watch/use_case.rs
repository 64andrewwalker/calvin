//! Watch Use Case implementation

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::event::{AccessKind, AccessMode, ModifyKind};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::application::{DeployOptions, DeployResult, DeployUseCase, RegistryUseCase};
use crate::domain::services::LayerResolver;
use crate::error::{CalvinError, CalvinResult};
use crate::infrastructure::fs::LocalFs;
use crate::infrastructure::repositories::{
    FsAssetRepository, TomlLockfileRepository, TomlRegistryRepository,
};

use super::cache::compute_content_hash;
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
        let paths_to_watch = if self.options.watch_all_layers {
            self.resolve_watch_paths()?
        } else {
            vec![self.options.source.clone()]
        };

        on_event(WatchEvent::WatchStarted {
            source: self.options.source.display().to_string(),
            watch_all_layers: self.options.watch_all_layers,
            watching: paths_to_watch
                .iter()
                .map(|p| p.display().to_string())
                .collect(),
        });

        // Set up file watcher
        let (tx, rx) = channel::<Event>();

        let mut _watchers: Vec<RecommendedWatcher> = Vec::new();
        for path in &paths_to_watch {
            let tx = tx.clone();
            let mut watcher = RecommendedWatcher::new(
                move |res: Result<Event, notify::Error>| {
                    if let Ok(event) = res {
                        let _ = tx.send(event);
                    }
                },
                Config::default(),
            )
            .map_err(|e| CalvinError::Io(std::io::Error::other(e.to_string())))?;

            watcher
                .watch(path, RecursiveMode::Recursive)
                .map_err(|e| CalvinError::Io(std::io::Error::other(e.to_string())))?;

            _watchers.push(watcher);
        }

        // Watch loop with debouncing
        let mut state = WatcherState::new();
        // Track content hashes for change detection (filter out IDE auto-save noise).
        let mut content_hashes: HashMap<PathBuf, String> = HashMap::new();

        self.seed_content_hashes(&paths_to_watch, &mut content_hashes);

        self.do_sync(&on_event)?;

        let mut last_poll = Instant::now();

        while running.load(Ordering::SeqCst) {
            // Check for file changes (non-blocking with timeout)
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(event) => {
                    let should_process = match &event.kind {
                        notify::EventKind::Access(AccessKind::Close(AccessMode::Write)) => true,
                        notify::EventKind::Access(_) => false,
                        notify::EventKind::Modify(ModifyKind::Metadata(_)) => false,
                        _ => true,
                    };

                    if should_process {
                        for path in event.paths {
                            // Skip legacy lockfile (older versions wrote it into `.promptpack/`).
                            if path
                                .file_name()
                                .map(|n| n == ".calvin.lock")
                                .unwrap_or(false)
                            {
                                continue;
                            }

                            // Canonicalize when possible to keep a stable key in the hash map.
                            let canonical_path = path.canonicalize().unwrap_or(path);

                            if canonical_path.is_dir() {
                                state.add_change(canonical_path);
                                continue;
                            }

                            // Only watch .md assets + config.toml changes.
                            // Skills are directory-based assets and may include non-.md supplementals (scripts, etc.).
                            let is_md = canonical_path
                                .extension()
                                .map(|e| e == "md")
                                .unwrap_or(false);
                            let is_config = canonical_path
                                .file_name()
                                .map(|n| n == "config.toml")
                                .unwrap_or(false);
                            let is_skill_file =
                                is_path_under_any_skills_dir(&canonical_path, &paths_to_watch);

                            if !(is_md || is_config || is_skill_file) {
                                continue;
                            }

                            if canonical_path.exists() {
                                // If we can read the file, only queue a sync when contents changed.
                                match std::fs::read_to_string(&canonical_path) {
                                    Ok(content) => {
                                        let new_hash = compute_content_hash(&content);
                                        if content_hashes
                                            .get(&canonical_path)
                                            .is_some_and(|old| old == &new_hash)
                                        {
                                            continue;
                                        }
                                        content_hashes.insert(canonical_path.clone(), new_hash);
                                        state.add_change(canonical_path);
                                    }
                                    Err(_) => {
                                        // If the file is mid-write (editor swap), treat as changed.
                                        state.add_change(canonical_path);
                                    }
                                }
                            } else {
                                // Deletions should still trigger a sync so we can remove orphans.
                                content_hashes.remove(&canonical_path);
                                state.add_change(canonical_path);
                            }
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }

            if last_poll.elapsed() >= Duration::from_millis(250) {
                self.poll_for_changes(&paths_to_watch, &mut content_hashes, &mut state);
                last_poll = Instant::now();
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
                // Sync using full multi-layer deploy (PRD §11.4).
                self.do_sync(&on_event)?;
            }
        }

        let _ = _watchers.len();
        on_event(WatchEvent::Shutdown);
        Ok(())
    }

    fn poll_for_changes(
        &self,
        paths: &[PathBuf],
        content_hashes: &mut HashMap<PathBuf, String>,
        state: &mut WatcherState,
    ) {
        let mut seen: HashSet<PathBuf> = HashSet::new();
        for path in paths {
            self.poll_for_changes_in_path(path, path, content_hashes, state, &mut seen);
        }

        let mut deleted: Vec<PathBuf> = Vec::new();
        for path in content_hashes.keys() {
            if !seen.contains(path) && !path.exists() {
                deleted.push(path.clone());
            }
        }

        for path in deleted {
            content_hashes.remove(&path);
            state.add_change(path);
        }
    }

    fn poll_for_changes_in_path(
        &self,
        root: &Path,
        path: &Path,
        content_hashes: &mut HashMap<PathBuf, String>,
        state: &mut WatcherState,
        seen: &mut HashSet<PathBuf>,
    ) {
        if path.is_dir() {
            let Ok(entries) = std::fs::read_dir(path) else {
                return;
            };
            for entry in entries.flatten() {
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                if file_type.is_symlink() {
                    continue;
                }
                let child_path = entry.path();
                self.poll_for_changes_in_path(root, &child_path, content_hashes, state, seen);
            }
            return;
        }

        if !path.is_file() {
            return;
        }

        let is_md = path.extension().map(|e| e == "md").unwrap_or(false);
        let is_config = path
            .file_name()
            .map(|n| n == "config.toml")
            .unwrap_or(false);
        let is_skill_file = is_path_under_skills_dir(path, root);
        if !(is_md || is_config || is_skill_file) {
            return;
        }

        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        seen.insert(canonical_path.clone());

        let Ok(content) = std::fs::read_to_string(&canonical_path) else {
            return;
        };

        let new_hash = compute_content_hash(&content);
        if content_hashes
            .get(&canonical_path)
            .is_some_and(|old| old == &new_hash)
        {
            return;
        }

        content_hashes.insert(canonical_path.clone(), new_hash);
        state.add_change(canonical_path);
    }

    fn do_sync(&self, callback: &impl Fn(WatchEvent)) -> CalvinResult<()> {
        callback(WatchEvent::SyncStarted);

        let result = self.perform_sync();

        callback(WatchEvent::SyncComplete {
            written: result.written.len(),
            skipped: result.skipped.len(),
            errors: result.errors.len(),
        });

        Ok(())
    }

    fn perform_sync(&self) -> DeployResult {
        let use_project_layer = !self.options.config.sources.disable_project_layer;
        let use_user_layer = self.options.config.sources.use_user_layer
            && !self.options.config.sources.ignore_user_layer;
        let additional_layers = if self.options.config.sources.ignore_additional_layers {
            Vec::new()
        } else {
            self.options.config.sources.additional_layers.clone()
        };
        let use_additional_layers = !self.options.config.sources.ignore_additional_layers;

        let mut deploy_options = DeployOptions::new(self.options.source.clone())
            .with_project_root(self.options.project_root.clone())
            .with_project_layer_enabled(use_project_layer)
            .with_user_layer_enabled(use_user_layer)
            .with_additional_layers(additional_layers)
            .with_additional_layers_enabled(use_additional_layers)
            .with_scope(self.options.scope)
            .with_targets(self.options.targets.clone())
            .with_clean_orphans(true);
        if let Some(path) = self.options.config.sources.user_layer_path.clone() {
            deploy_options = deploy_options.with_user_layer_path(path);
        }

        let registry_repo = Arc::new(TomlRegistryRepository::new());
        let registry_use_case = Arc::new(RegistryUseCase::new(registry_repo));

        let fs = LocalFs::new();
        let lockfile_repo = TomlLockfileRepository::new();
        let asset_repo = FsAssetRepository::new();
        let adapters = crate::infrastructure::adapters::all_adapters();

        DeployUseCase::new(asset_repo, lockfile_repo, fs, adapters)
            .with_registry_use_case(registry_use_case)
            .execute(&deploy_options)
    }

    fn resolve_watch_paths(&self) -> CalvinResult<Vec<PathBuf>> {
        use crate::domain::services::LayerResolveError;

        let project_root = self.options.project_root.clone();
        let project_layer_path = if self.options.source.is_relative() {
            project_root.join(&self.options.source)
        } else {
            self.options.source.clone()
        };

        let mut resolver = LayerResolver::new(project_root)
            .with_project_layer_path(project_layer_path)
            .with_disable_project_layer(self.options.config.sources.disable_project_layer)
            .with_additional_layers(if self.options.config.sources.ignore_additional_layers {
                Vec::new()
            } else {
                self.options.config.sources.additional_layers.clone()
            })
            .with_remote_mode(false);

        let use_user_layer = self.options.config.sources.use_user_layer
            && !self.options.config.sources.ignore_user_layer;
        if use_user_layer {
            let user_layer_path = self
                .options
                .config
                .sources
                .user_layer_path
                .clone()
                .unwrap_or_else(crate::config::default_user_layer_path);
            resolver = resolver.with_user_layer_path(user_layer_path);
        }

        let resolution = resolver.resolve().map_err(|e| match e {
            LayerResolveError::NoLayersFound => CalvinError::NoLayersFound,
            LayerResolveError::PathNotFound { path } => CalvinError::DirectoryNotFound { path },
            _ => CalvinError::Io(std::io::Error::other(e.to_string())),
        })?;

        let mut paths: Vec<PathBuf> = resolution
            .layers
            .into_iter()
            .map(|layer| layer.path.resolved().to_path_buf())
            .collect();
        paths.sort();
        paths.dedup();
        Ok(paths)
    }

    fn seed_content_hashes(
        &self,
        paths: &[PathBuf],
        content_hashes: &mut HashMap<PathBuf, String>,
    ) {
        for path in paths {
            self.seed_content_hashes_for_path(path, path, content_hashes);
        }
    }

    fn seed_content_hashes_for_path(
        &self,
        root: &Path,
        path: &Path,
        content_hashes: &mut HashMap<PathBuf, String>,
    ) {
        if path.is_dir() {
            let Ok(entries) = std::fs::read_dir(path) else {
                return;
            };
            for entry in entries.flatten() {
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                if file_type.is_symlink() {
                    continue;
                }
                self.seed_content_hashes_for_path(root, &entry.path(), content_hashes);
            }
            return;
        }

        if !path.is_file() {
            return;
        }

        let is_md = path.extension().map(|e| e == "md").unwrap_or(false);
        let is_config = path
            .file_name()
            .map(|n| n == "config.toml")
            .unwrap_or(false);
        let is_skill_file = is_path_under_skills_dir(path, root);
        if !(is_md || is_config || is_skill_file) {
            return;
        }

        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let Ok(content) = std::fs::read_to_string(&canonical_path) else {
            return;
        };
        content_hashes.insert(canonical_path, compute_content_hash(&content));
    }
}

fn is_path_under_any_skills_dir(path: &Path, roots: &[PathBuf]) -> bool {
    roots
        .iter()
        .any(|root| is_path_under_skills_dir(path, root))
}

fn is_path_under_skills_dir(path: &Path, root: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(root) else {
        return false;
    };
    rel.components()
        .next()
        .is_some_and(|c| c.as_os_str() == std::ffi::OsStr::new("skills"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_directory_changes_are_considered_relevant() {
        let root = PathBuf::from(".promptpack");
        let script = root.join("skills/my-skill/scripts/validate.py");
        let markdown = root.join("skills/my-skill/reference.md");
        let policy = root.join("policies/style.md");

        assert!(is_path_under_skills_dir(&script, &root));
        assert!(is_path_under_skills_dir(&markdown, &root));
        assert!(!is_path_under_skills_dir(&policy, &root));
    }
}
