use std::path::Path;

use crate::ui::blocks::header::CommandHeader;
use crate::ui::components::diff::render_unified_diff_with_line_numbers;
use crate::ui::primitives::icon::Icon;
use crate::ui::widgets::r#box::{Box, BoxStyle};

pub fn render_diff_header(source: &Path, supports_color: bool, supports_unicode: bool) -> String {
    let mut header = CommandHeader::new(Icon::Diff, "Calvin Diff");
    header.add("Source", source.display().to_string());
    header.render(supports_color, supports_unicode)
}

pub fn render_file_diff(path: &str, old: &str, new: &str, supports_color: bool) -> String {
    render_unified_diff_with_line_numbers(path, old, new, supports_color)
}

pub fn render_diff_summary(
    new_files: usize,
    modified_files: usize,
    unchanged_files: usize,
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    let header = format!(
        "{} Diff Summary",
        Icon::Diff.colored(supports_color, supports_unicode)
    );

    let mut b = Box::with_title(header).style(BoxStyle::Info);
    b.add_empty();
    b.add_line(format!("{} new", new_files));
    b.add_line(format!("{} modified", modified_files));
    b.add_line(format!("{} unchanged", unchanged_files));
    b.render(supports_color, supports_unicode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_uses_diff_icon_in_ascii_mode() {
        let rendered = render_diff_header(Path::new(".promptpack"), false, false);
        assert!(rendered.contains("[DIFF] Calvin Diff"));
    }
}
