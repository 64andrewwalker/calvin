//! Deprecated UI render utilities
//!
//! Moved from `src/ui/render.rs` as part of a cleanup pass.
//! Kept for one release cycle before permanent deletion.

use std::io::{self, Write};

use crossterm::{cursor, terminal, QueueableCommand};

#[derive(Debug, Clone)]
pub struct TerminalState {
    pub width: u16,
    pub height: u16,
    pub cursor_x: u16,
    pub cursor_y: u16,
    buffer: Vec<u8>,
}

impl TerminalState {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            cursor_x: 0,
            cursor_y: 0,
            buffer: Vec::new(),
        }
    }

    pub fn write_str(&mut self, s: &str) {
        self.buffer.extend_from_slice(s.as_bytes());
    }

    pub fn clear_line(&mut self) -> io::Result<()> {
        self.buffer
            .queue(terminal::Clear(terminal::ClearType::CurrentLine))?;
        Ok(())
    }

    pub fn move_cursor(&mut self, x: u16, y: u16) -> io::Result<()> {
        self.cursor_x = x;
        self.cursor_y = y;
        self.buffer.queue(cursor::MoveTo(x, y))?;
        Ok(())
    }

    pub fn flush(&mut self) -> io::Result<()> {
        let mut out = io::stdout().lock();
        self.flush_to(&mut out)
    }

    pub fn flush_to(&mut self, out: &mut impl Write) -> io::Result<()> {
        out.write_all(&self.buffer)?;
        out.flush()?;
        self.buffer.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flush_clears_buffer() {
        let mut state = TerminalState::new(80, 24);
        state.write_str("hello");
        let mut out = Vec::new();
        state.flush_to(&mut out).unwrap();
        assert!(out.starts_with(b"hello"));

        // second flush should not write anything new
        let mut out2 = Vec::new();
        state.flush_to(&mut out2).unwrap();
        assert!(out2.is_empty());
    }
}
