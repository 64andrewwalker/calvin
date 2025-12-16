//! File watcher for continuous sync
//!
//! Implements the `watch` command with:
//! - Debouncing (100ms)
//! - Incremental compilation
//! - Graceful Ctrl+C shutdown
//! - NDJSON output for CI

use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashSet;

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::error::CalvinResult;
use crate::sync::{compile_assets, sync_outputs, SyncOptions, SyncResult};
use crate::parser::parse_directory;
use crate::models::Target;

/// Debounce duration in milliseconds
const DEBOUNCE_MS: u64 = 100;

/// Watch options
#[derive(Debug, Clone)]
pub struct WatchOptions {
    /// Path to .promptpack directory
    pub source: PathBuf,
    /// Project root (parent of source)
    pub project_root: PathBuf,
    /// Enabled targets
    pub targets: Vec<Target>,
    /// Output as NDJSON
    pub json: bool,
}

/// Watch event types for NDJSON output
#[derive(Debug, Clone)]
pub enum WatchEvent {
    Started { source: String },
    FileChanged { path: String },
    SyncStarted,
    SyncComplete { written: usize, skipped: usize, errors: usize },
    Error { message: String },
    Shutdown,
}

impl WatchEvent {
    pub fn to_json(&self) -> String {
        match self {
            WatchEvent::Started { source } => {
                format!(r#"{{"event":"started","source":"{}"}}"#, source)
            }
            WatchEvent::FileChanged { path } => {
                format!(r#"{{"event":"file_changed","path":"{}"}}"#, path)
            }
            WatchEvent::SyncStarted => {
                r#"{"event":"sync_started"}"#.to_string()
            }
            WatchEvent::SyncComplete { written, skipped, errors } => {
                format!(
                    r#"{{"event":"sync_complete","written":{},"skipped":{},"errors":{}}}"#,
                    written, skipped, errors
                )
            }
            WatchEvent::Error { message } => {
                format!(r#"{{"event":"error","message":"{}"}}"#, message.replace('"', "\\\""))
            }
            WatchEvent::Shutdown => {
                r#"{"event":"shutdown"}"#.to_string()
            }
        }
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
            !self.pending_changes.is_empty() 
                && last.elapsed() >= Duration::from_millis(DEBOUNCE_MS)
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
    event_callback(WatchEvent::Started {
        source: options.source.display().to_string(),
    });
    
    // Do initial sync
    do_sync(&options, &event_callback)?;

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
    ).map_err(|e| crate::error::CalvinError::Io(
        std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
    ))?;

    watcher.watch(&options.source, RecursiveMode::Recursive)
        .map_err(|e| crate::error::CalvinError::Io(
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        ))?;

    // Watch loop with debouncing
    let mut state = WatcherState::new();
    
    while running.load(Ordering::SeqCst) {
        // Check for file changes (non-blocking with timeout)
        if let Ok(path) = rx.recv_timeout(Duration::from_millis(50)) {
            // Only watch .md files
            if path.extension().map(|e| e == "md").unwrap_or(false) {
                event_callback(WatchEvent::FileChanged {
                    path: path.display().to_string(),
                });
                state.add_change(path);
            }
        }

        // Check if we should sync (debounced)
        if state.should_sync() {
            let _changes = state.take_changes();
            do_sync(&options, &event_callback)?;
        }
    }

    event_callback(WatchEvent::Shutdown);
    Ok(())
}

fn do_sync(options: &WatchOptions, callback: &impl Fn(WatchEvent)) -> CalvinResult<()> {
    callback(WatchEvent::SyncStarted);

    let result = match perform_sync(options) {
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

fn perform_sync(options: &WatchOptions) -> CalvinResult<SyncResult> {
    let assets = parse_directory(&options.source)?;
    let outputs = compile_assets(&assets, &options.targets)?;
    
    let sync_options = SyncOptions {
        force: false,
        dry_run: false,
        targets: options.targets.clone(),
    };
    
    sync_outputs(&options.project_root, &outputs, &sync_options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_watch_event_to_json_started() {
        let event = WatchEvent::Started { 
            source: ".promptpack".to_string() 
        };
        let json = event.to_json();
        assert!(json.contains("\"event\":\"started\""));
        assert!(json.contains("\"source\":\".promptpack\""));
    }

    #[test]
    fn test_watch_event_to_json_file_changed() {
        let event = WatchEvent::FileChanged { 
            path: "policies/test.md".to_string() 
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
            errors: 0 
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
            message: "Something \"failed\"".to_string() 
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
            "---\ndescription: Test\n---\n# Content"
        ).unwrap();
        
        let options = WatchOptions {
            source: source.clone(),
            project_root: dir.path().to_path_buf(),
            targets: vec![],
            json: false,
        };
        
        let events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();
        
        let running = Arc::new(AtomicBool::new(false)); // Stop immediately
        
        let _ = watch(options, running, |event| {
            events_clone.lock().unwrap().push(event.to_json());
        });
        
        let captured = events.lock().unwrap();
        assert!(!captured.is_empty());
        assert!(captured[0].contains("started"));
    }
}
