//! Shared JSON event types for consistent CLI output.
//!
//! All commands should use these event types for JSON output to ensure
//! consistent field naming and structure across the CLI.

// Note: Generic events (StartEvent, CompleteEvent, etc.) are available for future migrations.
// Clean-specific events are actively used in src/commands/clean.rs.

use serde::Serialize;

/// Event emitted when a command starts.
/// Generic event type for commands that don't have specialized event structs.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct StartEvent<'a> {
    pub event: &'static str,
    pub command: &'a str,
    pub version: &'static str,
}

#[allow(dead_code)]
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
/// Generic event type for commands that don't have specialized event structs.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct CompleteEvent<'a> {
    pub event: &'static str,
    pub command: &'a str,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

#[allow(dead_code)]
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
/// Generic event type for commands that don't have specialized event structs.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct ErrorEvent<'a> {
    pub event: &'static str,
    pub command: &'a str,
    pub code: &'a str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

#[allow(dead_code)]
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
/// Generic event type for commands that don't have specialized event structs.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct DataEvent<'a, T: Serialize> {
    pub event: &'static str,
    pub command: &'a str,
    #[serde(flatten)]
    pub data: T,
}

#[allow(dead_code)]
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
/// Generic event type for commands that don't have specialized event structs.
#[allow(dead_code)]
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

#[allow(dead_code)]
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

// --- Clean Command Events ---

/// Clean command start event.
/// Includes `type` for backward compatibility (deprecated in v1.0).
#[derive(Debug, Clone, Serialize)]
pub struct CleanStartEvent {
    pub event: &'static str,
    pub command: &'static str,
    /// Deprecated: Use `event` instead. Kept for backward compatibility.
    #[serde(rename = "type")]
    pub type_compat: &'static str,
    pub scope: String,
    pub file_count: usize,
}

impl CleanStartEvent {
    pub fn new(scope: &str, file_count: usize) -> Self {
        Self {
            event: "start",
            command: "clean",
            type_compat: "clean_start",
            scope: scope.to_string(),
            file_count,
        }
    }
}

