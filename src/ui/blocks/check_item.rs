use crate::ui::primitives::icon::Icon;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct CheckItem {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    pub recommendation: Option<String>,
    pub details: Vec<String>,
}

impl CheckItem {
    pub fn render(&self, verbose: bool, supports_color: bool, supports_unicode: bool) -> String {
        let icon = match self.status {
            CheckStatus::Pass => Icon::Success,
            CheckStatus::Warning => Icon::Warning,
            CheckStatus::Error => Icon::Error,
        }
        .colored(supports_color, supports_unicode);

        let mut out = String::new();
        out.push_str(&format!("  {} {} - {}\n", icon, self.name, self.message));

        if let Some(rec) = &self.recommendation {
            out.push_str(&format!(
                "    {} {}\n",
                Icon::Arrow.colored(supports_color, supports_unicode),
                rec
            ));
        }

        if verbose {
            for detail in &self.details {
                out.push_str(&format!(
                    "    {} {}\n",
                    Icon::Arrow.colored(supports_color, supports_unicode),
                    detail
                ));
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_includes_recommendation_line() {
        let item = CheckItem {
            name: "settings".to_string(),
            status: CheckStatus::Warning,
            message: "No settings.json found".to_string(),
            recommendation: Some("Run `calvin deploy` to generate baseline".to_string()),
            details: Vec::new(),
        };

        let rendered = item.render(false, false, false);
        assert!(rendered.contains("[>] Run `calvin deploy` to generate baseline"));
    }
}
