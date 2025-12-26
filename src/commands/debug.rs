use std::path::Path;

use anyhow::Result;

use crate::ui::output::print_config_warnings;
use calvin::presentation::ColorWhen;

pub fn cmd_version(
    json: bool,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    let adapters = calvin::infrastructure::adapters::all_adapters();
    let cwd = std::env::current_dir()?;
    let config = calvin::config::Config::load_or_default(Some(&cwd));
    let ui = crate::ui::context::UiContext::new(json, verbose, color, no_animation, &config);

    if json {
        crate::ui::json::emit(serde_json::json!({
            "event": "start",
            "command": "version"
        }))?;

        let adapter_info: Vec<_> = adapters
            .iter()
            .map(|a| {
                serde_json::json!({
                    "target": format!("{:?}", a.target()).to_lowercase(),
                    "version": a.version(),
                })
            })
            .collect();

        let output = serde_json::json!({
            "calvin": env!("CARGO_PKG_VERSION"),
            "source_format": "1.0",
            "adapters": adapter_info
        });
        crate::ui::json::emit(serde_json::json!({
            "event": "complete",
            "command": "version",
            "data": output
        }))?;
    } else {
        let adapters: Vec<(String, String)> = adapters
            .into_iter()
            .map(|a| (format!("{:?}", a.target()), a.version().to_string()))
            .collect();
        print!(
            "{}",
            crate::ui::views::version::render_version(
                env!("CARGO_PKG_VERSION"),
                &adapters,
                ui.color,
                ui.unicode
            )
        );
    }
    Ok(())
}

pub fn cmd_migrate(
    format: Option<String>,
    adapter: Option<String>,
    dry_run: bool,
    json: bool,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let config = calvin::config::Config::load_or_default(Some(&cwd));
    let ui = crate::ui::context::UiContext::new(json, verbose, color, no_animation, &config);

    if json {
        crate::ui::json::emit(serde_json::json!({
            "event": "start",
            "command": "migrate",
            "format": format,
            "adapter": adapter,
            "dry_run": dry_run
        }))?;
    } else {
        print!(
            "{}",
            crate::ui::views::migrate::render_migrate_header(
                format.as_deref(),
                adapter.as_deref(),
                dry_run,
                ui.color,
                ui.unicode
            )
        );
    }

    // Lockfile migration (Phase 0): `.promptpack/.calvin.lock` → `./calvin.lock`
    let old_lockfile = cwd.join(".promptpack/.calvin.lock");
    let new_lockfile = cwd.join("calvin.lock");

    let mut changes: Vec<serde_json::Value> = Vec::new();
    if old_lockfile.exists() && !new_lockfile.exists() {
        changes.push(serde_json::json!({
            "type": "move_lockfile",
            "from": old_lockfile.display().to_string(),
            "to": new_lockfile.display().to_string(),
        }));
    }

    if changes.is_empty() {
        if json {
            crate::ui::json::emit(serde_json::json!({
                "event": "complete",
                "command": "migrate",
                "status": "success",
                "message": "Already at latest version (1.0). No migration needed.",
                "changes": []
            }))?;
        } else {
            println!();
            print!(
                "{}",
                crate::ui::views::migrate::render_migrate_complete(
                    "Already at latest version (1.0). No migration needed.",
                    ui.color,
                    ui.unicode
                )
            );
        }
        return Ok(());
    }

    if dry_run {
        if json {
            crate::ui::json::emit(serde_json::json!({
                "event": "complete",
                "command": "migrate",
                "status": "success",
                "message": "Dry run - no changes made.",
                "changes": changes
            }))?;
        } else {
            println!();
            print!(
                "{}",
                crate::ui::views::migrate::render_migrate_complete(
                    "Dry run - no changes made.",
                    ui.color,
                    ui.unicode
                )
            );
        }
        return Ok(());
    }

    // Apply changes
    if old_lockfile.exists() && !new_lockfile.exists() {
        std::fs::create_dir_all(
            new_lockfile
                .parent()
                .expect("calvin.lock parent must exist"),
        )?;
        let content = std::fs::read_to_string(&old_lockfile)?;
        std::fs::write(&new_lockfile, content)?;
        let _ = std::fs::remove_file(&old_lockfile);
    }

    if json {
        crate::ui::json::emit(serde_json::json!({
            "event": "complete",
            "command": "migrate",
            "status": "success",
            "message": "Migration complete.",
            "changes": changes
        }))?;
    } else {
        println!();
        print!(
            "{}",
            crate::ui::views::migrate::render_migrate_complete(
                "Migration complete.",
                ui.color,
                ui.unicode
            )
        );
    }

    Ok(())
}

