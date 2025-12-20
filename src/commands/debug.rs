use std::path::Path;

use anyhow::Result;

use crate::cli::ColorWhen;
use crate::ui::output::print_config_warnings;

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

    // Placeholder for future migration logic
    // Currently only version 1.0 exists.

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

    Ok(())
}

pub fn cmd_diff(source: &Path, home: bool, json: bool) -> Result<()> {
    // Check if we should use the new engine
    if should_use_new_diff_engine() {
        cmd_diff_new_engine(source, home, json)
    } else {
        cmd_diff_legacy(source, home, json)
    }
}

/// Check if we should use the new diff engine
fn should_use_new_diff_engine() -> bool {
    !std::env::var("CALVIN_LEGACY_DIFF").is_ok_and(|v| v == "1" || v.to_lowercase() == "true")
}

/// New engine implementation using DiffUseCase
fn cmd_diff_new_engine(source: &Path, home: bool, json: bool) -> Result<()> {
    use calvin::application::DiffOptions;
    use calvin::config::DeployTargetConfig;
    use calvin::domain::value_objects::Scope;
    use calvin::presentation::factory::create_diff_use_case;
    use std::fs;

    // Load configuration
    let config_path = source.join("config.toml");
    let (config, warnings) = calvin::config::Config::load_with_warnings(&config_path)
        .unwrap_or((calvin::config::Config::default(), Vec::new()));
    let ui = crate::ui::context::UiContext::new(json, 0, None, true, &config);

    // Determine effective scope: CLI flag overrides config
    let scope = if home || config.deploy.target == DeployTargetConfig::Home {
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
    print_config_warnings(&config_path, &warnings, &ui);

    // Create and execute DiffUseCase
    let use_case = create_diff_use_case();
    let options = DiffOptions::new(source).with_scope(scope);
    let result = use_case.execute(&options);

    // Determine compare root for reading existing content
    let project_root = source
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let compare_root = if scope == Scope::User {
        dirs::home_dir().unwrap_or_default()
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
                calvin::sync::expand_home_dir(&entry.path)
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
                calvin::sync::expand_home_dir(&entry.path)
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

/// Legacy engine implementation (original)
fn cmd_diff_legacy(source: &Path, home: bool, json: bool) -> Result<()> {
    use calvin::config::DeployTargetConfig;
    use calvin::infrastructure::fs::LocalFs;
    use calvin::sync::lockfile::{Lockfile, LockfileNamespace};
    use calvin::sync::orphan::detect_orphans;
    use calvin::sync::{AssetPipeline, ScopePolicy};
    use std::fs;

    // Load configuration
    let config_path = source.join("config.toml");
    let (config, warnings) = calvin::config::Config::load_with_warnings(&config_path)
        .unwrap_or((calvin::config::Config::default(), Vec::new()));
    let ui = crate::ui::context::UiContext::new(json, 0, None, true, &config);

    // Determine effective target: CLI flag overrides config
    let use_home = home || config.deploy.target == DeployTargetConfig::Home;

    if json {
        crate::ui::json::emit(serde_json::json!({
            "event": "start",
            "command": "diff",
            "source": source.display().to_string(),
            "home": use_home
        }))?;
    } else {
        print!(
            "{}",
            crate::ui::views::diff::render_diff_header(source, ui.color, ui.unicode)
        );
    }
    print_config_warnings(&config_path, &warnings, &ui);

    // Use scope policy matching deploy behavior
    // ForceUser: all assets go to home
    // Keep: preserve original scope (project/user)
    let scope_policy = if use_home {
        ScopePolicy::ForceUser
    } else {
        ScopePolicy::Keep
    };

    let outputs = AssetPipeline::new(source.to_path_buf(), config.clone())
        .with_scope_policy(scope_policy)
        .compile()?;

    // Load lockfile and detect orphans
    let lockfile_path = source.join(".calvin.lock");
    let fs = LocalFs::new();
    let lockfile = Lockfile::load_or_new(&lockfile_path, &fs);
    let namespace = if use_home {
        LockfileNamespace::Home
    } else {
        LockfileNamespace::Project
    };
    let orphan_result = detect_orphans(&lockfile, &outputs, namespace);

    // Determine project root
    let project_root = source
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let compare_root = if use_home {
        dirs::home_dir().unwrap_or_default()
    } else {
        project_root
    };

    let mut new_files = Vec::new();
    let mut modified_files = Vec::new();
    let mut unchanged_files = Vec::new();
    let mut rendered_diffs = String::new();
    let mut json_out = if json {
        Some(std::io::stdout().lock())
    } else {
        None
    };

    for output in &outputs {
        let output_path_str = output.path().display().to_string();
        let path_str = if use_home {
            if output_path_str.starts_with('~') {
                output_path_str.clone()
            } else {
                format!("~/{}", output_path_str)
            }
        } else {
            output_path_str.clone()
        };

        // Default diff is project-level only (skip explicit home outputs).
        if !use_home && output_path_str.starts_with('~') {
            continue;
        }

        let target_path = if output_path_str.starts_with('~') {
            calvin::sync::expand_home_dir(output.path())
        } else {
            compare_root.join(output.path())
        };

        if target_path.exists() {
            // Compare content
            let existing = fs::read_to_string(&target_path).unwrap_or_default();
            if existing == output.content() {
                unchanged_files.push(path_str.clone());
                if let Some(out) = &mut json_out {
                    let _ = crate::ui::json::write_event(
                        out,
                        &serde_json::json!({
                            "event": "file",
                            "command": "diff",
                            "path": path_str.as_str(),
                            "status": "unchanged"
                        }),
                    );
                }
            } else {
                modified_files.push(path_str.clone());
                if let Some(out) = &mut json_out {
                    let _ = crate::ui::json::write_event(
                        out,
                        &serde_json::json!({
                            "event": "file",
                            "command": "diff",
                            "path": path_str.as_str(),
                            "status": "modified"
                        }),
                    );
                }
                if !json {
                    rendered_diffs.push_str(&crate::ui::views::diff::render_file_diff(
                        &path_str,
                        &existing,
                        output.content(),
                        ui.color,
                    ));
                    rendered_diffs.push('\n');
                }
            }
        } else {
            new_files.push(path_str.clone());
            if let Some(out) = &mut json_out {
                let _ = crate::ui::json::write_event(
                    out,
                    &serde_json::json!({
                        "event": "file",
                        "command": "diff",
                        "path": path_str.as_str(),
                        "status": "new"
                    }),
                );
            }
            if !json {
                rendered_diffs.push_str(&crate::ui::views::diff::render_file_diff(
                    &path_str,
                    "",
                    output.content(),
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
                "new": new_files.len(),
                "modified": modified_files.len(),
                "unchanged": unchanged_files.len(),
                "orphans": orphan_result.orphans.len()
            }),
        );
    } else {
        if !rendered_diffs.is_empty() {
            print!("{rendered_diffs}");
        }

        print!(
            "{}",
            crate::ui::views::diff::render_diff_summary_with_orphans(
                new_files.len(),
                modified_files.len(),
                unchanged_files.len(),
                orphan_result.orphans.len(),
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
