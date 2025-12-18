use std::path::{Path, PathBuf};

use crate::ui::primitives::icon::Icon;
use crate::ui::primitives::text::ColoredText;
use crate::ui::widgets::r#box::{Box, BoxStyle};

#[derive(Debug, Clone)]
pub struct ErrorBlock {
    file: PathBuf,
    line: Option<usize>,
    message: String,
    code_context: Option<Vec<(usize, String, bool)>>, // (line_no, content, highlight)
    fix: Option<String>,
}

impl ErrorBlock {
    pub fn new(file: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            line: None,
            message: message.into(),
            code_context: None,
            fix: None,
        }
    }

    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    pub fn with_fix(mut self, fix: impl Into<String>) -> Self {
        self.fix = Some(fix.into());
        self
    }

    pub fn with_file_context(mut self, before: usize, after: usize) -> Self {
        let Some(line) = self.line else { return self };
        let Some(ctx) = read_code_context(&self.file, line, before, after) else { return self };
        self.code_context = Some(ctx);
        self
    }

    pub fn render(&self, supports_color: bool, supports_unicode: bool) -> String {
        let mut b = Box::with_title("ERROR").style(BoxStyle::Error);

        b.add_line(self.file.display().to_string());
        if let Some(line) = self.line {
            b.add_line(format!("Line {}", line));
        }
        b.add_empty();
        b.add_line(self.message.clone());

        if let Some(lines) = &self.code_context {
            b.add_empty();
            for (no, text, highlight) in lines {
                let prefix = if *highlight {
                    Icon::Pointer.render(supports_unicode)
                } else {
                    " "
                };

                let rendered_line = if supports_color && *highlight {
                    ColoredText::error(text).render(true)
                } else {
                    text.clone()
                };

                b.add_line(format!("{prefix} {:>4} | {}", no, rendered_line));
            }
        }

        if let Some(fix) = &self.fix {
            b.add_empty();
            b.add_line(format!("FIX: {}", fix));
        }

        b.render(supports_color, supports_unicode)
    }
}

fn read_code_context(file: &Path, line: usize, before: usize, after: usize) -> Option<Vec<(usize, String, bool)>> {
    let content = std::fs::read_to_string(file).ok()?;
    let lines: Vec<&str> = content.lines().collect();
    if line == 0 || line > lines.len() {
        return None;
    }

    let start = line.saturating_sub(before).saturating_sub(1);
    let end = (line + after).min(lines.len());

    let mut out = Vec::new();
    for (idx, text) in lines[start..end].iter().enumerate() {
        let line_no = start + idx + 1;
        out.push((line_no, (*text).to_string(), line_no == line));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_pointer_for_highlighted_line() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("example.txt");
        std::fs::write(&file, "first\nsecond\nthird\n").unwrap();

        let rendered = ErrorBlock::new(&file, "bad")
            .with_line(2)
            .with_file_context(1, 1)
            .render(false, true);

        assert!(rendered.contains(&format!(
            "{}    2 | second",
            Icon::Pointer.render(true)
        )));
    }

    #[test]
    fn renders_ascii_pointer_when_unicode_unsupported() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("example.txt");
        std::fs::write(&file, "first\nsecond\nthird\n").unwrap();

        let rendered = ErrorBlock::new(&file, "bad")
            .with_line(2)
            .with_file_context(1, 1)
            .render(false, false);

        assert!(rendered.contains(&format!(
            "{}    2 | second",
            Icon::Pointer.render(false)
        )));
    }
}
