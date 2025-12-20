//! Check command engine implementation

use anyhow::Result;

use calvin::presentation::ColorWhen;

pub fn cmd_check(
    mode: &str,
    strict_warnings: bool,
    json: bool,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    use calvin::application::{CheckOptions, CheckUseCase};
    use calvin::config::SecurityMode;

    let security_mode = super::parse_security_mode(mode, SecurityMode::Balanced);

    // Use current directory as project root
    let project_root = std::env::current_dir()?;
    let config = calvin::config::Config::load_or_default(Some(&project_root));
    let ui = crate::ui::context::UiContext::new(json, verbose, color, no_animation, &config);

    if json {
        let mut out = std::io::stdout().lock();
        let _ = crate::ui::json::write_event(
            &mut out,
            &serde_json::json!({
                "event": "start",
                "command": "check",
                "mode": format!("{:?}", security_mode),
                "strict_warnings": strict_warnings,
                "verbose": verbose,
            }),
        );
    }

    if !json {
        print!(
            "{}",
            crate::ui::views::check::render_check_header(
                "Calvin Check",
                security_mode,
                strict_warnings,
                ui.color,
                ui.unicode
            )
        );
    }

    // Create CheckUseCase
    let use_case = CheckUseCase::new(config.clone());
    let options = CheckOptions {
        mode: security_mode,
        strict_warnings,
    };

    let result = if json {
        execute_json(&use_case, &project_root, options)?
    } else if ui.animation {
        execute_animated(&use_case, &project_root, options, &ui)?
    } else {
        use_case.execute(&project_root, options)?
    };

    // Determine exit status
    let has_issues = if strict_warnings {
        result.errors > 0 || result.warnings > 0
    } else {
        result.errors > 0
    };

    if json {
        emit_json_complete(&result, security_mode, strict_warnings, has_issues);
    } else {
        render_result(&result, verbose, has_issues, &ui);
    }

    // Emit GitHub Actions annotations if in CI
    if !json && ui.caps.is_ci && std::env::var("GITHUB_ACTIONS").is_ok() {
        emit_github_annotations(&result);
    }

    if has_issues {
        std::process::exit(1);
    }

    Ok(())
}

fn execute_json(
    use_case: &calvin::application::CheckUseCase,
    project_root: &std::path::Path,
    options: calvin::application::CheckOptions,
) -> Result<calvin::application::CheckResult> {
    use calvin::application::CheckStatus;

    let mut out = std::io::stdout().lock();
    use_case.execute_with_callback(project_root, options, |item| {
        let status = match item.status {
            CheckStatus::Pass => "pass",
            CheckStatus::Warning => "warning",
            CheckStatus::Error => "error",
        };
        let _ = crate::ui::json::write_event(
            &mut out,
            &serde_json::json!({
                "event": "check",
                "command": "check",
                "platform": item.platform,
                "name": item.name,
                "status": status,
                "message": item.message,
                "recommendation": item.recommendation,
                "details": item.details,
            }),
        );
    })
}

fn execute_animated(
    use_case: &calvin::application::CheckUseCase,
    project_root: &std::path::Path,
    options: calvin::application::CheckOptions,
    ui: &crate::ui::context::UiContext,
) -> Result<calvin::application::CheckResult> {
    use crate::ui::components::stream::{ItemStatus, StreamOutput};
    use crate::ui::live_region::LiveRegion;
    use calvin::application::CheckStatus;

    let mut list = StreamOutput::with_visible_count(10);
    let mut region = LiveRegion::new();
    let mut stdout = std::io::stdout().lock();

    let result = use_case.execute_with_callback(project_root, options, |item| {
        let index = list.len();
        list.add(format!(
            "{} {} - {}",
            item.platform, item.name, item.message
        ));
        let status = match item.status {
            CheckStatus::Pass => ItemStatus::Success,
            CheckStatus::Warning => ItemStatus::Warning,
            CheckStatus::Error => ItemStatus::Error,
        };
        list.update(index, status);
        if let Some(rec) = &item.recommendation {
            use crate::ui::primitives::icon::Icon;
            list.update_detail(index, format!("{} {}", Icon::Arrow.render(ui.unicode), rec));
        }
        let _ = region.update(&mut stdout, &list.render(ui.color, ui.unicode));
    })?;

    let _ = region.clear(&mut stdout);
    Ok(result)
}

fn emit_json_complete(
    result: &calvin::application::CheckResult,
    security_mode: calvin::config::SecurityMode,
    strict_warnings: bool,
    has_issues: bool,
) {
    let mut out = std::io::stdout().lock();
    let _ = crate::ui::json::write_event(
        &mut out,
        &serde_json::json!({
            "event": "complete",
            "command": "check",
            "mode": format!("{:?}", security_mode),
            "strict_warnings": strict_warnings,
            "passes": result.passed,
            "warnings": result.warnings,
            "errors": result.errors,
            "success": !has_issues
        }),
    );
}

fn render_result(
    result: &calvin::application::CheckResult,
    verbose: u8,
    has_issues: bool,
    ui: &crate::ui::context::UiContext,
) {
    // Convert CheckResult items to legacy report format for rendering
    let legacy_report = convert_to_legacy_report(result);
    print!(
        "{}",
        crate::ui::views::check::render_check_report(&legacy_report, verbose, ui.color, ui.unicode)
    );
    print!(
        "\n{}",
        crate::ui::views::check::render_check_summary(
            result.passed,
            result.warnings,
            result.errors,
            has_issues,
            ui.color,
            ui.unicode
        )
    );
}

fn emit_github_annotations(result: &calvin::application::CheckResult) {
    use calvin::application::CheckStatus;

    for item in &result.items {
        let level = match item.status {
            CheckStatus::Pass => continue,
            CheckStatus::Warning => crate::ui::ci::AnnotationLevel::Warning,
            CheckStatus::Error => crate::ui::ci::AnnotationLevel::Error,
        };
        let title = format!("{} {}", item.platform, item.name);
        println!(
            "{}",
            crate::ui::ci::github_actions_annotation(
                level,
                &item.message,
                None,
                None,
                Some(&title)
            )
        );
    }
}

/// Convert CheckResult to legacy report format for UI rendering
fn convert_to_legacy_report(
    result: &calvin::application::CheckResult,
) -> calvin::security::DoctorReport {
    use calvin::application::CheckStatus as AppStatus;
    use calvin::security::{CheckStatus as LegacyStatus, DoctorReport, SecurityCheck};

    let checks: Vec<SecurityCheck> = result
        .items
        .iter()
        .map(|item| SecurityCheck {
            platform: item.platform.clone(),
            name: item.name.clone(),
            status: match item.status {
                AppStatus::Pass => LegacyStatus::Pass,
                AppStatus::Warning => LegacyStatus::Warning,
                AppStatus::Error => LegacyStatus::Error,
            },
            message: item.message.clone(),
            recommendation: item.recommendation.clone(),
            details: item.details.clone(),
        })
        .collect();

    DoctorReport { checks }
}
