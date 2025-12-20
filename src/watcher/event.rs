//! Watch event types and options

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::models::Target;

/// Debounce duration in milliseconds
pub const DEBOUNCE_MS: u64 = 100;

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
pub(crate) struct WatcherState {
    pub(crate) pending_changes: HashSet<PathBuf>,
    pub(crate) last_change: Option<Instant>,
}

impl WatcherState {
    pub(crate) fn new() -> Self {
        Self {
            pending_changes: HashSet::new(),
            last_change: None,
        }
    }

    pub(crate) fn add_change(&mut self, path: PathBuf) {
        self.pending_changes.insert(path);
        self.last_change = Some(Instant::now());
    }

    pub(crate) fn should_sync(&self) -> bool {
        if let Some(last) = self.last_change {
            !self.pending_changes.is_empty() && last.elapsed() >= Duration::from_millis(DEBOUNCE_MS)
        } else {
            false
        }
    }

    pub(crate) fn take_changes(&mut self) -> Vec<PathBuf> {
        let changes: Vec<_> = self.pending_changes.drain().collect();
        self.last_change = None;
        changes
    }
}
