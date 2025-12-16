//! Calvin CLI - PromptOps compiler and synchronization tool
//!
//! Usage: calvin <COMMAND>
//!
//! Commands:
//!   sync    Compile and sync PromptPack to target platforms
//!   watch   Watch for changes and sync continuously
//!   diff    Preview changes without writing
//!   doctor  Validate platform configurations

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

/// Calvin - PromptOps compiler and synchronization tool
#[derive(Parser, Debug)]
#[command(name = "calvin")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Output format for CI
    #[arg(long, default_value = "false")]
    json: bool,

    /// Verbosity level (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Compile and sync PromptPack to target platforms
    Sync {
        /// Path to .promptpack directory
        #[arg(short, long, default_value = ".promptpack")]
        source: PathBuf,

        /// Force overwrite of modified files
        #[arg(short, long)]
        force: bool,

        /// Dry run - show what would be done
        #[arg(long)]
        dry_run: bool,
    },

    /// Watch for changes and sync continuously
    Watch {
        /// Path to .promptpack directory
        #[arg(short, long, default_value = ".promptpack")]
        source: PathBuf,
    },

    /// Preview changes without writing
    Diff {
        /// Path to .promptpack directory
        #[arg(short, long, default_value = ".promptpack")]
        source: PathBuf,
    },

    /// Validate platform configurations
    Doctor {
        /// Security mode to validate against
        #[arg(long, default_value = "balanced")]
        mode: String,
    },

    /// Parse and display PromptPack files (debugging)
    Parse {
        /// Path to .promptpack directory
        #[arg(short, long, default_value = ".promptpack")]
        source: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Sync { source, force, dry_run } => {
            cmd_sync(&source, force, dry_run, cli.json)
        }
        Commands::Watch { source } => {
            cmd_watch(&source, cli.json)
        }
        Commands::Diff { source } => {
            cmd_diff(&source, cli.json)
        }
        Commands::Doctor { mode } => {
            cmd_doctor(&mode, cli.json)
        }
        Commands::Parse { source } => {
            cmd_parse(&source, cli.json)
        }
    }
}

fn cmd_sync(source: &PathBuf, force: bool, dry_run: bool, json: bool) -> Result<()> {
    use calvin::sync::{compile_assets, sync_outputs, SyncOptions};

    if !json {
        println!("📦 Calvin Sync");
        println!("Source: {}", source.display());
        if force {
            println!("Mode: Force overwrite");
        }
        if dry_run {
            println!("Mode: Dry run");
        }
    }

    // Parse source directory
    let assets = calvin::parser::parse_directory(source)?;
    
    if !json {
        println!("\n✓ Parsed {} assets", assets.len());
    }

    // Compile to all targets
    let outputs = compile_assets(&assets, &[])?;
    
    if !json {
        println!("✓ Compiled to {} output files", outputs.len());
    }

    // Set up sync options
    let options = SyncOptions {
        force,
        dry_run,
        targets: Vec::new(),
    };

    // Determine project root (parent of source directory)
    let project_root = source.parent().unwrap_or(std::path::Path::new("."));

    // Sync outputs to filesystem
    let result = sync_outputs(project_root, &outputs, &options)?;

    if json {
        let output = serde_json::json!({
            "event": "sync",
            "status": if result.is_success() { "success" } else { "partial" },
            "written": result.written.len(),
            "skipped": result.skipped.len(),
            "errors": result.errors.len()
        });
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!("\n📊 Sync Results:");
        if !result.written.is_empty() {
            println!("  ✓ Written: {} files", result.written.len());
            for path in &result.written {
                println!("    - {}", path);
            }
        }
        if !result.skipped.is_empty() {
            println!("  ⚠ Skipped: {} files (modified by user)", result.skipped.len());
            for path in &result.skipped {
                println!("    - {}", path);
            }
        }
        if !result.errors.is_empty() {
            println!("  ✗ Errors: {}", result.errors.len());
            for err in &result.errors {
                println!("    - {}", err);
            }
        }
        println!();
    }

    Ok(())
}