/// Clean command complete event.
/// Includes `type` for backward compatibility (deprecated in v1.0).
#[derive(Debug, Clone, Serialize)]
pub struct CleanCompleteEvent {
    pub event: &'static str,
    pub command: &'static str,
    /// Deprecated: Use `event` instead. Kept for backward compatibility.
    #[serde(rename = "type")]
    pub type_compat: &'static str,
    pub deleted: usize,
    pub skipped: usize,
    pub errors: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl CleanCompleteEvent {
    pub fn new(deleted: usize, skipped: usize, errors: usize) -> Self {
        Self {
            event: "complete",
            command: "clean",
            type_compat: "clean_complete",
            deleted,
            skipped,
            errors,
            message: None,
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// Clean error event (lockfile-related errors).
#[derive(Debug, Clone, Serialize)]
pub struct CleanErrorEvent {
    pub event: &'static str,
    pub command: &'static str,
    /// Deprecated: Use `event` instead. Kept for backward compatibility.
    #[serde(rename = "type")]
    pub type_compat: &'static str,
    pub kind: &'static str,
    pub message: String,
}

impl CleanErrorEvent {
    pub fn lockfile_error(message: impl Into<String>) -> Self {
        Self {
            event: "error",
            command: "clean",
            type_compat: "clean_error",
            kind: "lockfile",
            message: message.into(),
        }
    }
}

/// File deleted event during clean.
#[derive(Debug, Clone, Serialize)]
pub struct FileDeletedEvent {
    pub event: &'static str,
    pub command: &'static str,
    /// Deprecated: Use `event` instead. Kept for backward compatibility.
    #[serde(rename = "type")]
    pub type_compat: &'static str,
    pub path: String,
    pub key: String,
}

impl FileDeletedEvent {
    pub fn new(path: String, key: String) -> Self {
        Self {
            event: "progress",
            command: "clean",
            type_compat: "file_deleted",
            path,
            key,
        }
    }
}

/// File skipped event during clean.
#[derive(Debug, Clone, Serialize)]
pub struct FileSkippedEvent {
    pub event: &'static str,
    pub command: &'static str,
    /// Deprecated: Use `event` instead. Kept for backward compatibility.
    #[serde(rename = "type")]
    pub type_compat: &'static str,
    pub path: String,
    pub key: String,
    pub reason: String,
}

impl FileSkippedEvent {
    pub fn new(path: String, key: String, reason: String) -> Self {
        Self {
            event: "progress",
            command: "clean",
            type_compat: "file_skipped",
            path,
            key,
            reason,
        }
    }
}

/// Clean all start event.
#[derive(Debug, Clone, Serialize)]
pub struct CleanAllStartEvent {
    pub event: &'static str,
    pub command: &'static str,
    /// Deprecated: Use `event` instead. Kept for backward compatibility.
    #[serde(rename = "type")]
    pub type_compat: &'static str,
    pub projects: usize,
}

impl CleanAllStartEvent {
    pub fn new(projects: usize) -> Self {
        Self {
            event: "start",
            command: "clean",
            type_compat: "clean_all_start",
            projects,
        }
    }
}

/// Clean all complete event.
#[derive(Debug, Clone, Serialize)]
pub struct CleanAllCompleteEvent {
    pub event: &'static str,
    pub command: &'static str,
    /// Deprecated: Use `event` instead. Kept for backward compatibility.
    #[serde(rename = "type")]
    pub type_compat: &'static str,
    pub projects: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<usize>,
}

impl CleanAllCompleteEvent {
    pub fn empty() -> Self {
        Self {
            event: "complete",
            command: "clean",
            type_compat: "clean_all_complete",
            projects: 0,
            deleted: None,
            errors: None,
        }
    }

    pub fn new(projects: usize, deleted: usize, errors: usize) -> Self {
        Self {
            event: "complete",
            command: "clean",
            type_compat: "clean_all_complete",
            projects,
            deleted: Some(deleted),
            errors: Some(errors),
        }
    }
}

/// Project skipped event during clean --all.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectSkippedEvent {
    pub event: &'static str,
    pub command: &'static str,
    /// Deprecated: Use `event` instead. Kept for backward compatibility.
    #[serde(rename = "type")]
    pub type_compat: &'static str,
    pub path: String,
    pub reason: &'static str,
}

impl ProjectSkippedEvent {
    pub fn missing_lockfile(path: String) -> Self {
        Self {
            event: "progress",
            command: "clean",
            type_compat: "project_skipped",
            path,
            reason: "missing_lockfile",
        }
    }
}

/// Project error event during clean --all.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectErrorEvent {
    pub event: &'static str,
    pub command: &'static str,
    /// Deprecated: Use `event` instead. Kept for backward compatibility.
    #[serde(rename = "type")]
    pub type_compat: &'static str,
    pub path: String,
    pub error: String,
}

impl ProjectErrorEvent {
    pub fn new(path: String, error: String) -> Self {
        Self {
            event: "error",
            command: "clean",
            type_compat: "project_error",
            path,
            error,
        }
    }
}

/// Project complete event during clean --all.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectCompleteEvent {
    pub event: &'static str,
    pub command: &'static str,
    /// Deprecated: Use `event` instead. Kept for backward compatibility.
    #[serde(rename = "type")]
    pub type_compat: &'static str,
    pub path: String,
    pub deleted: usize,
    pub skipped: usize,
    pub errors: usize,
}

impl ProjectCompleteEvent {
    pub fn new(path: String, deleted: usize, skipped: usize, errors: usize) -> Self {
        Self {
            event: "progress",
            command: "clean",
            type_compat: "project_complete",
            path,
            deleted,
            skipped,
            errors,
        }
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

    // --- Clean Event Tests ---

    #[test]
    fn clean_start_event_has_both_event_and_type() {
        let event = CleanStartEvent::new("project", 5);
        let json = serde_json::to_value(&event).unwrap();

        // New protocol fields
        assert_eq!(json["event"], "start");
        assert_eq!(json["command"], "clean");

        // Backward compat
        assert_eq!(json["type"], "clean_start");

        // Data fields
        assert_eq!(json["scope"], "project");
        assert_eq!(json["file_count"], 5);
    }

    #[test]
    fn clean_complete_event_has_both_event_and_type() {
        let event = CleanCompleteEvent::new(3, 1, 0);
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["event"], "complete");
        assert_eq!(json["command"], "clean");
        assert_eq!(json["type"], "clean_complete");
        assert_eq!(json["deleted"], 3);
        assert_eq!(json["skipped"], 1);
        assert_eq!(json["errors"], 0);
        assert!(json.get("message").is_none());
    }

