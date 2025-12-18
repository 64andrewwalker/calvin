use crate::ui::primitives::icon::Icon;
use crate::ui::primitives::text::ColoredText;

#[derive(Debug, Clone)]
pub struct CommandHeader {
    icon: Icon,
    title: String,
    items: Vec<(String, String)>,
}

impl CommandHeader {
    pub fn new(icon: Icon, title: impl Into<String>) -> Self {
        Self {
            icon,
            title: title.into(),
            items: Vec::new(),
        }
    }

    pub fn add(&mut self, label: impl Into<String>, value: impl Into<String>) {
        self.items.push((label.into(), value.into()));
    }

    pub fn render(&self, supports_color: bool, supports_unicode: bool) -> String {
        let mut out = String::new();
        let title = ColoredText::info(self.title.as_str())
            .bold()
            .render(supports_color);
        out.push_str(&format!(
            "{} {}\n",
            self.icon.colored(supports_color, supports_unicode),
            title
        ));
        for (label, value) in &self.items {
            out.push_str(&format!("{}: {}\n", label, value));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_ascii_icon_when_unicode_unsupported() {
        let mut header = CommandHeader::new(Icon::Deploy, "Calvin Deploy");
        header.add("Source", ".promptpack/");

        let rendered = header.render(false, false);
        assert!(rendered.contains("[DEPLOY] Calvin Deploy"));
    }
}
