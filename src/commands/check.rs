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
        use crate::ui::live_region::LiveRegion;
        use crate::ui::components::stream::{ItemStatus, StreamOutput};

        let mut list = StreamOutput::with_visible_count(10);
        let mut region = LiveRegion::new();
        let mut stdout = std::io::stdout().lock();

        let report = run_doctor_with_callback(&project_root, security_mode, |check| {
            let index = list.len();
            list.add(format!("{} {} - {}", check.platform, check.name, check.message));
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
                crate::ui::ci::github_actions_annotation(level, &check.message, None, None, Some(&title))
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
        println!("{} Calvin Doctor", Icon::Check.colored(ui.color, ui.unicode));
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
                    println!("    {} {}", Icon::Arrow.colored(ui.color, ui.unicode), detail);
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
            println!("{} All checks passed!", Icon::Success.colored(ui.color, ui.unicode));
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
        println!("{} Calvin Security Audit", Icon::Warning.colored(ui.color, ui.unicode));
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

fn parse_security_mode(mode: &str, default: calvin::config::SecurityMode) -> calvin::config::SecurityMode {
    use calvin::config::SecurityMode;

    match mode {
        "yolo" => SecurityMode::Yolo,
        "balanced" => SecurityMode::Balanced,
        "strict" => SecurityMode::Strict,
        _ => default,
    }
}
