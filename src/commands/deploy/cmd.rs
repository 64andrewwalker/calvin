//! Deploy command entry points using DeployUseCase

use std::path::Path;

use anyhow::Result;
use calvin::domain::policies::ScopePolicy;
use calvin::Target;

use super::options::DeployOptions;
use super::targets::DeployTarget;
use crate::cli::ColorWhen;
use crate::ui::context::UiContext;
use crate::ui::primitives::icon::Icon;
use crate::ui::views::deploy::{render_deploy_header, render_deploy_summary};

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
    let _scope_policy = if use_home {
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

    // Clone for use case creation
    let target_for_bridge = target.clone();
    let options_for_bridge = options.clone();

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
    let is_remote_target = matches!(&target_for_bridge, DeployTarget::Remote(_));

    let result = if is_remote_target {
        // Remote: use new engine with SyncDestination abstraction
        if let DeployTarget::Remote(remote_spec) = &target_for_bridge {
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
            let use_case_result = super::bridge::run_remote_deployment(
                remote_spec,
                source,
                &use_case_options,
                &effective_targets,
            );
            super::bridge::convert_result(&use_case_result)
        } else {
            unreachable!("is_remote_target check failed")
        }
    } else if json {
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
        // Non-JSON local mode: use new DeployUseCase architecture
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
    };

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

    // Note: Orphan detection is now handled by DeployUseCase internally
    // (when options.clean_orphans is set)

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
