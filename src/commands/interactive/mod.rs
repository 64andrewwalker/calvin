//! Interactive command module
//!
//! Provides an interactive CLI experience for first-time and existing users.

mod menu;
mod state;
#[cfg(test)]
mod tests;
mod wizard;

use std::path::Path;

use anyhow::Result;
use is_terminal::IsTerminal;

use calvin::presentation::ColorWhen;
use state::{detect_state, ProjectState};

/// Resolve layers and return JSON-serializable representation
fn resolve_layers_for_json(cwd: &Path, config: &calvin::config::Config) -> serde_json::Value {
    use calvin::domain::ports::LayerLoader;
    use calvin::domain::services::LayerResolver;
    use calvin::infrastructure::FsLayerLoader;

    let promptpack_dir = cwd.join(".promptpack");
    let use_user_layer = config.sources.use_user_layer && !config.sources.ignore_user_layer;
    let use_additional_layers = !config.sources.ignore_additional_layers;

    let mut resolver = LayerResolver::new(cwd.to_path_buf())
        .with_project_layer_path(promptpack_dir)
        .with_disable_project_layer(config.sources.disable_project_layer)
        .with_remote_mode(false)
        .with_additional_layers(if use_additional_layers {
            config.sources.additional_layers.clone()
        } else {
            Vec::new()
        });

    if use_user_layer {
        let user_layer_path = config
            .sources
            .user_layer_path
            .clone()
            .unwrap_or_else(calvin::config::default_user_layer_path);
        resolver = resolver.with_user_layer_path(user_layer_path);
    }

    let resolution = match resolver.resolve() {
        Ok(r) => r,
        Err(_) => return serde_json::json!([]),
    };

    // Load assets for each layer to get counts
    let loader = FsLayerLoader::default();
    let layers: Vec<serde_json::Value> = resolution
        .layers
        .iter()
        .map(|layer| {
            let mut layer_with_assets = layer.clone();
            let _ = loader.load_layer_assets(&mut layer_with_assets);
            serde_json::json!({
                "name": layer.name,
                "path": layer.path.original().display().to_string(),
                "assets": layer_with_assets.assets.len(),
            })
        })
        .collect();

    serde_json::json!(layers)
}

pub fn cmd_interactive(
    cwd: &Path,
    json: bool,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    let config = calvin::config::Config::load_or_default(Some(cwd));
    let state = detect_state(cwd, &config)?;
    let ui = crate::ui::context::UiContext::new(json, verbose, color, no_animation, &config);

    if json {
        // Resolve layer stack for JSON output (PRD §12.1)
        let layers_json = resolve_layers_for_json(cwd, &config);

        let output = match state {
            ProjectState::NoPromptPack => serde_json::json!({
                "event": "data",
                "command": "interactive",
                "state": "no_promptpack",
                "layers": layers_json,
            }),
            ProjectState::GlobalLayersOnly(count) => serde_json::json!({
                "event": "data",
                "command": "interactive",
                // Canonical state name (more accurate than the legacy name below).
                "state": "global_layers_only",
                // Backward-compatibility alias for older consumers.
                "state_aliases": ["user_layer_only"],
                "assets": { "total": count.total },
                "layers": layers_json,
            }),
            ProjectState::EmptyPromptPack => serde_json::json!({
                "event": "data",
                "command": "interactive",
                "state": "empty_promptpack",
                "layers": layers_json,
            }),
            ProjectState::Configured(count) => serde_json::json!({
                "event": "data",
                "command": "interactive",
                "state": "configured",
                "assets": { "total": count.total },
                "layers": layers_json,
            }),
        };
        crate::ui::json::emit(output)?;
        return Ok(());
    }

    if !std::io::stdin().is_terminal() {
        println!("No command provided.");
        println!("Try: `calvin deploy` or `calvin --help`");
        return Ok(());
    }

    print_banner(&ui);

    match state {
        ProjectState::NoPromptPack => menu::interactive_first_run(cwd, &ui, verbose),
        ProjectState::GlobalLayersOnly(count) => menu::interactive_global_layers_only(
            cwd,
            count.total,
            &ui,
            verbose,
            color,
            no_animation,
        ),
        ProjectState::EmptyPromptPack => {
            menu::interactive_existing_project(cwd, None, &ui, verbose, color, no_animation)
        }
        ProjectState::Configured(count) => menu::interactive_existing_project(
            cwd,
            Some(count.total),
            &ui,
            verbose,
            color,
            no_animation,
        ),
    }
}

fn print_banner(ui: &crate::ui::context::UiContext) {
    print!(
        "{}",
        crate::ui::views::interactive::render_banner(ui.color, ui.unicode)
    );
}

pub(crate) fn print_learn(ui: &crate::ui::context::UiContext) {
    print!(
        "{}",
        crate::ui::views::interactive::render_section_header(
            "The Problem Calvin Solves",
            ui.color,
            ui.unicode
        )
    );
    println!();
    println!("You use AI coding assistants (Claude, Cursor, Copilot...).");
    println!("Each one stores rules/commands in different locations.");
    println!("Maintaining them separately is tedious and error-prone.\n");
    print!(
        "{}",
        crate::ui::views::interactive::render_section_header("The Solution", ui.color, ui.unicode)
    );
    println!();
    println!("With Calvin, you write once in `.promptpack/`, then deploy everywhere:");
    println!("  `calvin deploy`\n");
}

pub(crate) fn print_commands() {
    println!("Commands:");
    println!("  calvin deploy            Deploy to this project");
    println!("  calvin deploy --home     Deploy to home directory");
    println!("  calvin deploy --remote   Deploy to remote destination");
    println!("  calvin check             Validate configuration and security");
    println!("  calvin watch             Watch and deploy on changes");
    println!("  calvin diff              Preview changes");
    println!("  calvin explain           Explain Calvin usage\n");
}
