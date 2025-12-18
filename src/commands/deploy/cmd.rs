//! Deploy command entry points using new DeployRunner

use std::io::Write;
use std::path::Path;

use anyhow::Result;
use calvin::Target;
use calvin::sync::SyncEvent;

use super::targets::{DeployTarget, ScopePolicy};
use super::options::DeployOptions;
use super::runner::DeployRunner;
use crate::cli::ColorWhen;
use crate::ui::context::UiContext;
use crate::ui::primitives::icon::Icon;
use crate::ui::views::deploy::{render_deploy_header, render_deploy_summary};

/// Deploy command entry point
#[allow(clippy::too_many_arguments)]
pub fn cmd_deploy(
    source: &Path,
    home: bool,
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
    // Validate args
    if home && remote.is_some() {
        anyhow::bail!("--home and --remote cannot be used together");
    }

    // Determine target
    let project_root = source.parent().unwrap_or(source).to_path_buf();
    let target = if home {
        DeployTarget::Home
    } else if let Some(remote) = remote {
        DeployTarget::Remote(remote)
    } else {
        DeployTarget::Project(project_root.clone())
    };

    // Determine scope policy
    let scope_policy = if home {
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
    let config = calvin::config::Config::load_or_default(Some(source));
    let ui = UiContext::new(json, verbose, color, no_animation, &config);

    // Create runner
    let runner = DeployRunner::new(
        source.to_path_buf(),
        target,
        scope_policy,
        options,
        ui,
    );

    // Render header
    if !json {
        let action = if dry_run { "Deploy (dry run)" } else { "Deploy" };
        let modes = if interactive { 
            vec!["Interactive".to_string()] 
        } else { 
            vec!["Force".to_string()] 
        };
        let (target_display, remote_display) = match runner.target() {
            DeployTarget::Remote(r) => (None, Some(r.as_str())),
            _ => (runner.target().destination_display().as_deref().map(|s| s.to_string()), None),
        };
        print!("{}", render_deploy_header(
            action,
            runner.source(),
            target_display.as_deref(),
            remote_display,
            &modes,
            runner.ui().color,
            runner.ui().unicode,
        ));
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
        // JSON mode: emit NDJSON event stream
        let mut out = std::io::stdout().lock();
        
        // Emit start event
        let _ = crate::ui::json::write_event(
            &mut out,
            &serde_json::json!({
                "event": "start",
                "command": "deploy",
            }),
        );
        let _ = out.flush();
        
        // Run with callback to emit item events
        let result = runner.run_with_callback(Some(|event: SyncEvent| {
            let mut out = std::io::stdout().lock();
            match &event {
                SyncEvent::ItemStart { index, path } => {
                    let _ = crate::ui::json::write_event(
                        &mut out,
                        &serde_json::json!({
                            "event": "item_start",
                            "command": "deploy",
                            "index": index,
                            "path": path,
                        }),
                    );
                }
                SyncEvent::ItemWritten { index, path } => {
                    let _ = crate::ui::json::write_event(
                        &mut out,
                        &serde_json::json!({
                            "event": "item_written",
                            "command": "deploy",
                            "index": index,
                            "path": path,
                        }),
                    );
                }
                SyncEvent::ItemSkipped { index, path } => {
                    let _ = crate::ui::json::write_event(
                        &mut out,
                        &serde_json::json!({
                            "event": "item_skipped",
                            "command": "deploy",
                            "index": index,
                            "path": path,
                        }),
                    );
                }
                SyncEvent::ItemError { index, path, message } => {
                    let _ = crate::ui::json::write_event(
                        &mut out,
                        &serde_json::json!({
                            "event": "item_error",
                            "command": "deploy",
                            "index": index,
                            "path": path,
                            "error": message,
                        }),
                    );
                }
            }
            let _ = out.flush();
        }))?;
        
        // Emit complete event
        let _ = crate::ui::json::write_event(
            &mut out,
            &serde_json::json!({
                "event": "complete",
                "command": "deploy",
                "status": if result.is_success() { "success" } else { "partial" },
                "written": result.written.len(),
                "skipped": result.skipped.len(),
                "errors": result.errors.len(),
            }),
        );
        let _ = out.flush();
        
        result
    } else {
        // Non-JSON mode: run without callback
        runner.run()?
    };

    // Render summary
    if json {
        // Already emitted in the JSON block above
    } else {
        // Estimate assets from written files (rough)
        let asset_count = result.written.len() / 4; // ~4 files per asset
        let target_count = 5; // All targets
        print!("{}", render_deploy_summary(
            if dry_run { "Deploy (dry run)" } else { "Deploy" },
            asset_count,
            target_count,
            &result,
            ui.color,
            ui.unicode,
        ));
    }
    
    // Save deploy target to config for watch command (only on success, not dry-run, local only)
    if !dry_run && result.is_success() && runner.target().is_local() {
        let config_path = source.join("config.toml");
        let target_config = if home {
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
        false,            // home
        remote,
        targets,
        force,
        interactive,
        dry_run,
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
        true,             // home
        None,             // remote
        cli_targets,
        force,
        interactive,
        dry_run,
        json,
        verbose,
        color,
        no_animation,
    )
}
