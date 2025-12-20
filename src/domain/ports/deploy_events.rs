//! Deploy Event Port
//!
//! Provides an observable interface for deploy operations.
//! Enables progress reporting, JSON event streams, and debugging.

use std::path::PathBuf;

/// Event emitted during deploy operations
#[derive(Debug, Clone)]
pub enum DeployEvent {
    /// Deploy started
    Started {
        source: PathBuf,
        destination: String,
        asset_count: usize,
    },

    /// Compilation completed
    Compiled { output_count: usize },

    /// File sync started
    FileStarted { index: usize, path: PathBuf },

    /// File was written successfully
    FileWritten { index: usize, path: PathBuf },

    /// File was skipped (up-to-date or conflict)
    FileSkipped {
        index: usize,
        path: PathBuf,
        reason: String,
    },

    /// File sync failed
    FileError {
        index: usize,
        path: PathBuf,
        error: String,
    },

    /// Orphan files detected (summary)
    OrphansDetected { total: usize, safe_to_delete: usize },

    /// Orphan file deleted
    OrphanDeleted { path: PathBuf },

    /// Deploy completed
    Completed {
        written_count: usize,
        skipped_count: usize,
        error_count: usize,
        deleted_count: usize,
    },
}

/// Trait for receiving deploy events
///
/// Implementations can be:
/// - ConsoleEventSink: Progress display in terminal
/// - JsonEventSink: NDJSON event stream for CI
/// - NoopEventSink: Silent operation
pub trait DeployEventSink: Send + Sync {
    /// Handle a deploy event
    fn on_event(&self, event: DeployEvent);

    /// Check if this sink wants detailed events (e.g., per-file)
    ///
    /// Some sinks (like CI) may only want summary events.
    fn wants_detailed_events(&self) -> bool {
        true
    }
}

/// No-op event sink for silent operation
pub struct NoopEventSink;

impl DeployEventSink for NoopEventSink {
    fn on_event(&self, _event: DeployEvent) {
        // Do nothing
    }

    fn wants_detailed_events(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Test event sink that records all events
    struct RecordingEventSink {
        events: Arc<Mutex<Vec<DeployEvent>>>,
    }

    impl RecordingEventSink {
        fn new() -> (Self, Arc<Mutex<Vec<DeployEvent>>>) {
            let events = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    events: events.clone(),
                },
                events,
            )
        }
    }

    impl DeployEventSink for RecordingEventSink {
        fn on_event(&self, event: DeployEvent) {
            self.events.lock().unwrap().push(event);
        }
    }

    #[test]
    fn recording_sink_captures_events() {
        let (sink, events) = RecordingEventSink::new();

        sink.on_event(DeployEvent::Started {
            source: PathBuf::from(".promptpack"),
            destination: "~/".to_string(),
            asset_count: 5,
        });

        sink.on_event(DeployEvent::FileWritten {
            index: 0,
            path: PathBuf::from("test.md"),
        });

        let recorded = events.lock().unwrap();
        assert_eq!(recorded.len(), 2);
    }

    #[test]
    fn noop_sink_wants_no_details() {
        let sink = NoopEventSink;
        assert!(!sink.wants_detailed_events());
    }
}
