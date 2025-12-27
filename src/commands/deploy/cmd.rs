//! Deploy command entry points using DeployUseCase

use std::path::Path;

use anyhow::Result;
use calvin::domain::policies::ScopePolicy;
use calvin::Target;

use super::options::DeployOptions;
use super::targets::DeployTarget;
use crate::commands::project_root::discover_project_root;
use crate::ui::context::UiContext;
use crate::ui::primitives::icon::Icon;
use crate::ui::primitives::text::display_with_tilde;
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

    let invocation_dir = std::env::current_dir()?;
    let project_root = discover_project_root(&invocation_dir);

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
        invocation_dir.join(source)
    } else {
        source.to_path_buf()
    };

    // Interactive layer selection when multiple layers exist
    let (use_user_layer, use_project_layer, additional_layers, use_additional_layers) =
        if interactive && !is_remote_target {
            // Query available layers using Application layer UseCase
            // This respects the user-specified source path via -s flag
            use calvin::application::layers::{LayerQueryOptions, LayerQueryUseCase};

            let query_options = LayerQueryOptions {
                project_layer_path: Some(project_layer_path.clone()),
                use_user_layer,
                use_additional_layers,
                additional_layers: additional_layers.clone(),
                disable_project_layer: !use_project_layer,
            };

            let use_case = LayerQueryUseCase::default();
            match use_case.query_with_options(&project_root, &config, &query_options) {
                Ok(layers_result) if layers_result.layers.len() > 1 => {
                    // Multiple layers exist, prompt user to select
                    match crate::ui::menu::select_layers_interactive(&layers_result, json) {
                        Some(selection) => {
                            let filtered_additional = if selection.use_additional_layers {
                                additional_layers.clone()
                            } else {
                                Vec::new()
                            };
                            (
                                selection.use_user_layer && use_user_layer,
                                selection.use_project_layer && use_project_layer,
                                filtered_additional,
                                selection.use_additional_layers,
                            )
                        }
                        None => anyhow::bail!("Layer selection aborted"),
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

    // Verbose: show resolved layer stack with asset provenance (PRD §10.4, §12.2)
    if !json && verbose > 0 {
        use calvin::domain::services::{merge_layers, LayerResolver};
        use calvin::infrastructure::FsLayerLoader;

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
                // Load assets for each layer to get counts and provenance
                use calvin::domain::ports::LayerLoader;
                let loader = FsLayerLoader::default();
                let mut layers_with_assets = Vec::new();
                for layer in &resolution.layers {
                    let mut layer_with_assets = layer.clone();
                    // Try to load assets; if it fails, use empty assets
                    let _ = loader.load_layer_assets(&mut layer_with_assets);
                    layers_with_assets.push(layer_with_assets);
                }

                // Print layer stack with asset counts (same format as `calvin layers`)
                let total_layers = layers_with_assets.len();
                println!("Layer Stack (highest priority first):");
                for (idx, layer) in layers_with_assets.iter().rev().enumerate() {
                    let layer_num = total_layers - idx;
                    let skill_count = layer
                        .assets
                        .iter()
                        .filter(|a| a.kind() == calvin::domain::entities::AssetKind::Skill)
                        .count();
                    let asset_count = layer.assets.len().saturating_sub(skill_count);
                    println!(
                        "  {}. [{}] {} ({} assets, {} skills)",
                        layer_num,
                        layer.name,
                        display_with_tilde(layer.path.original()),
                        asset_count,
                        skill_count
                    );
                }

                // Merge layers to get provenance and overrides
                let merge_result = merge_layers(&layers_with_assets);

                // Print asset provenance (PRD §10.4)
                // At -v level, show summary. At -vv level, show full list.
                if !merge_result.assets.is_empty() {
                    let override_count = merge_result.overrides.len();
                    let total_assets = merge_result.assets.len();

                    if verbose >= 2 {
                        // Full provenance list at -vv
                        println!("\nAsset Provenance ({} assets):", total_assets);
                        let mut sorted_assets: Vec<_> = merge_result.assets.iter().collect();
                        sorted_assets.sort_by(|a, b| a.0.cmp(b.0));
                        for (id, merged) in sorted_assets {
                            let override_note = if merged.overrides.is_some() {
                                " (override)"
                            } else {
                                ""
                            };
                            println!(
                                "  • {:<20} ← {}:{}{}",
                                id,
                                merged.source_layer,
                                display_with_tilde(&merged.source_file),
                                override_note
                            );
                        }
                    } else {
                        // Summary at -v
                        println!(
                            "\nAssets: {} merged ({} overridden)",
                            total_assets, override_count
                        );
                    }
                }

                // Print skills structure (PRD: -v shows skills tree)
                if verbose >= 1 {
                    let mut skills: Vec<_> = merge_result
                        .assets
                        .values()
                        .filter(|m| m.asset.kind() == calvin::domain::entities::AssetKind::Skill)
                        .collect();
                    skills.sort_by(|a, b| a.asset.id().cmp(b.asset.id()));

                    if !skills.is_empty() {
                        println!("\nSkills:");
                        for merged in skills {
                            println!("  • {} ({} layer)", merged.asset.id(), merged.source_layer);

                            let mut files: Vec<String> = Vec::new();
                            files.push("SKILL.md".to_string());
                            for rel in merged.asset.supplementals().keys() {
                                files.push(rel.display().to_string());
                            }
                            files.sort();

                            for file in files {
                                println!("    - {}", file);
                            }
                        }
                    }
                }

                // Print override information (PRD §12.2) - always show if present
                if !merge_result.overrides.is_empty() {
                    println!("\nOverrides:");
                    for ov in &merge_result.overrides {
                        println!(
                            "  • {:<20} {} overrides {}",
                            ov.asset_id, ov.by_layer, ov.from_layer
                        );
                    }
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
                &project_layer_path,
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
                &project_layer_path,
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
            &project_layer_path,
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
            &project_layer_path,
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

    // Surface non-fatal warnings (never fail silently).
    if !json && !result.warnings.is_empty() {
        for warning in &result.warnings {
            eprintln!("Warning: {}", warning);
        }
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
        let config_path = project_layer_path.join("config.toml");
        let target_config = if use_home {
            DeployTargetValue::Home
        } else {
            DeployTargetValue::Project
        };
        // Silently save - don't fail deploy if config save fails
        let _ = calvin::config::Config::save_deploy_target(&config_path, target_config);
    }

    if !result.is_success() {
        let mut message = format!("Deploy failed with {} error(s):", result.errors.len());
        for err in &result.errors {
            message.push_str("\n- ");
            message.push_str(err);
        }
        anyhow::bail!(message);
    }

    Ok(())
}
