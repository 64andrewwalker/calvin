use unicode_width::UnicodeWidthStr;

use crate::ui::primitives::border::BorderChar;
use crossterm::style::Stylize;

use crate::ui::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BoxStyle {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Default, Clone)]
pub struct Box {
    title: Option<String>,
    content: Vec<String>,
    width: Option<u16>,
    style: BoxStyle,
}

impl Box {
    pub fn with_title(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            ..Self::default()
        }
    }

    pub fn with_style(style: BoxStyle) -> Self {
        Self {
            style,
            ..Self::default()
        }
    }

    pub fn style(mut self, style: BoxStyle) -> Self {
        self.style = style;
        self
    }

    pub fn add_line(&mut self, line: impl Into<String>) {
        let line = line.into();
        for part in line.lines() {
            self.content.push(part.to_string());
        }
    }

    pub fn add_empty(&mut self) {
        self.content.push(String::new());
    }

    pub fn render(&self, supports_color: bool, supports_unicode: bool) -> String {
        let mut lines = Vec::new();
        if let Some(title) = &self.title {
            lines.push(title.clone());
        }
        lines.extend(self.content.iter().cloned());

        let inner_width = self
            .width
            .unwrap_or_else(|| {
                lines
                    .iter()
                    .map(|l| visible_width(l))
                    .max()
                    .unwrap_or(0)
                    .saturating_add(2) as u16
            })
            .max(2);

        let tl = BorderChar::TopLeft.render(supports_unicode);
        let tr = BorderChar::TopRight.render(supports_unicode);
        let bl = BorderChar::BottomLeft.render(supports_unicode);
        let br = BorderChar::BottomRight.render(supports_unicode);
        let h = BorderChar::Horizontal.render(supports_unicode);
        let v = BorderChar::Vertical.render(supports_unicode);

        let mut out = String::new();
        let top = format!("{}{}{}", tl, h.repeat(inner_width as usize), tr);
        out.push_str(&color_border(&top, supports_color, self.style));
        out.push('\n');

        for line in &lines {
            let w = visible_width(line);
            out.push_str(&color_border(v, supports_color, self.style));
            out.push(' ');
            out.push_str(line);
            out.push_str(&" ".repeat(inner_width.saturating_sub(1) as usize - w));
            out.push_str(&color_border(v, supports_color, self.style));
            out.push('\n');
        }

        let bottom = format!("{}{}{}", bl, h.repeat(inner_width as usize), br);
        out.push_str(&color_border(&bottom, supports_color, self.style));
        out.push('\n');
        out
    }
}

fn color_border(s: &str, supports_color: bool, style: BoxStyle) -> String {
    if !supports_color {
        return s.to_string();
    }

    let color = match style {
        BoxStyle::Info => theme::colors::INFO,
        BoxStyle::Success => theme::colors::SUCCESS,
        BoxStyle::Warning => theme::colors::WARNING,
        BoxStyle::Error => theme::colors::ERROR,
    };
    format!("{}", s.with(color))
}

fn visible_width(s: &str) -> usize {
    strip_ansi(s).width()
}

fn strip_ansi(s: &str) -> std::borrow::Cow<'_, str> {
    if !s.contains('\u{1b}') {
        return std::borrow::Cow::Borrowed(s);
    }

    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // Skip ANSI escape sequence: ESC [ ... <final>
            if matches!(chars.peek(), Some('[') | Some(']')) {
                let _ = chars.next();
            }
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        out.push(c);
    }

    std::borrow::Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_splits_multiline_content_into_rows() {
        let mut b = Box::with_title("TITLE");
        b.add_line("Line1\nLine2");
        let rendered = b.render(false, true);

        let line2 = rendered
            .lines()
            .find(|l| l.contains("Line2"))
            .expect("expected Line2 to appear in output");
        assert!(line2.starts_with(BorderChar::Vertical.render(true)));
    }
}
