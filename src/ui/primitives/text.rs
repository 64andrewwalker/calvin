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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_without_color_returns_plain_text() {
        let t = ColoredText::success("ok");
        assert_eq!(t.render(false), "ok");
    }

    #[test]
    fn render_with_color_includes_ansi_escape() {
        let t = ColoredText::error("no");
        let rendered = t.render(true);
        assert!(rendered.contains("\u{1b}["));
    }
}
