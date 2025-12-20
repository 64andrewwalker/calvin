//! Watch Use Case
//!
//! This module defines the `WatchUseCase` which orchestrates continuous file
//! watching and auto-deploy functionality.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::domain::value_objects::{Scope, Target};

/// Options for the watch operation
#[derive(Debug, Clone)]
pub struct WatchOptions {
    /// Source directory to watch (.promptpack)
    pub source: PathBuf,
    /// Project root directory
    pub project_root: PathBuf,
    /// Target platforms to compile for
    pub targets: Vec<Target>,
    /// Deploy scope (User = home, Project = local)
    pub scope: Scope,
    /// Output JSON events
    pub json: bool,
}

/// Event emitted during watch operation
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// Watch started
    Started { source: PathBuf, target: String },
    /// File changed
    FileChanged { path: PathBuf },
    /// Sync started
    SyncStarted { file_count: usize },
    /// Sync completed
    SyncComplete {
        written: usize,
        skipped: usize,
        errors: usize,
    },
    /// Error occurred
    Error { message: String },
    /// Watch stopped
    Stopped,
}

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
/// This is a higher-level abstraction over the watcher module.
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
    ///
    /// Note: This delegates to the legacy watcher. Full integration pending.
    pub fn start<F>(&self, running: Arc<AtomicBool>, on_event: F)
    where
        F: Fn(WatchEvent) + Clone,
    {
        use crate::watcher::{watch, WatchOptions as LegacyOptions};

        // Emit started event
        on_event(WatchEvent::Started {
            source: self.options.source.clone(),
            target: match self.options.scope {
                Scope::User => "Home".to_string(),
                Scope::Project => "Project".to_string(),
            },
        });

        // Convert to legacy options
        let legacy_options = LegacyOptions {
            source: self.options.source.clone(),
            project_root: self.options.project_root.clone(),
            targets: self
                .options
                .targets
                .iter()
                .map(|t| match t {
                    Target::ClaudeCode => crate::models::Target::ClaudeCode,
                    Target::Cursor => crate::models::Target::Cursor,
                    Target::VSCode => crate::models::Target::VSCode,
                    Target::Antigravity => crate::models::Target::Antigravity,
                    Target::Codex => crate::models::Target::Codex,
                    Target::All => crate::models::Target::All,
                })
                .collect(),
            json: self.options.json,
            config: crate::config::Config::default(),
            deploy_to_home: self.options.scope == Scope::User,
        };

        // Clone callback for use in closure
        let on_event_clone = on_event.clone();

        // Start watching with the legacy watcher
        let _ = watch(legacy_options, running.clone(), move |event| {
            // Convert legacy event to use case event
            let use_case_event = match &event {
                crate::watcher::WatchEvent::WatchStarted { .. } => None, // Already emitted
                crate::watcher::WatchEvent::FileChanged { path, .. } => {
                    Some(WatchEvent::FileChanged {
                        path: PathBuf::from(path),
                    })
                }
                crate::watcher::WatchEvent::SyncStarted => Some(WatchEvent::SyncStarted {
                    file_count: 0, // Legacy doesn't provide this
                }),
                crate::watcher::WatchEvent::SyncComplete {
                    written,
                    skipped,
                    errors,
                } => Some(WatchEvent::SyncComplete {
                    written: *written,
                    skipped: *skipped,
                    errors: *errors,
                }),
                crate::watcher::WatchEvent::Error { message, .. } => Some(WatchEvent::Error {
                    message: message.clone(),
                }),
                crate::watcher::WatchEvent::Shutdown => Some(WatchEvent::Stopped),
            };

            if let Some(event) = use_case_event {
                on_event_clone(event);
            }
        });

        // Check if we should stop
        if !running.load(Ordering::SeqCst) {
            on_event(WatchEvent::Stopped);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watch_options_can_be_created() {
        let options = WatchOptions {
            source: PathBuf::from(".promptpack"),
            project_root: PathBuf::from("."),
            targets: vec![Target::All],
            scope: Scope::User,
            json: false,
        };

        assert_eq!(options.source, PathBuf::from(".promptpack"));
        assert_eq!(options.scope, Scope::User);
    }

    #[test]
    fn watch_event_variants() {
        let started = WatchEvent::Started {
            source: PathBuf::from(".promptpack"),
            target: "Home".to_string(),
        };
        assert!(matches!(started, WatchEvent::Started { .. }));

        let changed = WatchEvent::FileChanged {
            path: PathBuf::from("test.md"),
        };
        assert!(matches!(changed, WatchEvent::FileChanged { .. }));

        let complete = WatchEvent::SyncComplete {
            written: 5,
            skipped: 10,
            errors: 0,
        };
        assert!(matches!(complete, WatchEvent::SyncComplete { .. }));
    }

    #[test]
    fn sync_result_is_success_when_no_errors() {
        let result = SyncResult {
            written: vec![PathBuf::from("a.md")],
            skipped: vec![PathBuf::from("b.md")],
            errors: vec![],
        };
        assert!(result.is_success());
    }

    #[test]
    fn sync_result_not_success_when_errors() {
        let result = SyncResult {
            written: vec![],
            skipped: vec![],
            errors: vec!["error".to_string()],
        };
        assert!(!result.is_success());
    }
}
