//! Deploy command entry points using new DeployRunner

use std::io::Write;
use std::path::Path;

use anyhow::Result;
use calvin::fs::LocalFileSystem;
use calvin::sync::lockfile::Lockfile;
use calvin::sync::orphan::delete_orphans;
use calvin::sync::ScopePolicy;
use calvin::Target;

use super::options::DeployOptions;
use super::runner::DeployRunner;
use super::targets::DeployTarget;
use crate::cli::ColorWhen;
use crate::ui::context::UiContext;
use crate::ui::primitives::icon::Icon;
use crate::ui::views::deploy::{render_deploy_header, render_deploy_summary};
use crate::ui::views::orphan::{render_orphan_list, render_orphan_summary};

/// Deploy command entry point
///
/// # Target Selection Priority
/// 1. `remote` - if specified, deploy to remote
/// 2. `home` - if true, deploy to home directory  
/// 3. `explicit_project` - if true, deploy to project (overrides config)
/// 4. config.deploy.target - fall back to configuration
/// 5. default - project
#[allow(clippy::too_many_arguments)]
pub fn cmd_deploy(
    source: &Path,
    home: bool,
    remote: Option<String>,
    targets: &Option<Vec<Target>>,
    force: bool,
    interactive: bool,
    dry_run: bool,
    cleanup: bool,
    json: bool,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    cmd_deploy_with_explicit_target(
        source,
        home,
        false, // explicit_project: use config default
        remote,
        targets,
        force,
        interactive,
        dry_run,
        cleanup,
        json,
        verbose,
        color,
        no_animation,
    )
}

