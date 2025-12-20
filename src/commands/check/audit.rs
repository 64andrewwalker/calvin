//! Audit command implementation

use anyhow::Result;

use calvin::presentation::ColorWhen;

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

    let security_mode = super::parse_security_mode(mode, SecurityMode::Strict);

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
