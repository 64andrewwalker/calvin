use std::path::Path;

use anyhow::Result;

use crate::ui::output::print_config_warnings;

pub fn cmd_version(json: bool) -> Result<()> {
    let adapters = calvin::adapters::all_adapters();

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
        println!("Calvin v{}", env!("CARGO_PKG_VERSION"));
        println!("Source Format: 1.0\n");
        println!("Adapters:");
        for adapter in adapters {
            println!("  - {:<12} v{}", format!("{:?}", adapter.target()), adapter.version());
        }
    }
    Ok(())
}

pub fn cmd_migrate(format: Option<String>, adapter: Option<String>, dry_run: bool, json: bool) -> Result<()> {
    if json {
        crate::ui::json::emit(serde_json::json!({
            "event": "start",
            "command": "migrate",
            "format": format,
            "adapter": adapter,
            "dry_run": dry_run
        }))?;
    } else {
        println!("Calvin Migrate");
        if let Some(f) = &format {
            println!("Target Format: {}", f);
        }
        if let Some(a) = &adapter {
            println!("Target Adapter: {}", a);
        }
        if dry_run {
            println!("Mode: Dry run");
        }
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
        let caps = crate::ui::terminal::detect_capabilities();
        println!(
            "\n{} Already at latest version (1.0). No migration needed.",
            crate::ui::primitives::icon::Icon::Success.colored(
                caps.supports_color,
                caps.supports_unicode
            )
        );
    }

    Ok(())
}

pub fn cmd_diff(source: &Path, json: bool) -> Result<()> {
    use calvin::sync::compile_assets;
    use std::fs;

    // Load configuration
    let config_path = source.join("config.toml");
    let (config, warnings) = calvin::config::Config::load_with_warnings(&config_path)
        .unwrap_or((calvin::config::Config::default(), Vec::new()));
    let ui = crate::ui::context::UiContext::new(json, 0, None, true, &config);
    if json {
        crate::ui::json::emit(serde_json::json!({
            "event": "start",
            "command": "diff",
            "source": source.display().to_string()
        }))?;
    } else {
        print!(
            "{}",
            crate::ui::views::diff::render_diff_header(source, ui.color, ui.unicode)
        );
    }
    print_config_warnings(&config_path, &warnings, &ui);

    // Parse source directory
    let assets = calvin::parser::parse_directory(source)?;

    // Compile to get expected outputs
    let outputs = compile_assets(&assets, &[], &config)?;

    // Determine project root
    let project_root = source
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

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
        let target_path = project_root.join(&output.path);
        let path_str = output.path.display().to_string();

        if target_path.exists() {
            // Compare content
            let existing = fs::read_to_string(&target_path).unwrap_or_default();
            if existing == output.content {
                unchanged_files.push(path_str);
                if let Some(out) = &mut json_out {
                    let _ = crate::ui::json::write_event(
                        out,
                        &serde_json::json!({
                            "event": "file",
                            "command": "diff",
                            "path": output.path.display().to_string(),
                            "status": "unchanged"
                        }),
                    );
                }
            } else {
                modified_files.push(path_str);
                if let Some(out) = &mut json_out {
                    let _ = crate::ui::json::write_event(
                        out,
                        &serde_json::json!({
                            "event": "file",
                            "command": "diff",
                            "path": output.path.display().to_string(),
                            "status": "modified"
                        }),
                    );
                }
                if !json {
                    rendered_diffs.push_str(&crate::ui::views::diff::render_file_diff(
                        &output.path.display().to_string(),
                        &existing,
                        &output.content,
                        ui.color,
                    ));
                    rendered_diffs.push('\n');
                }
            }
        } else {
            new_files.push(path_str);
            if let Some(out) = &mut json_out {
                let _ = crate::ui::json::write_event(
                    out,
                    &serde_json::json!({
                        "event": "file",
                        "command": "diff",
                        "path": output.path.display().to_string(),
                        "status": "new"
                    }),
                );
            }
            if !json {
                rendered_diffs.push_str(&crate::ui::views::diff::render_file_diff(
                    &output.path.display().to_string(),
                    "",
                    &output.content,
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
                "unchanged": unchanged_files.len()
            }),
        );
    } else {
        if !rendered_diffs.is_empty() {
            print!("{rendered_diffs}");
        }

        print!(
            "{}",
            crate::ui::views::diff::render_diff_summary(
                new_files.len(),
                modified_files.len(),
                unchanged_files.len(),
                ui.color,
                ui.unicode
            )
        );
    }

    Ok(())
}

pub fn cmd_parse(source: &Path, json: bool) -> Result<()> {
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
        println!("\nFound {} assets:\n", assets.len());
        for asset in &assets {
            println!("┌─ {}", asset.id);
            println!("│  Description: {}", asset.frontmatter.description);
            println!("│  Kind: {:?}", asset.frontmatter.kind);
            println!("│  Scope: {:?}", asset.frontmatter.scope);
            println!("│  Path: {}", asset.source_path.display());
            if !asset.frontmatter.targets.is_empty() {
                println!("│  Targets: {:?}", asset.frontmatter.targets);
            }
            if let Some(apply) = &asset.frontmatter.apply {
                println!("│  Apply: {}", apply);
            }
            println!("└─");
        }
    }

    Ok(())
}