/// Deploy command with explicit target override
///
/// When `explicit_project` is true, deploy to project regardless of config.
#[allow(clippy::too_many_arguments)]
pub fn cmd_deploy_with_explicit_target(
    source: &Path,
    home: bool,
    explicit_project: bool,
    remote: Option<String>,
    targets: &Option<Vec<Target>>,
    force: bool,
    interactive: bool,
    dry_run: bool,
    cleanup: bool,
    json: bool,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    use calvin::config::DeployTargetConfig;

    // Validate args
    if home && remote.is_some() {
        anyhow::bail!("--home and --remote cannot be used together");
    }

    // Determine project root
    let project_root = source.parent().unwrap_or(source).to_path_buf();

    // Load configuration early to determine effective target
    let config = calvin::config::Config::load_or_default(Some(&project_root));

    // Determine effective home setting:
    // Priority: remote > home flag > explicit_project > config > default (project)
    let use_home = if home {
        true
    } else if remote.is_some() || explicit_project {
        false // Remote or explicit project takes precedence
    } else {
        match config.deploy.target {
            DeployTargetConfig::Home => true,
            DeployTargetConfig::Project | DeployTargetConfig::Unset => false,
        }
    };

    // Determine target
    let target = if use_home {
        DeployTarget::Home
    } else if let Some(remote) = remote {
        DeployTarget::Remote(remote)
    } else {
        DeployTarget::Project(project_root.clone())
    };

    // Determine scope policy based on effective target
    let scope_policy = if use_home {
        ScopePolicy::ForceUser
    } else {
        ScopePolicy::Keep
    };

    // Build options
    let mut options = DeployOptions::new();
    options.force = force;
    options.interactive = interactive;
    options.dry_run = dry_run;
    options.json = json;
    options.verbose = verbose;
    options.no_animation = no_animation;
    if let Some(ts) = targets {
        options.targets = ts.clone();
    }

    // Create UI context
    let ui = UiContext::new(json, verbose, color, no_animation, &config);

    // Clone for new engine use
    let target_for_bridge = target.clone();
    let options_for_bridge = options.clone();

    // LEGACY: Create runner for fallback path (only used when CALVIN_LEGACY_ENGINE=1)
    // TODO: Remove in v0.4.0 after confirming new engine stability
    #[allow(deprecated)]
    let runner = DeployRunner::new(source.to_path_buf(), target, scope_policy, options, ui);

    // Render header
    if !json {
        let action = if dry_run {
            "Deploy (dry run)"
        } else {
            "Deploy"
        };
        let modes = if force {
            vec!["Force".to_string()]
        } else if interactive {
            vec!["Interactive".to_string()]
        } else {
            vec!["Auto".to_string()] // --yes mode: skip conflicts silently
        };
        let (target_display, remote_display) = match &target_for_bridge {
            DeployTarget::Remote(r) => (None, Some(r.as_str())),
            _ => (
                target_for_bridge
                    .destination_display()
                    .as_deref()
                    .map(|s| s.to_string()),
                None,
            ),
        };
        print!(
            "{}",
            render_deploy_header(
                action,
                source,
                target_display.as_deref(),
                remote_display,
                &modes,
                ui.color,
                ui.unicode,
            )
        );
    }

    // Check for existing issues first
    if !json {
        println!(
            "{} Scanning {}...",
            Icon::Progress.colored(ui.color, ui.unicode),
            source.display()
        );
    }

    // Run deploy
    let result = if json {
        // JSON mode: use new engine with JsonEventSink
        use calvin::infrastructure::JsonEventSink;
        use std::sync::Arc;

        let use_case_options = super::bridge::convert_options(
            source,
            &target_for_bridge,
            &options_for_bridge,
            cleanup,
        );
        let effective_targets = if options_for_bridge.targets.is_empty() {
            config.enabled_targets()
        } else {
            options_for_bridge.targets.clone()
        };
        let use_case = super::bridge::create_use_case_for_targets(&effective_targets);
        let json_sink = Arc::new(JsonEventSink::stdout());
        let use_case_result = use_case.execute_with_events(&use_case_options, json_sink);
        super::bridge::convert_result(&use_case_result)
    } else {
        // Non-JSON mode: run without callback
        // Check if new engine is enabled
        if super::bridge::should_use_new_engine() {
            // Use new DeployUseCase architecture
            // Pass cleanup flag to let use case handle orphan cleanup
            let use_case_options = super::bridge::convert_options(
                source,
                &target_for_bridge,
                &options_for_bridge,
                cleanup,
            );
            // Determine effective targets: CLI > config > all
            let effective_targets = if options_for_bridge.targets.is_empty() {
                config.enabled_targets()
            } else {
                options_for_bridge.targets.clone()
            };
            let use_case = super::bridge::create_use_case_for_targets(&effective_targets);
            let use_case_result = use_case.execute(&use_case_options);
            super::bridge::convert_result(&use_case_result)
        } else {
            // Use legacy DeployRunner
            runner.run()?
        }
    };

    // Track if we used the new engine (for skipping legacy orphan detection)
    // JSON mode always uses new engine now, non-JSON checks the flag
    let used_new_engine = json || super::bridge::should_use_new_engine();

    // Render summary
    if json {
        // Already emitted in the JSON block above
    } else {
        // Estimate assets from written files (rough)
        let asset_count = result.written.len() / 4; // ~4 files per asset
        let target_count = 5; // All targets
        print!(
            "{}",
            render_deploy_summary(
                if dry_run {
                    "Deploy (dry run)"
                } else {
                    "Deploy"
                },
                asset_count,
                target_count,
                &result,
                ui.color,
                ui.unicode,
            )
        );
    }

    // Orphan detection and cleanup (for local, successful deploys)
    // Skip for remote targets - deletion over SSH is complex
    // Note: dry_run mode will show what would be deleted but won't delete
    // Skip for new engine - it handles orphan cleanup internally
    if result.is_success() && target_for_bridge.is_local() && !used_new_engine {
        let fs = LocalFileSystem;
        let lockfile_path = runner.get_lockfile_path();
        let lockfile = Lockfile::load_or_new(&lockfile_path, &fs);

        let orphan_result = runner.detect_orphans();

        // Filter to only existing orphans
        let existing_orphans: Vec<_> = orphan_result
            .orphans
            .iter()
            .filter(|o| o.exists)
            .cloned()
            .collect();

        if !existing_orphans.is_empty() {
            let safe_count = existing_orphans
                .iter()
                .filter(|o| o.is_safe_to_delete())
                .count();

            if json {
                // Emit orphan detection event
                let mut out = std::io::stdout().lock();
                let _ = crate::ui::json::write_event(
                    &mut out,
                    &serde_json::json!({
                        "event": "orphans_detected",
                        "command": "deploy",
                        "count": existing_orphans.len(),
                        "safe_count": safe_count,
                    }),
                );
                let _ = out.flush();
            }

            // Decide what to do based on mode
            let should_delete = if cleanup {
                // --cleanup flag: auto-delete safe files
                true
            } else if interactive && safe_count > 0 {
                // Interactive mode: ask user
                if !json {
                    eprintln!();
                    eprint!(
                        "{}",
                        render_orphan_list(&existing_orphans, ui.color, ui.unicode)
                    );

                    // Ask for confirmation using dialoguer
                    use dialoguer::Confirm;
                    Confirm::new()
                        .with_prompt(format!(
                            "Delete {} file(s) with Calvin signature?",
                            safe_count
                        ))
                        .default(false)
                        .interact()
                        .unwrap_or(false)
                } else {
                    false
                }
            } else {
                // Non-interactive, non-cleanup: just warn
                if !json {
                    eprintln!(
                        "\n{} {} orphan file(s) detected. Run with --cleanup to remove.",
                        Icon::Warning.colored(ui.color, ui.unicode),
                        existing_orphans.len()
                    );
                }
                false
            };

            if should_delete {
                // In dry_run mode, show what would be deleted but don't actually delete
                if dry_run {
                    let would_delete = existing_orphans
                        .iter()
                        .filter(|o| force || o.is_safe_to_delete())
                        .count();
                    let would_skip = existing_orphans.len() - would_delete;

                    if json {
                        let mut out = std::io::stdout().lock();
                        let _ = crate::ui::json::write_event(
                            &mut out,
                            &serde_json::json!({
                                "event": "orphans_would_delete",
                                "command": "deploy",
                                "dry_run": true,
                                "would_delete": would_delete,
                                "would_skip": would_skip,
                            }),
                        );
                        let _ = out.flush();
                    } else {
                        eprintln!(
                            "\n{} Would delete {} orphan file(s) (dry run).",
                            Icon::Trash.colored(ui.color, ui.unicode),
                            would_delete
                        );
                        if would_skip > 0 {
                            eprintln!(
                                "{} Would skip {} file(s) (no Calvin signature).",
                                Icon::Warning.colored(ui.color, ui.unicode),
                                would_skip
                            );
                        }
                    }
                } else {
                    // Actually delete
                    let delete_result = delete_orphans(&existing_orphans, force, &fs);

                    match delete_result {
                        Ok(del_res) => {
                            if json {
                                let mut out = std::io::stdout().lock();
                                let _ = crate::ui::json::write_event(
                                    &mut out,
                                    &serde_json::json!({
                                        "event": "orphans_deleted",
                                        "command": "deploy",
                                        "deleted": del_res.deleted.len(),
                                        "skipped": del_res.skipped.len(),
                                    }),
                                );
                                let _ = out.flush();
                            } else if !del_res.deleted.is_empty() || !del_res.skipped.is_empty() {
                                eprintln!(
                                    "{}",
                                    render_orphan_summary(
                                        del_res.deleted.len(),
                                        del_res.skipped.len(),
                                        ui.color,
                                        ui.unicode
                                    )
                                );
                            }

                            // Update lockfile to remove deleted entries
                            if !del_res.deleted.is_empty() {
                                let mut updated_lockfile = lockfile.clone();
                                for key in &del_res.deleted {
                                    updated_lockfile.remove(key);
                                }
                                let _ = updated_lockfile.save(&lockfile_path, &fs);
                            }
                        }
                        Err(e) => {
                            if !json {
                                eprintln!(
                                    "{} Failed to delete orphan files: {}",
                                    Icon::Error.colored(ui.color, ui.unicode),
                                    e
                                );
                            }
                        }
                    }
                } // end of else block (not dry_run)
            }
        }
    }

    // Save deploy target to config for watch command (only on success, not dry-run, local only)
    if !dry_run && result.is_success() && target_for_bridge.is_local() {
        let config_path = source.join("config.toml");
        let target_config = if use_home {
            calvin::config::DeployTargetConfig::Home
        } else {
            calvin::config::DeployTargetConfig::Project
        };
        // Silently save - don't fail deploy if config save fails
        let _ = calvin::config::Config::save_deploy_target(&config_path, target_config);
    }

    Ok(())
}

