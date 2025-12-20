use anyhow::Result;

use crate::cli::ColorWhen;

pub fn cmd_check(
    mode: &str,
    strict_warnings: bool,
    json: bool,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    // Check if we should use the new engine
    if should_use_new_check_engine() {
        cmd_check_new_engine(mode, strict_warnings, json, verbose, color, no_animation)
    } else {
        cmd_check_legacy(mode, strict_warnings, json, verbose, color, no_animation)
    }
}

/// Check if we should use the new check engine
fn should_use_new_check_engine() -> bool {
    !std::env::var("CALVIN_LEGACY_CHECK").is_ok_and(|v| v == "1" || v.to_lowercase() == "true")
}

/// New engine implementation using CheckUseCase
fn cmd_check_new_engine(
    mode: &str,
    strict_warnings: bool,
    json: bool,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    use calvin::application::{CheckOptions, CheckStatus, CheckUseCase};
    use calvin::config::SecurityMode;

    let security_mode = parse_security_mode(mode, SecurityMode::Balanced);

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
        let mut out = std::io::stdout().lock();
        use_case.execute_with_callback(&project_root, options, |item| {
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
        })?
    } else if ui.animation {
        use crate::ui::components::stream::{ItemStatus, StreamOutput};
        use crate::ui::live_region::LiveRegion;

        let mut list = StreamOutput::with_visible_count(10);
        let mut region = LiveRegion::new();
        let mut stdout = std::io::stdout().lock();

        let result = use_case.execute_with_callback(&project_root, options, |item| {
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
        result
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
    } else {
        // Convert CheckResult items to legacy report format for rendering
        let legacy_report = convert_to_legacy_report(&result);
        print!(
            "{}",
            crate::ui::views::check::render_check_report(
                &legacy_report,
                verbose,
                ui.color,
                ui.unicode
            )
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

    if !json && ui.caps.is_ci && std::env::var("GITHUB_ACTIONS").is_ok() {
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

    if has_issues {
        std::process::exit(1);
    }

    Ok(())
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

/// Legacy engine implementation (original)
fn cmd_check_legacy(
    mode: &str,
    strict_warnings: bool,
    json: bool,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    use calvin::config::SecurityMode;
    use calvin::security::{run_doctor, run_doctor_with_callback, CheckStatus};

    let security_mode = parse_security_mode(mode, SecurityMode::Balanced);

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

    let report = if json {
        use calvin::security::CheckStatus;
        let mut out = std::io::stdout().lock();
        run_doctor_with_callback(&project_root, security_mode, |check| {
            let status = match check.status {
                CheckStatus::Pass => "pass",
                CheckStatus::Warning => "warning",
                CheckStatus::Error => "error",
            };
            let _ = crate::ui::json::write_event(
                &mut out,
                &serde_json::json!({
                    "event": "check",
                    "command": "check",
                    "platform": check.platform,
                    "name": check.name,
                    "status": status,
                    "message": check.message,
                    "recommendation": check.recommendation,
                    "details": check.details,
                }),
            );
        })
    } else if ui.animation {
        use crate::ui::components::stream::{ItemStatus, StreamOutput};
        use crate::ui::live_region::LiveRegion;

        let mut list = StreamOutput::with_visible_count(10);
        let mut region = LiveRegion::new();
        let mut stdout = std::io::stdout().lock();

        let report = run_doctor_with_callback(&project_root, security_mode, |check| {
            let index = list.len();
            list.add(format!(
                "{} {} - {}",
                check.platform, check.name, check.message
            ));
            let status = match check.status {
                CheckStatus::Pass => ItemStatus::Success,
                CheckStatus::Warning => ItemStatus::Warning,
                CheckStatus::Error => ItemStatus::Error,
            };
            list.update(index, status);
            if let Some(rec) = &check.recommendation {
                use crate::ui::primitives::icon::Icon;
                list.update_detail(index, format!("{} {}", Icon::Arrow.render(ui.unicode), rec));
            }
            let _ = region.update(&mut stdout, &list.render(ui.color, ui.unicode));
        });

        let _ = region.clear(&mut stdout);
        report
    } else {
        run_doctor(&project_root, security_mode)
    };

    // Determine exit status
    let has_issues = if strict_warnings {
        report.errors() > 0 || report.warnings() > 0
    } else {
        report.errors() > 0
    };

    if json {
        let mut out = std::io::stdout().lock();
        let _ = crate::ui::json::write_event(
            &mut out,
            &serde_json::json!({
                "event": "complete",
                "command": "check",
                "mode": format!("{:?}", security_mode),
                "strict_warnings": strict_warnings,
                "passes": report.passes(),
                "warnings": report.warnings(),
                "errors": report.errors(),
                "success": !has_issues
            }),
        );
    } else {
        print!(
            "{}",
            crate::ui::views::check::render_check_report(&report, verbose, ui.color, ui.unicode)
        );
        print!(
            "\n{}",
            crate::ui::views::check::render_check_summary(
                report.passes(),
                report.warnings(),
                report.errors(),
                has_issues,
                ui.color,
                ui.unicode
            )
        );
    }

    if !json && ui.caps.is_ci && std::env::var("GITHUB_ACTIONS").is_ok() {
        for check in &report.checks {
            let level = match check.status {
                CheckStatus::Pass => continue,
                CheckStatus::Warning => crate::ui::ci::AnnotationLevel::Warning,
                CheckStatus::Error => crate::ui::ci::AnnotationLevel::Error,
            };
            let title = format!("{} {}", check.platform, check.name);
            println!(
                "{}",
                crate::ui::ci::github_actions_annotation(
                    level,
                    &check.message,
                    None,
                    None,
                    Some(&title)
                )
            );
        }
    }

    if has_issues {
        std::process::exit(1);
    }

    Ok(())
}

pub fn cmd_doctor(
    mode: &str,
    json: bool,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    use calvin::config::SecurityMode;
    use calvin::security::{run_doctor, CheckStatus};

    let security_mode = parse_security_mode(mode, SecurityMode::Balanced);

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
                "command": "doctor",
                "mode": format!("{:?}", security_mode),
                "verbose": verbose,
            }),
        );
    }

    if !json {
        use crate::ui::primitives::icon::Icon;
        println!(
            "{} Calvin Doctor",
            Icon::Check.colored(ui.color, ui.unicode)
        );
        println!("Mode: {:?}", security_mode);
        println!();
    }

    let report = run_doctor(&project_root, security_mode);

    if json {
        let mut out = std::io::stdout().lock();
        let _ = crate::ui::json::write_event(
            &mut out,
            &serde_json::json!({
                "event": "complete",
                "command": "doctor",
                "mode": format!("{:?}", security_mode),
                "passes": report.passes(),
                "warnings": report.warnings(),
                "errors": report.errors(),
                "success": report.is_success()
            }),
        );
    } else {
        use crate::ui::primitives::icon::Icon;

        // Group by platform
        let mut current_platform = String::new();

        for check in &report.checks {
            if check.platform != current_platform {
                if !current_platform.is_empty() {
                    println!();
                }
                println!("{}", check.platform);
                current_platform = check.platform.clone();
            }

            let icon = match check.status {
                CheckStatus::Pass => Icon::Success,
                CheckStatus::Warning => Icon::Warning,
                CheckStatus::Error => Icon::Error,
            }
            .colored(ui.color, ui.unicode);

            println!("  {} {} - {}", icon, check.name, check.message);

            if let Some(rec) = &check.recommendation {
                println!("    {} {}", Icon::Arrow.colored(ui.color, ui.unicode), rec);
            }

            if verbose > 0 && !check.details.is_empty() {
                for detail in &check.details {
                    println!(
                        "    {} {}",
                        Icon::Arrow.colored(ui.color, ui.unicode),
                        detail
                    );
                }
            }
        }

        println!();
        println!(
            "Summary: {} passed, {} warnings, {} errors",
            report.passes(),
            report.warnings(),
            report.errors()
        );

        if !report.is_success() {
            println!();
            println!(
                "{} Doctor found issues. Run with --mode balanced or fix the errors.",
                Icon::Error.colored(ui.color, ui.unicode)
            );
        } else if report.warnings() > 0 {
            println!();
            println!(
                "{} Doctor passed with warnings.",
                Icon::Warning.colored(ui.color, ui.unicode)
            );
        } else {
            println!();
            println!(
                "{} All checks passed!",
                Icon::Success.colored(ui.color, ui.unicode)
            );
        }
    }

    Ok(())
}

pub fn cmd_audit(
    mode: &str,
    strict_warnings: bool,
    json: bool,
    _verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    use calvin::config::SecurityMode;
    use calvin::security::{run_doctor, CheckStatus};

    let security_mode = parse_security_mode(mode, SecurityMode::Strict);

    let project_root = std::env::current_dir()?;
    let config = calvin::config::Config::load_or_default(Some(&project_root));
    let ui = crate::ui::context::UiContext::new(json, 0, color, no_animation, &config);

    if json {
        let mut out = std::io::stdout().lock();
        let _ = crate::ui::json::write_event(
            &mut out,
            &serde_json::json!({
                "event": "start",
                "command": "audit",
                "mode": format!("{:?}", security_mode),
                "strict_warnings": strict_warnings,
            }),
        );
    }

    if !json {
        use crate::ui::primitives::icon::Icon;
        println!(
            "{} Calvin Security Audit",
            Icon::Warning.colored(ui.color, ui.unicode)
        );
        println!("Mode: {:?}", security_mode);
        if strict_warnings {
            println!("Strict: failing on warnings");
        }
        println!();
    }

    let report = run_doctor(&project_root, security_mode);

    // Determine exit status
    let has_issues = if strict_warnings {
        report.errors() > 0 || report.warnings() > 0
    } else {
        report.errors() > 0
    };

    if json {
        let mut out = std::io::stdout().lock();
        let _ = crate::ui::json::write_event(
            &mut out,
            &serde_json::json!({
                "event": "complete",
                "command": "audit",
                "mode": format!("{:?}", security_mode),
                "passes": report.passes(),
                "warnings": report.warnings(),
                "errors": report.errors(),
                "success": !has_issues
            }),
        );
    } else {
        use crate::ui::primitives::icon::Icon;

        for check in &report.checks {
            let icon = match check.status {
                CheckStatus::Pass => Icon::Success,
                CheckStatus::Warning => Icon::Warning,
                CheckStatus::Error => Icon::Error,
            }
            .colored(ui.color, ui.unicode);
            println!(
                "{} [{}] {}: {}",
                icon, check.platform, check.name, check.message
            );
        }

        println!();
        println!(
            "Result: {} passed, {} warnings, {} errors",
            report.passes(),
            report.warnings(),
            report.errors()
        );
    }

    if has_issues {
        if !json {
            println!();
            println!(
                "{} Audit FAILED - security issues detected",
                crate::ui::primitives::icon::Icon::Error.colored(ui.color, ui.unicode)
            );
        }
        std::process::exit(1);
    } else if !json {
        println!();
        println!(
            "{} Audit PASSED",
            crate::ui::primitives::icon::Icon::Success.colored(ui.color, ui.unicode)
        );
    }

    Ok(())
}

fn parse_security_mode(
    mode: &str,
    default: calvin::config::SecurityMode,
) -> calvin::config::SecurityMode {
    use calvin::config::SecurityMode;

    match mode {
        "yolo" => SecurityMode::Yolo,
        "balanced" => SecurityMode::Balanced,
        "strict" => SecurityMode::Strict,
        _ => default,
    }
}