fn cmd_watch(source: &PathBuf, _json: bool) -> Result<()> {
    println!("👀 Watch mode not yet implemented");
    println!("Would watch: {}", source.display());
    Ok(())
}

fn cmd_diff(source: &PathBuf, _json: bool) -> Result<()> {
    println!("📊 Diff mode not yet implemented");
    println!("Would compare: {}", source.display());
    Ok(())
}

fn cmd_doctor(mode: &str, json: bool) -> Result<()> {
    use calvin::config::SecurityMode;
    use calvin::security::{run_doctor, CheckStatus};

    let security_mode = match mode {
        "yolo" => SecurityMode::Yolo,
        "strict" => SecurityMode::Strict,
        _ => SecurityMode::Balanced,
    };

    // Use current directory as project root
    let project_root = std::env::current_dir()?;

    if !json {
        println!("🩺 Calvin Doctor");
        println!("Mode: {:?}", security_mode);
        println!();
    }

    let report = run_doctor(&project_root, security_mode);

    if json {
        let output = serde_json::json!({
            "event": "doctor",
            "mode": format!("{:?}", security_mode),
            "passes": report.passes(),
            "warnings": report.warnings(),
            "errors": report.errors(),
            "success": report.is_success()
        });
        println!("{}", serde_json::to_string(&output)?);
    } else {
        // Group by platform
        let mut current_platform = String::new();
        
        for check in &report.checks {
            if check.platform != current_platform {
                if !current_platform.is_empty() {
                    println!();
                }
                println!("{}", check.platform);
                current_platform = check.platform.clone();
            }
            
            let icon = match check.status {
                CheckStatus::Pass => "✓",
                CheckStatus::Warning => "⚠",
                CheckStatus::Error => "✗",
            };
            
            println!("  {} {} - {}", icon, check.name, check.message);
            
            if let Some(rec) = &check.recommendation {
                println!("    ↳ {}", rec);
            }
        }
        
        println!();
        println!("Summary: {} passed, {} warnings, {} errors",
            report.passes(),
            report.warnings(),
            report.errors()
        );
        
        if !report.is_success() {
            println!();
            println!("🔴 Doctor found issues. Run with --mode balanced or fix the errors.");
        } else if report.warnings() > 0 {
            println!();
            println!("🟡 Doctor passed with warnings.");
        } else {
            println!();
            println!("🟢 All checks passed!");
        }
    }

    Ok(())
}

fn cmd_parse(source: &PathBuf, json: bool) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_sync() {
        let cli = Cli::try_parse_from(["calvin", "sync"]).unwrap();
        assert!(matches!(cli.command, Commands::Sync { .. }));
    }

    #[test]
    fn test_cli_parse_sync_with_args() {
        let cli = Cli::try_parse_from([
            "calvin",
            "sync",
            "--source", "my-pack",
            "--force",
            "--dry-run"
        ]).unwrap();
        
        if let Commands::Sync { source, force, dry_run } = cli.command {
            assert_eq!(source, PathBuf::from("my-pack"));
            assert!(force);
            assert!(dry_run);
        } else {
            panic!("Expected Sync command");
        }
    }

    #[test]
    fn test_cli_parse_doctor() {
        let cli = Cli::try_parse_from(["calvin", "doctor", "--mode", "strict"]).unwrap();
        if let Commands::Doctor { mode } = cli.command {
            assert_eq!(mode, "strict");
        } else {
            panic!("Expected Doctor command");
        }
    }

    #[test]
    fn test_cli_json_flag() {
        let cli = Cli::try_parse_from(["calvin", "--json", "sync"]).unwrap();
        assert!(cli.json);
    }

    #[test]
    fn test_cli_verbose_flag() {
        let cli = Cli::try_parse_from(["calvin", "-vvv", "sync"]).unwrap();
        assert_eq!(cli.verbose, 3);
    }
}