/// Legacy sync command (deprecated, calls cmd_deploy)
#[allow(clippy::too_many_arguments)]
pub fn cmd_sync(
    source: &Path,
    remote: Option<String>,
    targets: &Option<Vec<Target>>,
    force: bool,
    interactive: bool,
    dry_run: bool,
    json: bool,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    cmd_deploy(
        source,
        false, // home
        remote,
        targets,
        force,
        interactive,
        dry_run,
        false, // cleanup - legacy command doesn't support it
        json,
        verbose,
        color,
        no_animation,
    )
}

/// Legacy install command (deprecated, calls cmd_deploy with --home)
#[allow(clippy::too_many_arguments)]
pub fn cmd_install(
    source: &Path,
    user: bool,
    global: bool,
    cli_targets: &Option<Vec<Target>>,
    force: bool,
    interactive: bool,
    dry_run: bool,
    json: bool,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    if !user && !global {
        anyhow::bail!(
            "Install requires --user or --global flag. Use --global to install all assets to home directory, or --user for only scope: user assets."
        );
    }

    // Both user and global install to home; the difference was scope filtering
    // In the new implementation, --home always uses ForceUser scope policy
    cmd_deploy(
        source,
        true, // home
        None, // remote
        cli_targets,
        force,
        interactive,
        dry_run,
        false, // cleanup - legacy command doesn't support it
        json,
        verbose,
        color,
        no_animation,
    )
}
