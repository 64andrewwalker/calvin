use std::path::Path;

use crate::ui::blocks::header::CommandHeader;
use crate::ui::blocks::summary::ResultSummary;
use crate::ui::primitives::icon::Icon;
use calvin::application::DeployResult;

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
    result: &DeployResult,
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    // Determine the overall status
    let all_skipped =
        result.written.is_empty() && !result.skipped.is_empty() && result.errors.is_empty();
    let nothing_to_do =
        result.written.is_empty() && result.skipped.is_empty() && result.errors.is_empty();
    let has_errors = !result.errors.is_empty();
    let has_writes = !result.written.is_empty();

    let title = if all_skipped || nothing_to_do {
        "Already Up-to-date".to_string()
    } else if has_errors {
        format!("{action} Results")
    } else if has_writes {
        format!("{action} Complete")
    } else {
        format!("{action} Results")
    };

    let mut summary = if all_skipped || nothing_to_do {
        // All files already match or nothing to process - this is success
        ResultSummary::success(title)
    } else if has_errors {
        ResultSummary::partial(title)
    } else if has_writes {
        ResultSummary::success(title)
    } else {
        ResultSummary::partial(title)
    };

    summary.add_stat(format!("assets → {} targets", target_count), asset_count);
    summary.add_stat("files written", result.written.len());

    if all_skipped {
        // When all files are skipped because they're up-to-date, show a positive message
        summary.add_info(format!("{} files already up-to-date", result.skipped.len()));
    } else if !result.skipped.is_empty() {
        summary.add_stat("skipped", result.skipped.len());
    }

    summary.add_stat("errors", result.errors.len());

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
    use std::path::PathBuf;

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
        let result = DeployResult {
            written: vec![PathBuf::from("a")],
            skipped: vec![],
            deleted: vec![],
            errors: vec![],
            asset_count: 1,
            output_count: 1,
        };

        let rendered = render_deploy_summary("Deploy", 1, 1, &result, false, false);
        assert!(rendered.contains("1 files written"));
    }
}
