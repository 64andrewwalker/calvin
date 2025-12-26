//! Watch event types and options

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::domain::value_objects::{Scope, Target};

/// Debounce duration in milliseconds
pub const DEBOUNCE_MS: u64 = 100;

/// Watch options
#[derive(Debug, Clone)]
pub struct WatchOptions {
    /// Path to .promptpack directory
    pub source: PathBuf,
    /// Project root (where project-scoped outputs + `calvin.lock` live)
    pub project_root: PathBuf,
    /// Enabled targets
    pub targets: Vec<Target>,
    /// Config
    pub config: Config,
    /// Output as NDJSON
    pub json: bool,
    /// Deploy scope (User = home, Project = local)
    pub scope: Scope,
    /// Watch all resolved layers (user/custom/project)
    pub watch_all_layers: bool,
}

impl WatchOptions {
    /// Create new watch options with minimal required fields
    pub fn new(source: PathBuf, project_root: PathBuf) -> Self {
        Self {
            source,
            project_root,
            targets: Vec::new(),
            config: Config::default(),
            json: false,
            scope: Scope::Project,
            watch_all_layers: false,
        }
    }

    /// Set the deploy scope
    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    /// Set config
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }

    /// Set JSON output mode
    pub fn with_json(mut self, json: bool) -> Self {
        self.json = json;
        self
    }

    /// Set whether to watch all layers
    pub fn with_watch_all_layers(mut self, watch_all_layers: bool) -> Self {
        self.watch_all_layers = watch_all_layers;
        self
    }

    /// Set target filters
    pub fn with_targets(mut self, targets: Vec<Target>) -> Self {
        self.targets = targets;
        self
    }

    /// Check if deploying to home directory
    pub fn deploy_to_home(&self) -> bool {
        self.scope == Scope::User
    }
}

/// Watch event types for NDJSON output
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum WatchEvent {
    /// Watch started
    WatchStarted {
        source: String,
        watch_all_layers: bool,
        watching: Vec<String>,
    },
    /// File changed
    FileChanged { path: String },
    /// Sync started
    SyncStarted,
    /// Sync completed
    SyncComplete {
        written: usize,
        skipped: usize,
        errors: usize,
    },
    /// Error occurred
    Error { message: String },
    /// Watch stopped
    Shutdown,
}

impl WatchEvent {
    /// Convert to JSON string with "command": "watch" field included
    pub fn to_json(&self) -> String {
        // Serialize to Value, add command field, then serialize to string
        let mut value =
            serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({"event": "error"}));
        if let Some(obj) = value.as_object_mut() {
            obj.insert("command".to_string(), serde_json::json!("watch"));
        }
        serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Watcher state for debouncing
#[derive(Debug)]
pub struct WatcherState {
    pending_changes: HashSet<PathBuf>,
    last_change: Option<Instant>,
}

impl Default for WatcherState {
    fn default() -> Self {
        Self::new()
    }
}

impl WatcherState {
    /// Create a new watcher state
    pub fn new() -> Self {
        Self {
            pending_changes: HashSet::new(),
            last_change: None,
        }
    }

    /// Add a file change to pending changes
    pub fn add_change(&mut self, path: PathBuf) {
        self.pending_changes.insert(path);
        self.last_change = Some(Instant::now());
    }

    /// Check if debounce period has passed and we have pending changes
    pub fn should_sync(&self) -> bool {
        if let Some(last) = self.last_change {
            !self.pending_changes.is_empty() && last.elapsed() >= Duration::from_millis(DEBOUNCE_MS)
        } else {
            false
        }
    }

    /// Take all pending changes, resetting state
    pub fn take_changes(&mut self) -> Vec<PathBuf> {
        let changes: Vec<_> = self.pending_changes.drain().collect();
        self.last_change = None;
        changes
    }

    /// Check if there are pending changes
    pub fn has_pending(&self) -> bool {
        !self.pending_changes.is_empty()
    }
}
