use std::path::Path;

use anyhow::Result;

use crate::ui::output::print_config_warnings;

pub fn cmd_version(json: bool) -> Result<()> {
    let adapters = calvin::adapters::all_adapters();

    if json {
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
        println!("{}", serde_json::to_string_pretty(&output)?);
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
    if !json {
        println!("🔄 Calvin Migrate");
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
        let output = serde_json::json!({
            "status": "success",
            "message": "Already at latest version (1.0). No migration needed.",
            "changes": []
        });
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!("\n✅ Already at latest version (1.0). No migration needed.");
    }

    Ok(())
}

pub fn cmd_diff(source: &Path, json: bool) -> Result<()> {
    use calvin::sync::compile_assets;
    use std::fs;

    if !json {
        println!("📊 Calvin Diff");
        println!("Source: {}", source.display());
        println!();
    }

    // Load configuration
    let config_path = source.join("config.toml");
    let (config, warnings) = calvin::config::Config::load_with_warnings(&config_path)
        .unwrap_or((calvin::config::Config::default(), Vec::new()));
    if !json {
        print_config_warnings(&config_path, &warnings);
    }

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

    for output in &outputs {
        let target_path = project_root.join(&output.path);
        let path_str = output.path.display().to_string();

        if target_path.exists() {
            // Compare content
            let existing = fs::read_to_string(&target_path).unwrap_or_default();
            if existing == output.content {
                unchanged_files.push(path_str);
            } else {
                modified_files.push(path_str);
            }
        } else {
            new_files.push(path_str);
        }
    }

    if json {
        let output = serde_json::json!({
            "event": "diff",
            "new": new_files.len(),
            "modified": modified_files.len(),
            "unchanged": unchanged_files.len()
        });
        println!("{}", serde_json::to_string(&output)?);
    } else {
        if !new_files.is_empty() {
            println!("📁 New files ({}):", new_files.len());
            for path in &new_files {
                println!("  + {}", path);
            }
            println!();
        }

        if !modified_files.is_empty() {
            println!("📝 Modified files ({}):", modified_files.len());
            for path in &modified_files {
                println!("  ~ {}", path);
            }
            println!();
        }

        if !unchanged_files.is_empty() {
            println!("✓ Unchanged files: {}", unchanged_files.len());
        }

        println!();
        println!(
            "Summary: {} new, {} modified, {} unchanged",
            new_files.len(),
            modified_files.len(),
            unchanged_files.len()
        );
    }

    Ok(())
}

pub fn cmd_parse(source: &Path, json: bool) -> Result<()> {
    if !json {
        println!("🔍 Parsing PromptPack: {}", source.display());
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
            println!("{}", serde_json::to_string(&output)?);
        }
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

