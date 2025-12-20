//! JSON Event Sink
//!
//! Outputs deploy events as NDJSON for CI/automation consumption.

use crate::domain::ports::{DeployEvent, DeployEventSink};
use std::io::{self, Write};
use std::sync::Mutex;

/// Event sink that outputs NDJSON events to stdout
pub struct JsonEventSink {
    /// Mutex to ensure thread-safe writes
    writer: Mutex<Box<dyn Write + Send>>,
}

impl JsonEventSink {
    /// Create a new JSON event sink writing to stdout
    pub fn stdout() -> Self {
        Self {
            writer: Mutex::new(Box::new(io::stdout())),
        }
    }

    /// Create a JSON event sink writing to a custom writer (for testing)
    #[allow(dead_code)]
    pub fn with_writer<W: Write + Send + 'static>(writer: W) -> Self {
        Self {
            writer: Mutex::new(Box::new(writer)),
        }
    }

    fn write_event(&self, event: serde_json::Value) {
        if let Ok(mut writer) = self.writer.lock() {
            let _ = writeln!(writer, "{}", event);
            let _ = writer.flush();
        }
    }
}

impl DeployEventSink for JsonEventSink {
    fn on_event(&self, event: DeployEvent) {
        let json = match event {
            DeployEvent::Started {
                source,
                destination,
                asset_count,
            } => {
                serde_json::json!({
                    "event": "start",
                    "command": "deploy",
                    "source": source.display().to_string(),
                    "destination": destination,
                    "asset_count": asset_count,
                })
            }

            DeployEvent::Compiled { output_count } => {
                serde_json::json!({
                    "event": "compiled",
                    "command": "deploy",
                    "output_count": output_count,
                })
            }

            DeployEvent::FileStarted { index, path } => {
                serde_json::json!({
                    "event": "item_start",
                    "command": "deploy",
                    "index": index,
                    "path": path.display().to_string(),
                })
            }

            DeployEvent::FileWritten { index, path } => {
                serde_json::json!({
                    "event": "item_written",
                    "command": "deploy",
                    "index": index,
                    "path": path.display().to_string(),
                })
            }

            DeployEvent::FileSkipped {
                index,
                path,
                reason,
            } => {
                serde_json::json!({
                    "event": "item_skipped",
                    "command": "deploy",
                    "index": index,
                    "path": path.display().to_string(),
                    "reason": reason,
                })
            }

            DeployEvent::FileError { index, path, error } => {
                serde_json::json!({
                    "event": "item_error",
                    "command": "deploy",
                    "index": index,
                    "path": path.display().to_string(),
                    "error": error,
                })
            }

            DeployEvent::OrphansDetected {
                total,
                safe_to_delete,
            } => {
                serde_json::json!({
                    "event": "orphans_detected",
                    "command": "deploy",
                    "total": total,
                    "safe_to_delete": safe_to_delete,
                })
            }

            DeployEvent::OrphanDeleted { path } => {
                serde_json::json!({
                    "event": "orphan_deleted",
                    "command": "deploy",
                    "path": path.display().to_string(),
                })
            }

            DeployEvent::Completed {
                written_count,
                skipped_count,
                error_count,
                deleted_count,
            } => {
                let status = if error_count == 0 {
                    "success"
                } else {
                    "partial"
                };
                serde_json::json!({
                    "event": "complete",
                    "command": "deploy",
                    "status": status,
                    "written": written_count,
                    "skipped": skipped_count,
                    "errors": error_count,
                    "deleted": deleted_count,
                })
            }
        };

        self.write_event(json);
    }

    fn wants_detailed_events(&self) -> bool {
        true // JSON mode wants all events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    struct TestWriter {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl TestWriter {
        fn new() -> (Self, Arc<Mutex<Vec<u8>>>) {
            let buffer = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    buffer: buffer.clone(),
                },
                buffer,
            )
        }
    }

    impl Write for TestWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buffer.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn json_sink_outputs_start_event() {
        let (writer, buffer) = TestWriter::new();
        let sink = JsonEventSink::with_writer(writer);

        sink.on_event(DeployEvent::Started {
            source: PathBuf::from(".promptpack"),
            destination: "~/".to_string(),
            asset_count: 5,
        });

        let output = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(output.contains("\"event\":\"start\""));
        assert!(output.contains("\"asset_count\":5"));
    }

    #[test]
    fn json_sink_outputs_complete_event() {
        let (writer, buffer) = TestWriter::new();
        let sink = JsonEventSink::with_writer(writer);

        sink.on_event(DeployEvent::Completed {
            written_count: 10,
            skipped_count: 5,
            error_count: 0,
            deleted_count: 2,
        });

        let output = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(output.contains("\"event\":\"complete\""));
        assert!(output.contains("\"status\":\"success\""));
        assert!(output.contains("\"written\":10"));
    }

    #[test]
    fn json_sink_outputs_partial_on_errors() {
        let (writer, buffer) = TestWriter::new();
        let sink = JsonEventSink::with_writer(writer);

        sink.on_event(DeployEvent::Completed {
            written_count: 10,
            skipped_count: 5,
            error_count: 2,
            deleted_count: 0,
        });

        let output = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(output.contains("\"status\":\"partial\""));
    }
}
