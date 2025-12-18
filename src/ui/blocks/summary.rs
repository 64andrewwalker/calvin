use crate::ui::primitives::icon::Icon;
use crate::ui::primitives::text::ColoredText;
use crate::ui::widgets::r#box::{Box, BoxStyle};

#[derive(Debug, Clone)]
pub struct ResultSummary {
    title: String,
    success: bool,
    stats: Vec<(String, usize)>,
    infos: Vec<String>,
    warnings: Vec<String>,
    next_step: Option<String>,
}

impl ResultSummary {
    pub fn success(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            success: true,
            stats: Vec::new(),
            infos: Vec::new(),
            warnings: Vec::new(),
            next_step: None,
        }
    }

    pub fn partial(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            success: false,
            stats: Vec::new(),
            infos: Vec::new(),
            warnings: Vec::new(),
            next_step: None,
        }
    }

    pub fn add_stat(&mut self, label: impl Into<String>, count: usize) {
        self.stats.push((label.into(), count));
    }

    /// Add an informational message (shown with success icon)
    pub fn add_info(&mut self, message: impl Into<String>) {
        self.infos.push(message.into());
    }

    pub fn add_warning(&mut self, message: impl Into<String>) {
        self.warnings.push(message.into());
    }

    pub fn with_next_step(&mut self, hint: impl Into<String>) {
        self.next_step = Some(hint.into());
    }

    pub fn render(&self, supports_color: bool, supports_unicode: bool) -> String {
        let (style, icon) = if self.success {
            (BoxStyle::Success, Icon::Success)
        } else {
            (BoxStyle::Warning, Icon::Warning)
        };

        let title = if self.success {
            ColoredText::success(self.title.as_str())
                .bold()
                .render(supports_color)
        } else {
            ColoredText::warning(self.title.as_str())
                .bold()
                .render(supports_color)
        };

        let header = format!(
            "{} {}",
            icon.colored(supports_color, supports_unicode),
            title
        );

        let mut b = Box::with_title(header).style(style);
        b.add_empty();

        for (label, count) in &self.stats {
            b.add_line(format!("{} {}", count, label));
        }

        if !self.infos.is_empty() {
            b.add_empty();
            for info in &self.infos {
                b.add_line(format!(
                    "{} {}",
                    Icon::Success.colored(supports_color, supports_unicode),
                    info
                ));
            }
        }

        if !self.warnings.is_empty() {
            b.add_empty();
            for warning in &self.warnings {
                b.add_line(format!(
                    "{} {}",
                    Icon::Warning.colored(supports_color, supports_unicode),
                    warning
                ));
            }
        }

        if let Some(next_step) = &self.next_step {
            b.add_empty();
            b.add_line(format!(
                "{} {} {}",
                Icon::Arrow.colored(supports_color, supports_unicode),
                ColoredText::dim("Next:").render(supports_color),
                next_step
            ));
        }

        b.render(supports_color, supports_unicode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_success_icon_in_title() {
        let mut summary = ResultSummary::success("Deploy Complete");
        summary.add_stat("files written", 2);

        let rendered = summary.render(false, false);
        assert!(rendered.contains("[OK] Deploy Complete"));
    }
}
