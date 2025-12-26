use std::fmt;

use crossterm::style::Stylize;

use crate::ui::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticColor {
    Success,
    Error,
    Warning,
    Info,
    Dim,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColoredText {
    text: String,
    color: Option<SemanticColor>,
    bold: bool,
}

impl ColoredText {
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: Some(SemanticColor::Success),
            bold: false,
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: Some(SemanticColor::Error),
            bold: false,
        }
    }

    pub fn warning(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: Some(SemanticColor::Warning),
            bold: false,
        }
    }

    pub fn info(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: Some(SemanticColor::Info),
            bold: false,
        }
    }

    pub fn dim(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: Some(SemanticColor::Dim),
            bold: false,
        }
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn render(&self, supports_color: bool) -> String {
        if !supports_color {
            return self.text.clone();
        }

        if self.color.is_none() {
            if self.bold {
                return format!("{}", self.text.as_str().bold());
            }
            return self.text.clone();
        }

        let mut styled = match self.color {
            Some(SemanticColor::Success) => self.text.as_str().with(theme::colors::SUCCESS),
            Some(SemanticColor::Error) => self.text.as_str().with(theme::colors::ERROR),
            Some(SemanticColor::Warning) => self.text.as_str().with(theme::colors::WARNING),
            Some(SemanticColor::Info) => self.text.as_str().with(theme::colors::INFO),
            Some(SemanticColor::Dim) => self.text.as_str().with(theme::colors::DIM),
            None => unreachable!("handled by early return"),
        };

        if self.bold {
            styled = styled.bold();
        }

        format!("{}", styled)
    }
}

impl fmt::Display for ColoredText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

/// Truncate a string in the middle if it exceeds max_len, adding "…" in the center.
///
/// Uses Unicode ellipsis (…) and correctly handles multi-byte characters.
/// Useful for displaying file paths in constrained UI spaces.
///
/// # Example
/// ```ignore
/// assert_eq!(truncate_middle("short", 10), "short");
/// assert_eq!(truncate_middle("this_is_a_very_long_path", 15), "this_is…g_path");
/// ```
pub fn truncate_middle(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        return s.to_string();
    }
    if max_len <= 1 {
        return "…".to_string();
    }

    // Reserve 1 char for the ellipsis
    let keep = max_len.saturating_sub(1);
    let left = keep / 2;
    let right = keep.saturating_sub(left);

    let left_part: String = s.chars().take(left).collect();
    let right_part: String = s
        .chars()
        .rev()
        .take(right)
        .collect::<String>()
        .chars()
        .rev()
        .collect();

    format!("{}…{}", left_part, right_part)
}

/// Convert an absolute path to use tilde (~) notation for the home directory.
///
/// This makes paths more readable and portable in CLI output.
///
/// # Example
/// ```ignore
/// // On macOS with HOME=/Users/alice
/// let path = PathBuf::from("/Users/alice/.calvin/.promptpack");
/// assert_eq!(display_with_tilde(&path), "~/.calvin/.promptpack");
///
/// // Non-home paths are unchanged
/// let path = PathBuf::from("/tmp/foo");
/// assert_eq!(display_with_tilde(&path), "/tmp/foo");
/// ```
pub fn display_with_tilde(path: &std::path::Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(stripped) = path.strip_prefix(&home) {
            return format!("~/{}", stripped.display());
        }
    }
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_without_color_returns_plain_text() {
        let t = ColoredText::success("ok");
        assert_eq!(t.render(false), "ok");
    }

    #[test]
    fn truncate_middle_short_string_unchanged() {
        assert_eq!(truncate_middle("short", 10), "short");
        assert_eq!(truncate_middle("exact", 5), "exact");
    }

    #[test]
    fn truncate_middle_long_string_truncated() {
        let s = "this_is_a_very_long_path";
        let truncated = truncate_middle(s, 15);
        assert!(truncated.chars().count() <= 15);
        assert!(truncated.contains("…"));
    }

    #[test]
    fn truncate_middle_handles_unicode() {
        let s = "路径/文件/名称.txt";
        let truncated = truncate_middle(s, 10);
        assert!(truncated.chars().count() <= 10);
    }

    #[test]
    fn truncate_middle_very_short_max_len() {
        assert_eq!(truncate_middle("hello", 1), "…");
        assert_eq!(truncate_middle("hello", 0), "…");
    }

    #[test]
    fn render_with_color_includes_ansi_escape() {
        let t = ColoredText::error("no");
        let rendered = t.render(true);
        assert!(rendered.contains("\u{1b}["));
    }
}
