//! Deploy command entry points using new DeployRunner

use std::path::Path;

use anyhow::Result;
use calvin::Target;

use super::targets::{DeployTarget, ScopePolicy};
use super::options::DeployOptions;
use super::runner::DeployRunner;
use crate::cli::ColorWhen;
use crate::ui::context::UiContext;
use crate::ui::primitives::icon::Icon;
use crate::ui::views::deploy::{render_deploy_header, render_deploy_summary};

/// Deploy command entry point (new implementation)
#[allow(clippy::too_many_arguments)]
pub fn cmd_deploy_v2(
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
    let result = runner.run()?;

    // Render summary
    if json {
        let mut out = std::io::stdout().lock();
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

    Ok(())
}
