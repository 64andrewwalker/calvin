use std::path::Path;

use anyhow::{bail, Result};

use calvin::presentation::ColorWhen;

pub fn cmd_watch(
    source: &Path,
    home: bool,
    watch_all_layers: bool,
    json: bool,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    use calvin::application::watch::{WatchEvent, WatchOptions, WatchUseCase};
    use calvin::domain::value_objects::{DeployTarget, Scope};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Determine project root
    let project_root = std::env::current_dir()?;

    let project_layer_path = if source.is_relative() {
        project_root.join(source)
    } else {
        source.to_path_buf()
    };

    // Load base config (user config + project config, if present).
    let base_config = calvin::config::Config::load_or_default(Some(&project_root));

    // Merge config across resolved promptpack layers (user/custom/project).
    let additional_layers: Vec<std::path::PathBuf> = if base_config.sources.ignore_additional_layers
    {
        Vec::new()
    } else {
        base_config.sources.additional_layers.clone()
    };
    let use_additional_layers = !base_config.sources.ignore_additional_layers;

    let use_user_layer =
        base_config.sources.use_user_layer && !base_config.sources.ignore_user_layer;

    let (config, warnings) = calvin::config::merge_promptpack_layer_configs(
        &base_config,
        calvin::config::PromptpackLayerInputs {
            project_root: project_root.clone(),
            project_layer_path,
            disable_project_layer: base_config.sources.disable_project_layer,
            user_layer_path: base_config.sources.user_layer_path.clone(),
            use_user_layer,
            additional_layers,
            use_additional_layers,
            remote_mode: false,
        },
    )?;
    for warning in warnings {
        eprintln!(
            "Warning: Unknown config key '{}' in {}{}",
            warning.key,
            warning.file.display(),
            warning
                .suggestion
                .as_ref()
                .map(|s| format!(". Did you mean '{}'?", s))
                .unwrap_or_default()
        );
    }

    let mut targets = config.enabled_targets();
    if targets.contains(&calvin::Target::All) {
        targets = calvin::Target::ALL_CONCRETE.to_vec();
    }

    let ui = crate::ui::context::UiContext::new(json, 0, color, no_animation, &config);

    // Determine deploy target: CLI flag > config
    // If neither, show helpful error
    let scope = if home {
        Scope::User
    } else {
        match config.deploy.target {
            DeployTarget::Home => Scope::User,
            DeployTarget::Project => Scope::Project,
            DeployTarget::Unset => {
                if json {
                    bail!("No deploy target configured. Add [deploy].target to config.toml or use --home flag.");
                }
                // Show helpful error message using Box widget
                use crate::ui::widgets::r#box::{Box as UiBox, BoxStyle};
                let mut b = UiBox::with_title("Configuration Required").style(BoxStyle::Warning);
                b.add_line("No deploy target configured.");
                b.add_empty();
                b.add_line("Options:");
                b.add_line("  1. Run 'calvin deploy' first to set up your target");
                b.add_line("  2. Add to .promptpack/config.toml:");
                b.add_line("       [deploy]");
                b.add_line("       target = \"home\"   # or \"project\"");
                b.add_line("  3. Use: calvin watch --home");
                eprintln!("\n{}", b.render(ui.color, ui.unicode));
                std::process::exit(1);
            }
        }
    };

    let target_label = if scope == Scope::User {
        "Home (~/)"
    } else {
        "Project"
    };

    let options = WatchOptions::new(source.to_path_buf(), project_root)
        .with_scope(scope)
        .with_json(json)
        .with_config(config)
        .with_targets(targets)
        .with_watch_all_layers(watch_all_layers);

    // Set up Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    ctrlc::set_handler(move || {
        running_clone.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl+C handler");

    if !json {
        let source_display = source.display().to_string();
        print!(
            "{}",
            crate::ui::views::watch::render_watch_header_with_target(
                &source_display,
                target_label,
                ui.color,
                ui.unicode
            )
        );
    }

    // Start watching using the new WatchUseCase
    let use_case = WatchUseCase::new(options);
    use_case.start(running, |event| {
        if json {
            println!("{}", event.to_json());
        } else {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| {
                    let secs = d.as_secs() % 86_400;
                    let h = secs / 3600;
                    let m = (secs % 3600) / 60;
                    let s = secs % 60;
                    format!("{:02}:{:02}:{:02}", h, m, s)
                })
                .unwrap_or_else(|_| "00:00:00".to_string());

            let rendered = crate::ui::views::watch::render_watch_event(
                &timestamp, &event, ui.color, ui.unicode,
            );

            match event {
                WatchEvent::Error { .. } => eprint!("{rendered}"),
                _ => print!("{rendered}"),
            }
        }
    })?;

    Ok(())
}
