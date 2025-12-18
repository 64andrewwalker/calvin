use crate::ui::widgets::r#box::{Box, BoxStyle};

pub fn render_version(
    calvin_version: &str,
    adapters: &[(String, String)],
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    let mut b = Box::with_style(BoxStyle::Info);
    b.add_line(format!("Calvin v{}", calvin_version));
    b.add_line("Source Format: 1.0");
    b.add_empty();
    b.add_line("Adapters:");
    for (name, version) in adapters {
        b.add_line(format!("  - {:<12} v{}", name, version));
    }
    b.render(supports_color, supports_unicode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_renders_with_themed_borders() {
        let rendered = render_version(
            "0.0.0",
            &[("Cursor".to_string(), "1".to_string())],
            false,
            true,
        );
        assert!(rendered.starts_with('╭'));
    }
}

