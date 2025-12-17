use anyhow::Result;

pub fn cmd_check(mode: &str, strict_warnings: bool, json: bool, verbose: u8) -> Result<()> {
    use calvin::config::SecurityMode;
    use calvin::security::{run_doctor, CheckStatus};

    let security_mode = parse_security_mode(mode, SecurityMode::Balanced);

    // Use current directory as project root
    let project_root = std::env::current_dir()?;

    if !json {
        println!("🔍 Calvin Check");
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
        let output = serde_json::json!({
            "event": "check",
            "mode": format!("{:?}", security_mode),
            "strict_warnings": strict_warnings,
            "passes": report.passes(),
            "warnings": report.warnings(),
            "errors": report.errors(),
            "success": !has_issues
        });
        println!("{}", serde_json::to_string(&output)?);
    } else {
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
                CheckStatus::Pass => "✓",
                CheckStatus::Warning => "⚠",
                CheckStatus::Error => "✗",
            };

            println!("  {} {} - {}", icon, check.name, check.message);

            if let Some(rec) = &check.recommendation {
                println!("    ↳ {}", rec);
            }

            if verbose > 0 && !check.details.is_empty() {
                for detail in &check.details {
                    println!("    ↳ {}", detail);
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

        if has_issues {
            println!();
            println!("🔴 Check FAILED - issues detected");
        } else if report.warnings() > 0 {
            println!();
            println!("🟡 Check passed with warnings.");
        } else {
            println!();
            println!("🟢 All checks passed!");
        }
    }

    if has_issues {
        std::process::exit(1);
    }

    Ok(())
}

pub fn cmd_doctor(mode: &str, json: bool, verbose: u8) -> Result<()> {
    use calvin::config::SecurityMode;
    use calvin::security::{run_doctor, CheckStatus};

    let security_mode = parse_security_mode(mode, SecurityMode::Balanced);

    // Use current directory as project root
    let project_root = std::env::current_dir()?;

    if !json {
        println!("🩺 Calvin Doctor");
        println!("Mode: {:?}", security_mode);
        println!();
    }

    let report = run_doctor(&project_root, security_mode);

    if json {
        let output = serde_json::json!({
            "event": "doctor",
            "mode": format!("{:?}", security_mode),
            "passes": report.passes(),
            "warnings": report.warnings(),
            "errors": report.errors(),
            "success": report.is_success()
        });
        println!("{}", serde_json::to_string(&output)?);
    } else {
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
                CheckStatus::Pass => "✓",
                CheckStatus::Warning => "⚠",
                CheckStatus::Error => "✗",
            };

            println!("  {} {} - {}", icon, check.name, check.message);

            if let Some(rec) = &check.recommendation {
                println!("    ↳ {}", rec);
            }

            if verbose > 0 && !check.details.is_empty() {
                for detail in &check.details {
                    println!("    ↳ {}", detail);
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
            println!("🔴 Doctor found issues. Run with --mode balanced or fix the errors.");
        } else if report.warnings() > 0 {
            println!();
            println!("🟡 Doctor passed with warnings.");
        } else {
            println!();
            println!("🟢 All checks passed!");
        }
    }

    Ok(())
}

pub fn cmd_audit(mode: &str, strict_warnings: bool, json: bool, _verbose: u8) -> Result<()> {
    use calvin::config::SecurityMode;
    use calvin::security::{run_doctor, CheckStatus};

    let security_mode = parse_security_mode(mode, SecurityMode::Strict);

    let project_root = std::env::current_dir()?;

    if !json {
        println!("🔒 Calvin Security Audit");
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
        let output = serde_json::json!({
            "event": "audit",
            "mode": format!("{:?}", security_mode),
            "passes": report.passes(),
            "warnings": report.warnings(),
            "errors": report.errors(),
            "success": !has_issues
        });
        println!("{}", serde_json::to_string(&output)?);
    } else {
        for check in &report.checks {
            let icon = match check.status {
                CheckStatus::Pass => "✓",
                CheckStatus::Warning => "⚠",
                CheckStatus::Error => "✗",
            };
            println!("{} [{}] {}: {}", icon, check.platform, check.name, check.message);
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
            println!("🔴 Audit FAILED - security issues detected");
        }
        std::process::exit(1);
    } else if !json {
        println!();
        println!("🟢 Audit PASSED");
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
