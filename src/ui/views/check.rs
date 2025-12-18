use calvin::config::SecurityMode;
use calvin::security::{CheckStatus, DoctorReport};
use crate::ui::blocks::check_item::{CheckItem, CheckStatus as UiCheckStatus};
use crate::ui::blocks::header::CommandHeader;
use crate::ui::blocks::summary::ResultSummary;
use crate::ui::primitives::icon::Icon;

pub fn render_check_header(
    title: &str,
    mode: SecurityMode,
    strict_warnings: bool,
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    let mut header = CommandHeader::new(Icon::Check, title);
    header.add("Mode", format!("{:?}", mode));
    if strict_warnings {
        header.add("Strict", "failing on warnings");
    }
    header.render(supports_color, supports_unicode)
}

pub fn render_check_report(
    report: &DoctorReport,
    verbose: u8,
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    let mut out = String::new();

    let mut current_platform: Option<&str> = None;
    for check in &report.checks {
        if current_platform != Some(check.platform.as_str()) {
            if current_platform.is_some() {
                out.push('\n');
            }
            out.push_str(&check.platform);
            out.push('\n');
            current_platform = Some(check.platform.as_str());
        }

        let status = match check.status {
            CheckStatus::Pass => UiCheckStatus::Pass,
            CheckStatus::Warning => UiCheckStatus::Warning,
            CheckStatus::Error => UiCheckStatus::Error,
        };

        let item = CheckItem {
            name: check.name.clone(),
            status,
            message: check.message.clone(),
            recommendation: check.recommendation.clone(),
            details: check.details.clone(),
        };

        out.push_str(&item.render(verbose > 0, supports_color, supports_unicode));
    }

    out
}

pub fn render_check_summary(
    passes: usize,
    warnings: usize,
    errors: usize,
    has_issues: bool,
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    let title = if has_issues {
        "Check FAILED"
    } else if warnings > 0 {
        "Check passed with warnings"
    } else {
        "All checks passed"
    };

    let mut summary = if has_issues || warnings > 0 {
        ResultSummary::partial(title)
    } else {
        ResultSummary::success(title)
    };

    summary.add_stat("passed", passes);
    summary.add_stat("warnings", warnings);
    summary.add_stat("errors", errors);
    summary.with_next_step("Run `calvin deploy` to sync your prompts");

    summary.render(supports_color, supports_unicode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_platform_header() {
        let mut report = DoctorReport::new();
        report.add_pass("Codex", "commands", "ok");

        let rendered = render_check_report(&report, 0, false, false);
        assert!(rendered.contains("Codex\n"));
    }
}
