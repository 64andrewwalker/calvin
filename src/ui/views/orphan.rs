//! Orphan file display views
//!
//! **DEPRECATED**: These views are being replaced by DeployUseCase's built-in orphan handling.
//! They will be removed in v0.4.0.

#![allow(dead_code)]

use crossterm::style::Stylize;

use crate::ui::primitives::icon::Icon;
use crate::ui::theme;
use calvin::sync::orphan::OrphanFile;

/// Render orphan file list with status indicators
///
/// Returns formatted list showing:
/// - Success icon (green) for files safe to delete (has Calvin signature)
/// - Warning icon (yellow) for files that need manual review (no signature)
pub fn render_orphan_list(
    orphans: &[OrphanFile],
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    let mut output = String::new();

    let header = format!(
        "{} {} orphan files detected:",
        Icon::Warning.colored(supports_color, supports_unicode),
        orphans.len()
    );
    output.push_str(&header);
    output.push('\n');

    for orphan in orphans {
        let (icon, status, color) = if orphan.is_safe_to_delete() {
            (
                Icon::Success.render(supports_unicode),
                "[safe]",
                theme::colors::SUCCESS,
            )
        } else if orphan.exists {
            (
                Icon::Warning.render(supports_unicode),
                "[no signature]",
                theme::colors::WARNING,
            )
        } else {
            (
                Icon::Pending.render(supports_unicode),
                "[not found]",
                theme::colors::DIM,
            )
        };

        // Extract path from key for display
        let path = &orphan.path;

        let line = if supports_color {
            format!(
                "  {} {}  {}",
                icon.with(color),
                path,
                status.with(theme::colors::DIM)
            )
        } else {
            format!("  {} {}  {}", icon, path, status)
        };

        output.push_str(&line);
        output.push('\n');
    }

    output
}

/// Render orphan deletion summary
pub fn render_orphan_summary(
    deleted: usize,
    skipped: usize,
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    let mut parts = Vec::new();

    if deleted > 0 {
        let icon = Icon::Trash.colored(supports_color, supports_unicode);
        parts.push(format!("{} Deleted {} orphan file(s).", icon, deleted));
    }

    if skipped > 0 {
        let icon = Icon::Warning.colored(supports_color, supports_unicode);
        parts.push(format!(
            "{} Skipped {} file(s) (no Calvin signature).",
            icon, skipped
        ));
    }

    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orphan_list_shows_safe_and_unsafe() {
        let orphans = vec![
            OrphanFile {
                key: "home:~/.claude/commands/safe.md".to_string(),
                path: "~/.claude/commands/safe.md".to_string(),
                has_signature: true,
                exists: true,
            },
            OrphanFile {
                key: "home:~/.claude/commands/unsafe.md".to_string(),
                path: "~/.claude/commands/unsafe.md".to_string(),
                has_signature: false,
                exists: true,
            },
        ];

        let rendered = render_orphan_list(&orphans, false, false);

        assert!(rendered.contains("2 orphan files detected"));
        assert!(rendered.contains("safe.md"));
        assert!(rendered.contains("[safe]"));
        assert!(rendered.contains("unsafe.md"));
        assert!(rendered.contains("[no signature]"));
    }

    #[test]
    fn orphan_list_shows_not_found() {
        let orphans = vec![OrphanFile {
            key: "home:~/.claude/commands/gone.md".to_string(),
            path: "~/.claude/commands/gone.md".to_string(),
            has_signature: true,
            exists: false,
        }];

        let rendered = render_orphan_list(&orphans, false, false);
        assert!(rendered.contains("[not found]"));
    }

    #[test]
    fn orphan_summary_shows_deleted_count() {
        let rendered = render_orphan_summary(3, 0, false, false);
        assert!(rendered.contains("Deleted 3 orphan file(s)"));
    }

    #[test]
    fn orphan_summary_shows_skipped_count() {
        let rendered = render_orphan_summary(0, 2, false, false);
        assert!(rendered.contains("Skipped 2 file(s)"));
        assert!(rendered.contains("no Calvin signature"));
    }

    #[test]
    fn orphan_summary_shows_both() {
        let rendered = render_orphan_summary(5, 1, false, false);
        assert!(rendered.contains("Deleted 5"));
        assert!(rendered.contains("Skipped 1"));
    }

    #[test]
    fn test_orphan_list_render_colors() {
        // Test that colored rendering doesn't panic and includes content
        let orphans = vec![OrphanFile {
            key: "home:~/.claude/commands/test.md".to_string(),
            path: "~/.claude/commands/test.md".to_string(),
            has_signature: true,
            exists: true,
        }];

        // Should not panic with colors enabled
        let colored = render_orphan_list(&orphans, true, true);
        assert!(colored.contains("test.md"));
        assert!(colored.contains("[safe]"));

        // Should not panic with ASCII mode
        let ascii = render_orphan_list(&orphans, false, false);
        assert!(ascii.contains("test.md"));
    }
}
