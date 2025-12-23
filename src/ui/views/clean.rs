//! Clean command UI views
//!
//! Provides consistent rendering for the clean command output.

use std::path::Path;

use crate::ui::blocks::header::CommandHeader;
use crate::ui::blocks::summary::ResultSummary;
use crate::ui::primitives::icon::Icon;
use crate::ui::primitives::text::ColoredText;
use calvin::application::clean::CleanResult;
use calvin::domain::value_objects::Scope;

/// Render the clean command header
pub fn render_clean_header(
    source: &Path,
    scope: Option<Scope>,
    dry_run: bool,
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    let action = if dry_run { "Clean (Dry Run)" } else { "Clean" };
    let mut header = CommandHeader::new(Icon::Trash, format!("Calvin {}", action));

    header.add("Source", source.display().to_string());

    let scope_label = match scope {
        Some(Scope::User) => "Home (~)",
        Some(Scope::Project) => "Project",
        None => "All",
    };
    header.add("Scope", scope_label);

    header.render(supports_color, supports_unicode)
}

/// Render a preview list of files to be cleaned
pub fn render_clean_preview(
    result: &CleanResult,
    supports_color: bool,
    _supports_unicode: bool,
) -> String {
    let mut out = String::new();

    if !result.deleted.is_empty() {
        out.push_str(
            &ColoredText::warning("Files to be deleted:")
                .bold()
                .render(supports_color),
        );
        out.push('\n');
        for deleted in &result.deleted {
            out.push_str(&format!("  - {}\n", deleted.path.display()));
        }
    }

    if !result.skipped.is_empty() {
        out.push('\n');
        out.push_str(&ColoredText::dim("Files to be skipped:").render(supports_color));
        out.push('\n');
        for skipped in &result.skipped {
            out.push_str(&format!(
                "  - {} ({})\n",
                skipped.path.display(),
                skipped.reason
            ));
        }
    }

    out
}

/// Render the clean result summary
pub fn render_clean_result(
    result: &CleanResult,
    dry_run: bool,
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    let title = if dry_run {
        "Dry Run Complete"
    } else if result.is_success() && !result.deleted.is_empty() {
        "Clean Complete"
    } else if result.deleted.is_empty() && result.skipped.is_empty() {
        "Nothing to Clean"
    } else {
        "Clean Complete"
    };

    let has_issues = !result.errors.is_empty();
    let mut summary = if has_issues {
        ResultSummary::partial(title)
    } else {
        ResultSummary::success(title)
    };

    if dry_run {
        summary.add_stat("files would be deleted", result.deleted.len());
        summary.add_stat("files would be skipped", result.skipped.len());
    } else {
        summary.add_stat("files deleted", result.deleted.len());
        summary.add_stat("files skipped", result.skipped.len());
    }

    if !result.errors.is_empty() {
        summary.add_stat("errors", result.errors.len());
        for error in &result.errors {
            summary.add_warning(format!("{}", error));
        }
    }

    if dry_run && !result.deleted.is_empty() {
        summary.with_next_step("Run without --dry-run to delete");
    } else if !dry_run && result.deleted.is_empty() && result.skipped.is_empty() {
        summary.with_next_step("No deployed files found");
    }

    summary.render(supports_color, supports_unicode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use calvin::application::clean::SkipReason;
    use std::path::PathBuf;

    #[test]
    fn header_includes_source() {
        let rendered = render_clean_header(Path::new(".promptpack"), None, false, false, false);
        assert!(rendered.contains("Source: .promptpack"));
    }

    #[test]
    fn header_shows_dry_run_mode() {
        let rendered = render_clean_header(
            Path::new(".promptpack"),
            Some(Scope::User),
            true,
            false,
            false,
        );
        assert!(rendered.contains("Dry Run"));
        assert!(rendered.contains("Home"));
    }

    #[test]
    fn preview_lists_deleted_files() {
        let mut result = CleanResult::new();
        result.add_deleted(
            PathBuf::from("/path/to/file.md"),
            "home:/path/to/file.md".to_string(),
        );

        let rendered = render_clean_preview(&result, false, false);
        assert!(rendered.contains("/path/to/file.md"));
        assert!(rendered.contains("deleted"));
    }

    #[test]
    fn preview_lists_skipped_files() {
        let mut result = CleanResult::new();
        result.add_skipped(
            PathBuf::from("/path/to/file.md"),
            SkipReason::Modified,
            "home:/path/to/file.md".to_string(),
        );

        let rendered = render_clean_preview(&result, false, false);
        assert!(rendered.contains("/path/to/file.md"));
        assert!(rendered.contains("modified"));
    }

    #[test]
    fn summary_shows_deleted_count() {
        let mut result = CleanResult::new();
        result.add_deleted(PathBuf::from("a"), "home:a".to_string());
        result.add_deleted(PathBuf::from("b"), "home:b".to_string());

        let rendered = render_clean_result(&result, false, false, false);
        assert!(rendered.contains("2 files deleted"));
    }

    #[test]
    fn summary_shows_dry_run_wording() {
        let mut result = CleanResult::new();
        result.add_deleted(PathBuf::from("a"), "home:a".to_string());

        let rendered = render_clean_result(&result, true, false, false);
        assert!(rendered.contains("would be deleted"));
    }
}
