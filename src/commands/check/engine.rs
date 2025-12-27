//! Check command engine implementation

use anyhow::Result;

use calvin::presentation::ColorWhen;

#[allow(clippy::too_many_arguments)]
pub fn cmd_check(
    mode: &str,
    strict_warnings: bool,
    all: bool,
    all_layers: bool,
    show_ignored: bool,
    debug_ignore: bool,
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

    // Handle --show-ignored and --debug-ignore flags
    if show_ignored || debug_ignore {
        return display_ignore_info(
            &project_root,
            &config,
            show_ignored,
            debug_ignore,
            json,
            &ui,
        );
    }

    let options = CheckOptions {
        mode: security_mode,
        strict_warnings,
    };

    if all {
        let _ = execute_check_all(&project_root, options, all_layers, json, verbose)?;
        return Ok(());
    }

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

    let result = if json {
        execute_json(&use_case, &project_root, options, all_layers)?
    } else if ui.animation {
        execute_animated(&use_case, &project_root, options, all_layers, &ui)?
    } else {
        execute_single_project(&use_case, &project_root, options, all_layers)?
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

/// Display .calvinignore patterns and/or ignored files for each layer.
fn display_ignore_info(
    project_root: &std::path::Path,
    config: &calvin::config::Config,
    show_ignored: bool,
    debug_ignore: bool,
    json: bool,
    _ui: &crate::ui::context::UiContext,
) -> Result<()> {
    use calvin::application::layers::LayerQueryUseCase;
    use calvin::domain::value_objects::IgnorePatterns;

    let use_case = LayerQueryUseCase::default();
    let layer_result = use_case.query(project_root, config)?;

    if json {
        let mut out = std::io::stdout().lock();
        for layer in &layer_result.layers {
            let ignore = IgnorePatterns::load(&layer.resolved_path)?;

            let mut layer_info = serde_json::json!({
                "event": "ignore_info",
                "command": "check",
                "layer": layer.name,
                "path": layer.resolved_path.display().to_string(),
                "pattern_count": ignore.pattern_count(),
            });

            if show_ignored && !ignore.is_empty() {
                // Read the .calvinignore file content to get patterns
                let calvinignore_path = layer.resolved_path.join(".calvinignore");
                if calvinignore_path.exists() {
                    let content = std::fs::read_to_string(&calvinignore_path)?;
                    let patterns: Vec<&str> = content
                        .lines()
                        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
                        .collect();
                    layer_info["patterns"] = serde_json::json!(patterns);
                }
            }

            let _ = crate::ui::json::write_event(&mut out, &layer_info);
        }
        return Ok(());
    }

    // Text output
    println!();
    println!(".calvinignore Patterns");
    println!("======================");
    println!();

    for layer in &layer_result.layers {
        let ignore = IgnorePatterns::load(&layer.resolved_path)?;

        println!("  {} ({} patterns)", layer.name, ignore.pattern_count(),);

        if (show_ignored || debug_ignore) && !ignore.is_empty() {
            let calvinignore_path = layer.resolved_path.join(".calvinignore");
            if calvinignore_path.exists() {
                let content = std::fs::read_to_string(&calvinignore_path)?;
                for line in content.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() && !trimmed.starts_with('#') {
                        println!("    • {}", trimmed);
                    }
                }
            }
        }
        println!();
    }

    if debug_ignore {
        println!();
        println!("Note: --debug-ignore individual file listing not yet implemented.");
        println!("Use verbose deploy (-v) for ignored file counts.");
    }

    Ok(())
}

fn execute_json(
    use_case: &calvin::application::CheckUseCase,
    project_root: &std::path::Path,
    options: calvin::application::CheckOptions,
    all_layers: bool,
) -> Result<calvin::application::CheckResult> {
    use calvin::application::CheckStatus;

    let mut out = std::io::stdout().lock();
    let mut result = use_case.execute_with_callback(project_root, options, |item| {
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
    })?;

    if all_layers {
        append_layer_checks_json(project_root, &mut result, &mut out)?;
    }

    Ok(result)
}

fn execute_animated(
    use_case: &calvin::application::CheckUseCase,
    project_root: &std::path::Path,
    options: calvin::application::CheckOptions,
    all_layers: bool,
    ui: &crate::ui::context::UiContext,
) -> Result<calvin::application::CheckResult> {
    use crate::ui::components::stream::{ItemStatus, StreamOutput};
    use crate::ui::live_region::LiveRegion;
    use calvin::application::CheckStatus;

    let mut list = StreamOutput::with_visible_count(10);
    let mut region = LiveRegion::new();
    let mut stdout = std::io::stdout().lock();

    let mut result = use_case.execute_with_callback(project_root, options, |item| {
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
    if all_layers {
        append_layer_checks_text(project_root, &mut result)?;
    }

    let _ = region.clear(&mut stdout);
    Ok(result)
}

fn execute_single_project(
    use_case: &calvin::application::CheckUseCase,
    project_root: &std::path::Path,
    options: calvin::application::CheckOptions,
    all_layers: bool,
) -> Result<calvin::application::CheckResult> {
    let mut result = use_case.execute(project_root, options)?;
    if all_layers {
        append_layer_checks_text(project_root, &mut result)?;
    }
    Ok(result)
}

fn append_layer_checks_text(
    project_root: &std::path::Path,
    result: &mut calvin::application::CheckResult,
) -> Result<()> {
    let config = calvin::config::Config::load_or_default(Some(project_root));
    let use_case = calvin::application::layers::LayerQueryUseCase::default();

    match use_case.query(project_root, &config) {
        Ok(layers) => {
            result.passed += 1;
            result.items.push(calvin::application::CheckItem {
                platform: "layers".to_string(),
                name: "layer_stack".to_string(),
                status: calvin::application::CheckStatus::Pass,
                message: format!("Resolved {} layers", layers.layers.len()),
                recommendation: None,
                details: vec![
                    format!("merged_assets={}", layers.merged_asset_count),
                    format!("overridden_assets={}", layers.overridden_asset_count),
                ],
            });
        }
        Err(e) => {
            result.errors += 1;
            result.items.push(calvin::application::CheckItem {
                platform: "layers".to_string(),
                name: "layer_stack".to_string(),
                status: calvin::application::CheckStatus::Error,
                message: format!("Failed to resolve layers: {}", e),
                recommendation: None,
                details: Vec::new(),
            });
        }
    }

    Ok(())
}

fn append_layer_checks_json(
    project_root: &std::path::Path,
    result: &mut calvin::application::CheckResult,
    out: &mut impl std::io::Write,
) -> Result<()> {
    let config = calvin::config::Config::load_or_default(Some(project_root));
    let use_case = calvin::application::layers::LayerQueryUseCase::default();

    match use_case.query(project_root, &config) {
        Ok(layers) => {
            result.passed += 1;
            let item = calvin::application::CheckItem {
                platform: "layers".to_string(),
                name: "layer_stack".to_string(),
                status: calvin::application::CheckStatus::Pass,
                message: format!("Resolved {} layers", layers.layers.len()),
                recommendation: None,
                details: vec![
                    format!("merged_assets={}", layers.merged_asset_count),
                    format!("overridden_assets={}", layers.overridden_asset_count),
                ],
            };
            result.items.push(item.clone());
            let _ = crate::ui::json::write_event(
                out,
                &serde_json::json!({
                    "event": "check",
                    "command": "check",
                    "platform": item.platform,
                    "name": item.name,
                    "status": "pass",
                    "message": item.message,
                    "recommendation": item.recommendation,
                    "details": item.details,
                }),
            );
        }
        Err(e) => {
            result.errors += 1;
            let item = calvin::application::CheckItem {
                platform: "layers".to_string(),
                name: "layer_stack".to_string(),
                status: calvin::application::CheckStatus::Error,
                message: format!("Failed to resolve layers: {}", e),
                recommendation: None,
                details: Vec::new(),
            };
            result.items.push(item.clone());
            let _ = crate::ui::json::write_event(
                out,
                &serde_json::json!({
                    "event": "check",
                    "command": "check",
                    "platform": item.platform,
                    "name": item.name,
                    "status": "error",
                    "message": item.message,
                    "recommendation": item.recommendation,
                    "details": item.details,
                }),
            );
        }
    }

    Ok(())
}

fn execute_check_all(
    _cwd: &std::path::Path,
    options: calvin::application::CheckOptions,
    all_layers: bool,
    json: bool,
    verbose: u8,
) -> Result<calvin::application::CheckResult> {
    use calvin::domain::ports::RegistryError;
    use calvin::presentation::factory::create_registry_use_case;

    let registry = create_registry_use_case();
    let projects = registry.list_projects().map_err(|e| match e {
        RegistryError::Corrupted { path, .. } => {
            anyhow::Error::new(calvin::CalvinError::RegistryCorrupted { path })
        }
        _ => anyhow::Error::new(e),
    })?;

    if projects.is_empty() {
        if json {
            let mut out = std::io::stdout().lock();
            let _ = crate::ui::json::write_event(
                &mut out,
                &serde_json::json!({
                    "event": "complete",
                    "command": "check",
                    "success": true,
                    "projects": 0,
                    "message": "No projects in registry"
                }),
            );
        } else {
            println!("No projects in registry.");
        }
        return Ok(calvin::application::CheckResult::default());
    }

    let mut aggregate = calvin::application::CheckResult::default();
    let original = std::env::current_dir()?;

    if json {
        let mut out = std::io::stdout().lock();
        let _ = crate::ui::json::write_event(
            &mut out,
            &serde_json::json!({
                "event": "start",
                "command": "check",
                "all": true,
                "projects": projects.len(),
                "verbose": verbose,
            }),
        );

        for project in &projects {
            let _ = crate::ui::json::write_event(
                &mut out,
                &serde_json::json!({
                    "event": "project_start",
                    "command": "check",
                    "project": project.path.display().to_string(),
                }),
            );

            let config = calvin::config::Config::load_or_default(Some(&project.path));
            let use_case = calvin::application::CheckUseCase::new(config);

            if let Err(e) = std::env::set_current_dir(&project.path) {
                aggregate.errors += 1;
                let _ = crate::ui::json::write_event(
                    &mut out,
                    &serde_json::json!({
                        "event": "project_error",
                        "command": "check",
                        "project": project.path.display().to_string(),
                        "error": e.to_string(),
                    }),
                );
                continue;
            }

            let mut result =
                use_case.execute_with_callback(&project.path, options.clone(), |item| {
                    let status = match item.status {
                        calvin::application::CheckStatus::Pass => "pass",
                        calvin::application::CheckStatus::Warning => "warning",
                        calvin::application::CheckStatus::Error => "error",
                    };
                    let _ = crate::ui::json::write_event(
                        &mut out,
                        &serde_json::json!({
                            "event": "check",
                            "command": "check",
                            "project": project.path.display().to_string(),
                            "platform": item.platform,
                            "name": item.name,
                            "status": status,
                            "message": item.message,
                            "recommendation": item.recommendation,
                            "details": item.details,
                        }),
                    );
                })?;

            if all_layers {
                append_layer_checks_json(&project.path, &mut result, &mut out)?;
                // tag the layer check event with project as well via explicit event
                let _ = crate::ui::json::write_event(
                    &mut out,
                    &serde_json::json!({
                        "event": "check",
                        "command": "check",
                        "project": project.path.display().to_string(),
                        "platform": "layers",
                        "name": "layer_stack",
                        "status": "pass",
                        "message": "Layer stack checked",
                    }),
                );
            }

            aggregate.passed += result.passed;
            aggregate.warnings += result.warnings;
            aggregate.errors += result.errors;

            let _ = crate::ui::json::write_event(
                &mut out,
                &serde_json::json!({
                    "event": "project_complete",
                    "command": "check",
                    "project": project.path.display().to_string(),
                    "passes": result.passed,
                    "warnings": result.warnings,
                    "errors": result.errors,
                }),
            );
        }

        let _ = std::env::set_current_dir(original);

        let has_issues = if options.strict_warnings {
            aggregate.errors > 0 || aggregate.warnings > 0
        } else {
            aggregate.errors > 0
        };

        let _ = crate::ui::json::write_event(
            &mut out,
            &serde_json::json!({
                "event": "complete",
                "command": "check",
                "all": true,
                "projects": projects.len(),
                "passes": aggregate.passed,
                "warnings": aggregate.warnings,
                "errors": aggregate.errors,
                "success": !has_issues
            }),
        );

        if has_issues {
            std::process::exit(1);
        }

        return Ok(aggregate);
    }

    // Text mode: concise per-project output
    println!("Checking {} projects...", projects.len());
    for project in &projects {
        println!();
        println!("Project: {}", project.path.display());
        let config = calvin::config::Config::load_or_default(Some(&project.path));
        let use_case = calvin::application::CheckUseCase::new(config);

        if let Err(e) = std::env::set_current_dir(&project.path) {
            aggregate.errors += 1;
            eprintln!("  Error: {}", e);
            continue;
        }

        let mut result = use_case.execute(&project.path, options.clone())?;
        if all_layers {
            append_layer_checks_text(&project.path, &mut result)?;
        }

        aggregate.passed += result.passed;
        aggregate.warnings += result.warnings;
        aggregate.errors += result.errors;

        println!("  Passed: {}", result.passed);
        println!("  Warnings: {}", result.warnings);
        println!("  Errors: {}", result.errors);
    }

    let _ = std::env::set_current_dir(original);

    Ok(aggregate)
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
