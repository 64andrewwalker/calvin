use std::io::{self, Write};

use crossterm::{cursor, terminal, QueueableCommand};

#[derive(Debug, Default)]
pub struct LiveRegion {
    last_lines: usize,
}

impl LiveRegion {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self, out: &mut impl Write) -> io::Result<()> {
        self.update(out, "")
    }

    pub fn update(&mut self, out: &mut impl Write, content: &str) -> io::Result<()> {
        let mut content = content.to_string();
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }

        let lines_to_clear = self.last_lines.min(u16::MAX as usize) as u16;
        if lines_to_clear > 0 {
            out.queue(cursor::MoveUp(lines_to_clear))?;
        }

        for _ in 0..lines_to_clear {
            out.queue(cursor::MoveToColumn(0))?;
            out.queue(terminal::Clear(terminal::ClearType::CurrentLine))?;
            out.queue(cursor::MoveDown(1))?;
        }

        if lines_to_clear > 0 {
            out.queue(cursor::MoveUp(lines_to_clear))?;
        }

        out.write_all(content.as_bytes())?;
        out.flush()?;

        self.last_lines = content.chars().filter(|&c| c == '\n').count();
        Ok(())
    }
}
