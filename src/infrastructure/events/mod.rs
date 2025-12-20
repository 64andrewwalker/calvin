//! Event Sink Implementations
//!
//! Provides concrete implementations of DeployEventSink:
//! - JsonEventSink: NDJSON output for CI/automation
//! - ConsoleEventSink: Human-readable progress (future)

mod json;

pub use json::JsonEventSink;
