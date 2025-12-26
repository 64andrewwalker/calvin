//! Shared JSON event types for consistent CLI output.
//!
//! All commands should use these event types for JSON output to ensure
//! consistent field naming and structure across the CLI.

// TODO: Remove this once Phase 2 migration is complete
#![allow(dead_code)]

use serde::Serialize;

/// Event emitted when a command starts.
#[derive(Debug, Clone, Serialize)]
pub struct StartEvent<'a> {
    pub event: &'static str,
    pub command: &'a str,
    pub version: &'static str,
}

impl<'a> StartEvent<'a> {
    pub fn new(command: &'a str) -> Self {
        Self {
            event: "start",
            command,
            version: env!("CARGO_PKG_VERSION"),
        }
    }
}

/// Event emitted when a command completes successfully.
#[derive(Debug, Clone, Serialize)]
pub struct CompleteEvent<'a> {
    pub event: &'static str,
    pub command: &'a str,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

impl<'a> CompleteEvent<'a> {
    pub fn success(command: &'a str) -> Self {
        Self {
            event: "complete",
            command,
            success: true,
            duration_ms: None,
        }
    }

    pub fn failure(command: &'a str) -> Self {
        Self {
            event: "complete",
            command,
            success: false,
            duration_ms: None,
        }
    }

    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }
}

/// Event emitted when an error occurs.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorEvent<'a> {
    pub event: &'static str,
    pub command: &'a str,
    pub code: &'a str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

impl<'a> ErrorEvent<'a> {
    pub fn new(command: &'a str, code: &'a str, message: impl Into<String>) -> Self {
        Self {
            event: "error",
            command,
            code,
            message: message.into(),
            help: None,
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

/// Wrapper for data events that includes command context.
#[derive(Debug, Clone, Serialize)]
pub struct DataEvent<'a, T: Serialize> {
    pub event: &'static str,
    pub command: &'a str,
    #[serde(flatten)]
    pub data: T,
}

impl<'a, T: Serialize> DataEvent<'a, T> {
    pub fn new(command: &'a str, data: T) -> Self {
        Self {
            event: "data",
            command,
            data,
        }
    }
}

/// Progress event for long-running operations.
#[derive(Debug, Clone, Serialize)]
pub struct ProgressEvent<'a> {
    pub event: &'static str,
    pub command: &'a str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
}

impl<'a> ProgressEvent<'a> {
    pub fn new(command: &'a str, message: impl Into<String>) -> Self {
        Self {
            event: "progress",
            command,
            message: message.into(),
            current: None,
            total: None,
        }
    }

    pub fn with_progress(mut self, current: usize, total: usize) -> Self {
        self.current = Some(current);
        self.total = Some(total);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_event_serializes_correctly() {
        let event = StartEvent::new("deploy");
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["event"], "start");
        assert_eq!(json["command"], "deploy");
        assert!(json["version"].is_string());
    }

    #[test]
    fn complete_event_success_serializes_correctly() {
        let event = CompleteEvent::success("clean");
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["event"], "complete");
        assert_eq!(json["command"], "clean");
        assert_eq!(json["success"], true);
        assert!(json.get("duration_ms").is_none());
    }

    #[test]
    fn complete_event_failure_serializes_correctly() {
        let event = CompleteEvent::failure("check");
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["event"], "complete");
        assert_eq!(json["command"], "check");
        assert_eq!(json["success"], false);
    }

    #[test]
    fn complete_event_with_duration_serializes_correctly() {
        let event = CompleteEvent::success("deploy").with_duration(1234);
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["duration_ms"], 1234);
    }

    #[test]
    fn error_event_serializes_correctly() {
        let event = ErrorEvent::new("deploy", "FILE_NOT_FOUND", "Config file not found");
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["event"], "error");
        assert_eq!(json["command"], "deploy");
        assert_eq!(json["code"], "FILE_NOT_FOUND");
        assert_eq!(json["message"], "Config file not found");
        assert!(json.get("help").is_none());
    }

    #[test]
    fn error_event_with_help_serializes_correctly() {
        let event = ErrorEvent::new("deploy", "FILE_NOT_FOUND", "Config file not found")
            .with_help("Run 'calvin init' to create a config file");
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["help"], "Run 'calvin init' to create a config file");
    }

    #[test]
    fn data_event_flattens_data() {
        #[derive(Serialize)]
        struct TestData {
            count: usize,
            items: Vec<String>,
        }

        let data = TestData {
            count: 3,
            items: vec!["a".into(), "b".into(), "c".into()],
        };
        let event = DataEvent::new("layers", data);
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["event"], "data");
        assert_eq!(json["command"], "layers");
        assert_eq!(json["count"], 3);
        assert_eq!(json["items"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn progress_event_serializes_correctly() {
        let event = ProgressEvent::new("deploy", "Compiling assets...");
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["event"], "progress");
        assert_eq!(json["command"], "deploy");
        assert_eq!(json["message"], "Compiling assets...");
    }

    #[test]
    fn progress_event_with_progress_serializes_correctly() {
        let event = ProgressEvent::new("clean", "Deleting files...").with_progress(5, 10);
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["current"], 5);
        assert_eq!(json["total"], 10);
    }
}
