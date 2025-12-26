//! JSON output utilities for CLI commands.
//!
//! This module provides:
//! - Shared event types for consistent JSON output (`events`)
//! - Helper functions for emitting NDJSON events
//!
//! ## Usage
//!
//! ```ignore
//! use crate::ui::json::{emit, emit_event, events::*};
//!
//! // Using typed events (preferred)
//! emit_event(&StartEvent::new("deploy"))?;
//! emit_event(&CompleteEvent::success("deploy"))?;
//!
//! // Using raw JSON (for dynamic data)
//! emit(serde_json::json!({ "event": "custom", ... }))?;
//! ```

// TODO: Remove this once Phase 2 migration is complete
#![allow(dead_code)]

pub mod events;

use serde::Serialize;
use std::io::{self, Write};

/// Write a single NDJSON event (one JSON object per line).
pub fn write_event(out: &mut impl Write, event: &serde_json::Value) -> io::Result<()> {
    let line = serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string());
    out.write_all(line.as_bytes())?;
    out.write_all(b"\n")?;
    Ok(())
}

/// Convenience helper that writes raw JSON value to stdout.
pub fn emit(event: serde_json::Value) -> io::Result<()> {
    let mut out = io::stdout().lock();
    write_event(&mut out, &event)
}

/// Emit a typed event as NDJSON to stdout.
///
/// This is the preferred way to emit events as it ensures type safety
/// and consistent field naming.
///
/// # Example
///
/// ```ignore
/// use crate::ui::json::{emit_event, events::StartEvent};
///
/// emit_event(&StartEvent::new("deploy"))?;
/// ```
pub fn emit_event<T: Serialize>(event: &T) -> io::Result<()> {
    let json =
        serde_json::to_string(event).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut out = io::stdout().lock();
    out.write_all(json.as_bytes())?;
    out.write_all(b"\n")?;
    Ok(())
}

/// Write a typed event to a custom writer.
///
/// Useful for testing or redirecting output.
pub fn write_typed_event<T: Serialize, W: Write>(out: &mut W, event: &T) -> io::Result<()> {
    let json =
        serde_json::to_string(event).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    out.write_all(json.as_bytes())?;
    out.write_all(b"\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use events::*;

    #[test]
    fn emit_event_writes_valid_json() {
        let mut buffer = Vec::new();
        let event = StartEvent::new("test");

        write_typed_event(&mut buffer, &event).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.ends_with('\n'));

        let parsed: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
        assert_eq!(parsed["event"], "start");
        assert_eq!(parsed["command"], "test");
    }

    #[test]
    fn emit_event_writes_ndjson_format() {
        let mut buffer = Vec::new();

        write_typed_event(&mut buffer, &StartEvent::new("cmd")).unwrap();
        write_typed_event(&mut buffer, &CompleteEvent::success("cmd")).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        let lines: Vec<_> = output.lines().collect();

        assert_eq!(lines.len(), 2);
        for line in lines {
            assert!(serde_json::from_str::<serde_json::Value>(line).is_ok());
        }
    }
}
