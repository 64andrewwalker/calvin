use crate::ui::widgets::r#box::{Box, BoxStyle};
use crate::ui::primitives::icon::Icon;

#[derive(Debug, Clone)]
pub struct WarningBlock {
    title: String,
    lines: Vec<String>,
}

impl WarningBlock {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            lines: Vec::new(),
        }
    }

    pub fn add_line(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
    }

    pub fn render(&self, supports_color: bool, supports_unicode: bool) -> String {
        let header = format!(
            "{} {}",
            Icon::Warning.colored(supports_color, supports_unicode),
            self.title
        );
        let mut b = Box::with_title(header).style(BoxStyle::Warning);
        for line in &self.lines {
            b.add_line(line.clone());
        }
        b.render(supports_color, supports_unicode)
    }
}