pub fn cmd_diff(source: &Path, home: bool, json: bool) -> Result<()> {
    use calvin::application::DiffOptions;
    use calvin::domain::value_objects::{DeployTarget, Scope};
    use calvin::presentation::factory::create_diff_use_case;
    use std::fs;

    let project_root = std::env::current_dir()?;

    // Load base config (user config + project config, if present).
    let base_config = calvin::config::Config::load_or_default(Some(&project_root));

    // Merge config across resolved promptpack layers (user/custom/project).
    let project_layer_path = if source.is_relative() {
        project_root.join(source)
    } else {
        source.to_path_buf()
    };

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
            additional_layers: additional_layers.clone(),
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

    let ui = crate::ui::context::UiContext::new(json, 0, None, true, &config);

    // Determine effective scope: CLI flag overrides config
    let scope = if home || config.deploy.target == DeployTarget::Home {
        Scope::User
    } else {
        Scope::Project
    };

    if json {
        crate::ui::json::emit(serde_json::json!({
            "event": "start",
            "command": "diff",
            "source": source.display().to_string(),
            "home": scope == Scope::User
        }))?;
    } else {
        print!(
            "{}",
            crate::ui::views::diff::render_diff_header(source, ui.color, ui.unicode)
        );
    }

    // Create and execute DiffUseCase
    let use_case = create_diff_use_case();
    let mut targets = config.enabled_targets();
    if targets.contains(&calvin::Target::All) {
        targets = calvin::Target::ALL_CONCRETE.to_vec();
    }
    let options = DiffOptions::new(source)
        .with_scope(scope)
        .with_project_root(&project_root)
        .with_targets(targets)
        .with_project_layer_enabled(!base_config.sources.disable_project_layer)
        .with_user_layer_enabled(use_user_layer)
        .with_additional_layers_enabled(use_additional_layers)
        .with_additional_layers(additional_layers);
    let options = if let Some(path) = base_config.sources.user_layer_path.clone() {
        options.with_user_layer_path(path)
    } else {
        options
    };
    let result = use_case.execute(&options);

    // Determine compare root for reading existing content
    let compare_root = if scope == Scope::User {
        calvin::infrastructure::calvin_home_dir().unwrap_or_default()
    } else {
        project_root
    };

    let mut rendered_diffs = String::new();
    let mut json_out = if json {
        Some(std::io::stdout().lock())
    } else {
        None
    };

    // Process creates
    for entry in &result.creates {
        let path_str = entry.path.display().to_string();
        if let Some(out) = &mut json_out {
            let _ = crate::ui::json::write_event(
                out,
                &serde_json::json!({
                    "event": "file",
                    "command": "diff",
                    "path": path_str,
                    "status": "new"
                }),
            );
        }
        if !json {
            if let Some(content) = &entry.new_content {
                rendered_diffs.push_str(&crate::ui::views::diff::render_file_diff(
                    &path_str, "", content, ui.color,
                ));
                rendered_diffs.push('\n');
            }
        }
    }

    // Process updates
    for entry in &result.updates {
        let path_str = entry.path.display().to_string();
        if let Some(out) = &mut json_out {
            let _ = crate::ui::json::write_event(
                out,
                &serde_json::json!({
                    "event": "file",
                    "command": "diff",
                    "path": path_str,
                    "status": "modified"
                }),
            );
        }
        if !json {
            // Read existing content to generate diff
            let target_path = if path_str.starts_with('~') {
                calvin::infrastructure::fs::expand_home(&entry.path)
            } else {
                compare_root.join(&entry.path)
            };
            let existing = fs::read_to_string(&target_path).unwrap_or_default();
            if let Some(new_content) = &entry.new_content {
                rendered_diffs.push_str(&crate::ui::views::diff::render_file_diff(
                    &path_str,
                    &existing,
                    new_content,
                    ui.color,
                ));
                rendered_diffs.push('\n');
            }
        }
    }

    // Process skipped (unchanged)
    for entry in &result.skipped {
        let path_str = entry.path.display().to_string();
        if let Some(out) = &mut json_out {
            let _ = crate::ui::json::write_event(
                out,
                &serde_json::json!({
                    "event": "file",
                    "command": "diff",
                    "path": path_str,
                    "status": "unchanged"
                }),
            );
        }
    }

    // Process conflicts
    for entry in &result.conflicts {
        let path_str = entry.path.display().to_string();
        if let Some(out) = &mut json_out {
            let _ = crate::ui::json::write_event(
                out,
                &serde_json::json!({
                    "event": "file",
                    "command": "diff",
                    "path": path_str,
                    "status": "conflict"
                }),
            );
        }
        if !json {
            // Read existing content to generate diff
            let target_path = if path_str.starts_with('~') {
                calvin::infrastructure::fs::expand_home(&entry.path)
            } else {
                compare_root.join(&entry.path)
            };
            let existing = fs::read_to_string(&target_path).unwrap_or_default();
            if let Some(new_content) = &entry.new_content {
                rendered_diffs.push_str(&crate::ui::views::diff::render_file_diff(
                    &path_str,
                    &existing,
                    new_content,
                    ui.color,
                ));
                rendered_diffs.push('\n');
            }
        }
    }

    if json {
        let mut out = std::io::stdout().lock();
        let _ = crate::ui::json::write_event(
            &mut out,
            &serde_json::json!({
                "event": "complete",
                "command": "diff",
                "new": result.creates.len(),
                "modified": result.updates.len(),
                "unchanged": result.skipped.len(),
                "conflicts": result.conflicts.len(),
                "orphans": result.orphans.len()
            }),
        );
    } else {
        if !rendered_diffs.is_empty() {
            print!("{rendered_diffs}");
        }

        print!(
            "{}",
            crate::ui::views::diff::render_diff_summary_with_orphans(
                result.creates.len(),
                result.updates.len(),
                result.skipped.len(),
                result.orphans.len(),
                ui.color,
                ui.unicode
            )
        );
    }

    Ok(())
}

