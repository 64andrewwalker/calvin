use crate::ui::primitives::icon::Icon;
use crate::ui::widgets::r#box::{Box, BoxStyle};

pub fn render_migrate_header(
    format: Option<&str>,
    adapter: Option<&str>,
    dry_run: bool,
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    let mut b = Box::with_style(BoxStyle::Info);
    b.add_line("Calvin Migrate");
    if let Some(format) = format {
        b.add_line(format!("Target Format: {}", format));
    }
    if let Some(adapter) = adapter {
        b.add_line(format!("Target Adapter: {}", adapter));
    }
    if dry_run {
        b.add_line("Mode: Dry run");
    }
    b.render(supports_color, supports_unicode)
}

pub fn render_migrate_complete(message: &str, supports_color: bool, supports_unicode: bool) -> String {
    let mut b = Box::with_style(BoxStyle::Success);
    b.add_line(format!(
        "{} {}",
        Icon::Success.colored(supports_color, supports_unicode),
        message
    ));
    b.render(supports_color, supports_unicode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_complete_uses_themed_borders() {
        let rendered = render_migrate_complete("ok", false, true);
        assert!(rendered.starts_with('╭'));
    }
}

