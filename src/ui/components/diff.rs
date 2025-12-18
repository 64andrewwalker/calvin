use crossterm::style::Stylize;
use similar::{ChangeTag, TextDiff};

use crate::ui::theme;

pub fn render_unified_diff_with_line_numbers(
    path: &str,
    old: &str,
    new: &str,
    supports_color: bool,
) -> String {
    let diff = TextDiff::from_lines(old, new);
    let old_lines = old.lines().count().max(1);
    let new_lines = new.lines().count().max(1);
    let width = old_lines.max(new_lines).to_string().len();

    let mut out = String::new();

    let header_a = format!("--- a/{}", path);
    let header_b = format!("+++ b/{}", path);
    out.push_str(&color_line(&header_a, ChangeTag::Equal, supports_color, LineStyle::Header));
    out.push('\n');
    out.push_str(&color_line(&header_b, ChangeTag::Equal, supports_color, LineStyle::Header));
    out.push('\n');

    for change in diff.iter_all_changes() {
        let (old_no, new_no, sign) = match change.tag() {
            ChangeTag::Delete => (change.old_index().map(|i| i + 1), None, "-"),
            ChangeTag::Insert => (None, change.new_index().map(|i| i + 1), "+"),
            ChangeTag::Equal => (
                change.old_index().map(|i| i + 1),
                change.new_index().map(|i| i + 1),
                " ",
            ),
        };

        let old_col = old_no
            .map(|n| format!("{:>width$}", n, width = width))
            .unwrap_or_else(|| " ".repeat(width));
        let new_col = new_no
            .map(|n| format!("{:>width$}", n, width = width))
            .unwrap_or_else(|| " ".repeat(width));

        let value = change.value().trim_end_matches('\n');
        let line = format!("{old_col} {new_col} {sign} {value}");
        out.push_str(&color_line(&line, change.tag(), supports_color, LineStyle::Body));
        out.push('\n');
    }

    out
}

#[derive(Debug, Clone, Copy)]
enum LineStyle {
    Header,
    Body,
}

fn color_line(s: &str, tag: ChangeTag, supports_color: bool, style: LineStyle) -> String {
    if !supports_color {
        return s.to_string();
    }

    match style {
        LineStyle::Header => format!("{}", s.with(theme::colors::INFO)),
        LineStyle::Body => match tag {
            ChangeTag::Delete => format!("{}", s.with(theme::colors::ERROR)),
            ChangeTag::Insert => format!("{}", s.with(theme::colors::SUCCESS)),
            ChangeTag::Equal => format!("{}", s.with(theme::colors::DIM)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_added_lines_with_plus_prefix() {
        let rendered =
            render_unified_diff_with_line_numbers("file.txt", "a\nb\n", "a\nc\n", false);
        assert!(rendered.contains("+ c"));
    }

    #[test]
    fn renders_removed_lines_with_minus_prefix() {
        let rendered =
            render_unified_diff_with_line_numbers("file.txt", "a\nb\n", "a\nc\n", false);
        assert!(rendered.contains("- b"));
    }
}