    #[test]
    fn clean_complete_event_with_message() {
        let event = CleanCompleteEvent::new(0, 0, 0).with_message("No lockfile found");
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["message"], "No lockfile found");
    }

    #[test]
    fn clean_error_event_has_both_event_and_type() {
        let event = CleanErrorEvent::lockfile_error("Failed to parse");
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["event"], "error");
        assert_eq!(json["command"], "clean");
        assert_eq!(json["type"], "clean_error");
        assert_eq!(json["kind"], "lockfile");
        assert_eq!(json["message"], "Failed to parse");
    }

    #[test]
    fn file_deleted_event_has_both_event_and_type() {
        let event = FileDeletedEvent::new("/path/to/file".into(), "key".into());
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["event"], "progress");
        assert_eq!(json["command"], "clean");
        assert_eq!(json["type"], "file_deleted");
        assert_eq!(json["path"], "/path/to/file");
        assert_eq!(json["key"], "key");
    }

    #[test]
    fn file_skipped_event_has_both_event_and_type() {
        let event = FileSkippedEvent::new("/path/to/file".into(), "key".into(), "missing".into());
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["event"], "progress");
        assert_eq!(json["command"], "clean");
        assert_eq!(json["type"], "file_skipped");
        assert_eq!(json["path"], "/path/to/file");
        assert_eq!(json["key"], "key");
        assert_eq!(json["reason"], "missing");
    }

    #[test]
    fn clean_all_start_event_has_both_event_and_type() {
        let event = CleanAllStartEvent::new(5);
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["event"], "start");
        assert_eq!(json["command"], "clean");
        assert_eq!(json["type"], "clean_all_start");
        assert_eq!(json["projects"], 5);
    }

    #[test]
    fn clean_all_complete_event_empty() {
        let event = CleanAllCompleteEvent::empty();
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["event"], "complete");
        assert_eq!(json["command"], "clean");
        assert_eq!(json["type"], "clean_all_complete");
        assert_eq!(json["projects"], 0);
        assert!(json.get("deleted").is_none());
        assert!(json.get("errors").is_none());
    }

    #[test]
    fn clean_all_complete_event_with_stats() {
        let event = CleanAllCompleteEvent::new(3, 10, 1);
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["projects"], 3);
        assert_eq!(json["deleted"], 10);
        assert_eq!(json["errors"], 1);
    }

    #[test]
    fn project_skipped_event_has_both_event_and_type() {
        let event = ProjectSkippedEvent::missing_lockfile("/path/to/project".into());
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["event"], "progress");
        assert_eq!(json["command"], "clean");
        assert_eq!(json["type"], "project_skipped");
        assert_eq!(json["path"], "/path/to/project");
        assert_eq!(json["reason"], "missing_lockfile");
    }

    #[test]
    fn project_error_event_has_both_event_and_type() {
        let event = ProjectErrorEvent::new("/path/to/project".into(), "chdir failed".into());
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["event"], "error");
        assert_eq!(json["command"], "clean");
        assert_eq!(json["type"], "project_error");
        assert_eq!(json["path"], "/path/to/project");
        assert_eq!(json["error"], "chdir failed");
    }

    #[test]
    fn project_complete_event_has_both_event_and_type() {
        let event = ProjectCompleteEvent::new("/path/to/project".into(), 5, 2, 1);
        let json = serde_json::to_value(&event).unwrap();

        assert_eq!(json["event"], "progress");
        assert_eq!(json["command"], "clean");
        assert_eq!(json["type"], "project_complete");
        assert_eq!(json["path"], "/path/to/project");
        assert_eq!(json["deleted"], 5);
        assert_eq!(json["skipped"], 2);
        assert_eq!(json["errors"], 1);
    }
}
