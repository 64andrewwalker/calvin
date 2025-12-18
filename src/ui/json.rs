use std::io::{self, Write};

/// Write a single NDJSON event (one JSON object per line).
pub fn write_event(out: &mut impl Write, event: &serde_json::Value) -> io::Result<()> {
    let line = serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string());
    out.write_all(line.as_bytes())?;
    out.write_all(b"\n")?;
    Ok(())
}

/// Convenience helper that writes to stdout.
pub fn emit(event: serde_json::Value) -> io::Result<()> {
    let mut out = io::stdout().lock();
    write_event(&mut out, &event)
}