pub fn cmd_parse(source: &Path, json: bool) -> Result<()> {
    let config_path = source.join("config.toml");
    let config = calvin::config::Config::load_or_default(Some(source));
    let ui = crate::ui::context::UiContext::new(json, 0, None, true, &config);

    if json {
        crate::ui::json::emit(serde_json::json!({
            "event": "start",
            "command": "parse",
            "source": source.display().to_string()
        }))?;
    } else {
        println!("Parsing PromptPack: {}", source.display());
    }

    let assets = calvin::parser::parse_directory(source)?;

    if json {
        for asset in &assets {
            let output = serde_json::json!({
                "event": "asset",
                "id": asset.id,
                "description": asset.frontmatter.description,
                "kind": format!("{:?}", asset.frontmatter.kind),
                "scope": format!("{:?}", asset.frontmatter.scope),
                "path": asset.source_path.display().to_string(),
            });
            crate::ui::json::emit(output)?;
        }
        crate::ui::json::emit(serde_json::json!({
            "event": "complete",
            "command": "parse",
            "assets": assets.len()
        }))?;
    } else {
        print!(
            "{}",
            crate::ui::views::parse::render_parse_header(
                &source.display().to_string(),
                assets.len(),
                ui.color,
                ui.unicode
            )
        );
        println!();
        print_config_warnings(&config_path, &[], &ui);
        for asset in &assets {
            print!(
                "{}",
                crate::ui::views::parse::render_asset(asset, ui.color, ui.unicode)
            );
            println!();
        }
    }

    Ok(())
}
