use std::path::Path;

use calvin::SyncResult;
use crate::ui::blocks::header::CommandHeader;
use crate::ui::blocks::summary::ResultSummary;
use crate::ui::primitives::icon::Icon;

pub fn render_deploy_header(
    action: &str,
    source: &Path,
    target: Option<&str>,
    remote: Option<&str>,
    modes: &[String],
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    let mut header = CommandHeader::new(Icon::Deploy, format!("Calvin {}", action));
    header.add("Source", source.display().to_string());

    if let Some(target) = target {
        header.add("Target", target);
    }
    if let Some(remote) = remote {
        header.add("Remote", remote);
    }
    for mode in modes {
        header.add("Mode", mode);
    }

    header.render(supports_color, supports_unicode)
}

pub fn render_deploy_summary(
    action: &str,
    asset_count: usize,
    target_count: usize,
    result: &SyncResult,
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    let title = if result.is_success() && result.skipped.is_empty() {
        format!("{action} Complete")
    } else {
        format!("{action} Results")
    };

    let mut summary = if result.is_success() && result.skipped.is_empty() {
        ResultSummary::success(title)
    } else {
        ResultSummary::partial(title)
    };

    summary.add_stat(format!("assets → {} targets", target_count), asset_count);
    summary.add_stat("files written", result.written.len());
    summary.add_stat("skipped", result.skipped.len());
    summary.add_stat("errors", result.errors.len());

    if !result.skipped.is_empty() {
        summary.add_warning(format!("{} files skipped", result.skipped.len()));
    }
    if !result.errors.is_empty() {
        summary.add_warning(format!("{} errors encountered", result.errors.len()));
    }

    if action.eq_ignore_ascii_case("deploy") {
        summary.with_next_step("Run `calvin check` to verify");
    }

    summary.render(supports_color, supports_unicode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_includes_source_label() {
        let rendered = render_deploy_header(
            "Deploy",
            Path::new(".promptpack"),
            None,
            None,
            &[],
            false,
            false,
        );
        assert!(rendered.contains("Source: .promptpack"));
    }

    #[test]
    fn summary_includes_written_stat() {
        let mut result = SyncResult::new();
        result.written.push("a".to_string());

        let rendered = render_deploy_summary("Deploy", 1, 1, &result, false, false);
        assert!(rendered.contains("1 files written"));
    }
}
