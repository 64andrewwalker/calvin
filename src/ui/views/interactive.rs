use crate::ui::primitives::icon::Icon;
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

pub fn render_setup_intro(supports_color: bool, supports_unicode: bool) -> String {
    let mut b = Box::with_style(BoxStyle::Info);
    b.add_line("Great! Let's set up Calvin in 3 quick steps.");
    b.render(supports_color, supports_unicode)
}

pub fn render_step_header(
    step: u8,
    total: u8,
    title: &str,
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    let mut b = Box::with_style(BoxStyle::Info);
    b.add_line(format!("Step {} of {}", step, total));
    b.add_line(format!("  {}", title));
    b.render(supports_color, supports_unicode)
}

pub fn render_section_header(title: &str, supports_color: bool, supports_unicode: bool) -> String {
    let mut b = Box::with_style(BoxStyle::Info);
    b.add_line(title);
    b.render(supports_color, supports_unicode)
}

pub fn render_setup_complete(supports_color: bool, supports_unicode: bool) -> String {
    let mut b = Box::with_style(BoxStyle::Success);
    b.add_line(format!(
        "{} Setup Complete!",
        Icon::Success.colored(supports_color, supports_unicode)
    ));
    b.add_empty();
    b.add_line("Created:");
    b.add_line("  .promptpack/config.toml");
    b.add_line("  .promptpack/actions/");
    b.add_empty();
    b.add_line("Next steps:");
    b.add_line("  1. Edit your prompts in `.promptpack/`");
    b.add_line("  2. Deploy to your AI tools: `calvin deploy`");
    b.add_line("  3. Validate config: `calvin check`");
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

    #[test]
    fn step_header_uses_themed_borders() {
        let rendered = render_step_header(1, 3, "Choose targets", false, true);
        assert!(rendered.starts_with('╭'));
    }

    #[test]
    fn setup_intro_uses_themed_borders() {
        let rendered = render_setup_intro(false, true);
        assert!(rendered.starts_with('╭'));
        assert!(rendered.contains("Great! Let's set up Calvin"));
    }
}
