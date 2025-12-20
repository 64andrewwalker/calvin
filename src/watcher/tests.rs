//! Tests for the watcher module

use super::cache::{parse_incremental, IncrementalCache};
use super::event::{WatchEvent, WatchOptions, WatcherState, DEBOUNCE_MS};
use super::sync::watch;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;
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
