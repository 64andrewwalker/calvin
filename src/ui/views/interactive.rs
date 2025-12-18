use crate::ui::widgets::r#box::{Box, BoxStyle};

pub fn render_banner(supports_color: bool, supports_unicode: bool) -> String {
    let mut b = Box::with_style(BoxStyle::Info);
    b.add_empty();
    b.add_line("  Calvin - Making AI agents behave");
    b.add_empty();
    b.add_line("  Maintain AI rules in one place, deploy to");
    b.add_line("  Claude, Cursor, VS Code, and more.");
    b.add_empty();
    b.render(supports_color, supports_unicode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_uses_box_borders_in_ascii_mode() {
        let rendered = render_banner(false, false);
        assert!(rendered.lines().next().unwrap_or_default().starts_with('+'));
    }
}

