//! Doctor command implementation

use anyhow::Result;

use calvin::presentation::ColorWhen;

pub fn cmd_doctor(
    mode: &str,
    json: bool,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    use calvin::config::SecurityMode;
    use calvin::security::{run_doctor, CheckStatus};

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
