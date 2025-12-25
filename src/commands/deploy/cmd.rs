//! Deploy command entry points using DeployUseCase

use std::path::Path;

use anyhow::Result;
use calvin::domain::policies::ScopePolicy;
use calvin::Target;

use super::options::DeployOptions;
use super::targets::DeployTarget;
use crate::ui::context::UiContext;
use crate::ui::primitives::icon::Icon;
use crate::ui::views::deploy::{render_deploy_header, render_deploy_summary};
use calvin::presentation::ColorWhen;

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
    layers: &[std::path::PathBuf],
    no_user_layer: bool,
    no_additional_layers: bool,
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
        layers,
        no_user_layer,
        no_additional_layers,
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
    layers: &[std::path::PathBuf],
    no_user_layer: bool,
    no_additional_layers: bool,
    force: bool,
    interactive: bool,
    dry_run: bool,
    cleanup: bool,
    json: bool,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    use calvin::domain::value_objects::DeployTarget as DeployTargetValue;

    // Validate args
    if home && remote.is_some() {
        anyhow::bail!("--home and --remote cannot be used together");
    }

    let project_root = std::env::current_dir()?;

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
            DeployTargetValue::Home => true,
            DeployTargetValue::Project | DeployTargetValue::Unset => false,
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

    let is_remote_target = matches!(&target_for_bridge, DeployTarget::Remote(_));

    let use_project_layer = is_remote_target || !config.sources.disable_project_layer;

    let use_user_layer = !is_remote_target
        && !no_user_layer
        && config.sources.use_user_layer
        && !config.sources.ignore_user_layer;

    let mut additional_layers: Vec<std::path::PathBuf> = if config.sources.ignore_additional_layers
    {
        Vec::new()
    } else {
        config.sources.additional_layers.clone()
    };
    if !no_additional_layers {
        additional_layers.extend(layers.iter().cloned());
    } else {
        additional_layers.clear();
    }
    let use_additional_layers = !is_remote_target && !no_additional_layers;

    let project_layer_path = if source.is_relative() {
        project_root.join(source)
    } else {
        source.to_path_buf()
    };

    // Interactive layer selection when multiple layers exist
    let (use_user_layer, use_project_layer, additional_layers, use_additional_layers) =
        if interactive && !is_remote_target {
            // Query available layers for interactive selection using LayerResolver
            // This respects the user-specified source path via -s flag
            use calvin::domain::services::LayerResolver;

            let mut resolver = LayerResolver::new(project_root.clone())
                .with_project_layer_path(project_layer_path.clone())
                .with_disable_project_layer(!use_project_layer)
                .with_additional_layers(if use_additional_layers {
                    additional_layers.clone()
                } else {
                    Vec::new()
                })
                .with_remote_mode(is_remote_target);

            if use_user_layer {
                let effective_user_path = config
                    .sources
                    .user_layer_path
                    .clone()
                    .unwrap_or_else(calvin::config::default_user_layer_path);
                resolver = resolver.with_user_layer_path(effective_user_path);
            }

            match resolver.resolve() {
                Ok(mut resolution) if resolution.layers.len() > 1 => {
                    // Load asset counts for each layer
                    use calvin::domain::ports::LayerLoader;
                    let loader = calvin::infrastructure::layer::FsLayerLoader::default();
                    for layer in &mut resolution.layers {
                        let _ = loader.load_layer_assets(layer);
                    }

                    // Convert to LayerQueryResult for the interactive menu
                    let layers_result = calvin::application::layers::LayerQueryResult {
                        layers: resolution
                            .layers
                            .iter()
                            .map(|l| calvin::application::layers::LayerSummary {
                                name: l.name.clone(),
                                layer_type: match l.layer_type {
                                    calvin::domain::entities::LayerType::User => "user".to_string(),
                                    calvin::domain::entities::LayerType::Custom => {
                                        "custom".to_string()
                                    }
                                    calvin::domain::entities::LayerType::Project => {
                                        "project".to_string()
                                    }
                                },
                                original_path: l.path.original().clone(),
                                resolved_path: l.path.resolved().clone(),
                                asset_count: l.assets.len(),
                            })
                            .collect(),
                        merged_asset_count: 0,
                        overridden_asset_count: 0,
                    };

                    // Multiple layers exist, prompt user to select
                    match crate::ui::menu::select_layers_interactive(&layers_result, json) {
                        Some(selection) => {
                            // Filter additional layers based on selection
                            // If user selected any custom layers, keep all additional layers
                            let has_custom_selected = selection
                                .selected_layers
                                .iter()
                                .any(|name| name.starts_with("custom"));
                            let filtered_additional: Vec<std::path::PathBuf> =
                                if has_custom_selected {
                                    additional_layers.clone()
                                } else {
                                    Vec::new()
                                };
                            let use_additional = has_custom_selected;
                            (
                                selection.use_user_layer && use_user_layer,
                                selection.use_project_layer && use_project_layer,
                                filtered_additional,
                                use_additional,
                            )
                        }
                        None => {
                            // User aborted
                            anyhow::bail!("Layer selection aborted");
                        }
                    }
                }
                _ => {
                    // Single layer or error - use defaults
                    (
                        use_user_layer,
                        use_project_layer,
                        additional_layers,
                        use_additional_layers,
                    )
                }
            }
        } else {
            (
                use_user_layer,
                use_project_layer,
                additional_layers,
                use_additional_layers,
            )
        };

    // Update user_layer_path based on layer selection
    let user_layer_path = if use_user_layer {
        config
            .sources
            .user_layer_path
            .clone()
            .or_else(|| Some(calvin::config::default_user_layer_path()))
    } else {
        None
    };

    // Verbose: show resolved layer stack (Phase 1 multi-layer visibility)
    if !json && verbose > 0 {
        use calvin::domain::services::LayerResolver;
        let mut resolver = LayerResolver::new(project_root.clone())
            .with_project_layer_path(project_layer_path.clone())
            .with_disable_project_layer(!use_project_layer)
            .with_additional_layers(if use_additional_layers {
                additional_layers.clone()
            } else {
                Vec::new()
            })
            .with_remote_mode(is_remote_target);

        if use_user_layer {
            if let Some(user_layer_path) = user_layer_path
                .clone()
                .or_else(|| Some(calvin::config::default_user_layer_path()))
            {
                resolver = resolver.with_user_layer_path(user_layer_path);
            }
        }

        match resolver.resolve() {
            Ok(resolution) => {
                println!("Layers:");
                for layer in resolution.layers {
                    println!("  - [{}] {}", layer.name, layer.path.original().display());
                }
                for warning in resolution.warnings {
                    eprintln!("Warning: {}", warning);
                }
            }
            Err(e) => {
                eprintln!("Warning: failed to resolve layers: {}", e);
            }
        }
    }

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

    // Determine effective targets: CLI > config > default
    // This is resolved once and used for both adapters and options
    let effective_targets = super::layer_config::resolve_effective_targets(
        &config,
        &options_for_bridge.targets,
        interactive,
        json,
        calvin::config::PromptpackLayerInputs {
            project_root: project_root.clone(),
            project_layer_path: project_layer_path.clone(),
            disable_project_layer: config.sources.disable_project_layer,
            user_layer_path: user_layer_path.clone(),
            use_user_layer,
            additional_layers: additional_layers.clone(),
            use_additional_layers,
            remote_mode: is_remote_target,
        },
    )?;

    // Run deploy
    let result = if is_remote_target {
        // Remote: use new engine with SyncDestination abstraction
        if let DeployTarget::Remote(remote_spec) = &target_for_bridge {
            let use_case_options = super::bridge::convert_options(
                &project_root,
                source,
                &target_for_bridge,
                &options_for_bridge,
                cleanup,
                &effective_targets,
                super::bridge::LayerInputs {
                    use_project_layer: true,
                    user_layer_path: None,
                    use_user_layer: false,
                    additional_layers: Vec::new(),
                    use_additional_layers: false,
                },
            );
            super::bridge::run_remote_deployment(
                remote_spec,
                source,
                &use_case_options,
                &effective_targets,
            )
        } else {
            unreachable!("is_remote_target check failed")
        }
    } else if json {
        // JSON mode: use new engine with JsonEventSink
        use calvin::infrastructure::JsonEventSink;
        use std::sync::Arc;

        let use_case_options = super::bridge::convert_options(
            &project_root,
            source,
            &target_for_bridge,
            &options_for_bridge,
            cleanup,
            &effective_targets,
            super::bridge::LayerInputs {
                use_project_layer,
                user_layer_path: user_layer_path.clone(),
                use_user_layer,
                additional_layers: additional_layers.clone(),
                use_additional_layers,
            },
        );
        let use_case = super::bridge::create_use_case_for_targets(&effective_targets);
        let json_sink = Arc::new(JsonEventSink::stdout());
        use_case.execute_with_events(&use_case_options, json_sink)
    } else {
        // Non-JSON local mode: use new DeployUseCase architecture
        let use_case_options = super::bridge::convert_options(
            &project_root,
            source,
            &target_for_bridge,
            &options_for_bridge,
            cleanup,
            &effective_targets,
            super::bridge::LayerInputs {
                use_project_layer,
                user_layer_path: user_layer_path.clone(),
                use_user_layer,
                additional_layers: additional_layers.clone(),
                use_additional_layers,
            },
        );
        let use_case = super::bridge::create_use_case_for_targets(&effective_targets);
        use_case.execute(&use_case_options)
    };

    // Render summary
    if json {
        // Already emitted in the JSON block above
    } else {
        // Use actual asset count from result
        let asset_count = result.asset_count;
        let target_count = effective_targets.len().max(1); // Use actual target count
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

    // Save deploy target to config for watch command.
    //
    // Semantics:
    // - Only persist when the user did NOT explicitly override the destination via CLI flags.
    // - Only persist when the effective deploy target is currently Unset (avoid rewriting user intent).
    // - Never persist for dry-run or non-local targets.
    if !dry_run
        && result.is_success()
        && target_for_bridge.is_local()
        && !home
        && !explicit_project
        && config.deploy.target == DeployTargetValue::Unset
    {
        let config_path = source.join("config.toml");
        let target_config = if use_home {
            DeployTargetValue::Home
        } else {
            DeployTargetValue::Project
        };
        // Silently save - don't fail deploy if config save fails
        let _ = calvin::config::Config::save_deploy_target(&config_path, target_config);
    }

    Ok(())
}
