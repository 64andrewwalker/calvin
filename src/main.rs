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
    /// Install PromptPack assets to system/user scope
    Install {
        /// Path to .promptpack directory
        #[arg(short, long, default_value = ".promptpack")]
        source: PathBuf,

        /// Install to user scope (home directory)
        #[arg(long)]
        user: bool,

        /// Force overwrite of modified files
        #[arg(short, long)]
        force: bool,

        /// Dry run - show what would be done
        #[arg(long)]
        dry_run: bool,
    },

    /// Compile and sync PromptPack to target platforms
    Sync {
        /// Path to .promptpack directory
        #[arg(short, long, default_value = ".promptpack")]
        source: PathBuf,

        /// Remote destination (user@host:/path)
        #[arg(long)]
        remote: Option<String>,

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

    /// Security audit for CI (exits non-zero on violations)
    Audit {
        /// Security mode (strict recommended for CI)
        #[arg(long, default_value = "strict")]
        mode: String,

        /// Fail on warnings too
        #[arg(long)]
        strict_warnings: bool,
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
        Commands::Sync { source, remote, force, dry_run } => {
            cmd_sync(&source, remote, force, dry_run, cli.json)
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
        Commands::Audit { mode, strict_warnings } => {
            cmd_audit(&mode, strict_warnings, cli.json)
        }
        Commands::Parse { source } => {
            cmd_parse(&source, cli.json)
        }
        Commands::Install { source, user, force, dry_run } => {
            cmd_install(&source, user, force, dry_run, cli.json)
        }
    }
}

fn cmd_install(source: &PathBuf, user: bool, force: bool, dry_run: bool, json: bool) -> Result<()> {
    use calvin::sync::{compile_assets, sync_outputs, SyncOptions};
    use calvin::models::Scope;

    if !json {
        println!("📦 Calvin Install");
        println!("Source: {}", source.display());
    }

    if !user {
        anyhow::bail!("Install currently only supports --user flag to install to home directory. Use 'sync' for project synchronization.");
    }

    if !json {
        println!("Mode: User scope (Home directory)");
        if force {
            println!("Option: Force overwrite");
        }
        if dry_run {
            println!("Option: Dry run");
        }
    }

    // Load configuration
    let config_path = source.join("config.toml");
    let config = calvin::config::Config::load(&config_path).unwrap_or_default();

    // Parse source directory
    let assets = calvin::parser::parse_directory(source)?;
    let total_assets = assets.len();
    
    // Filter for user scope assets only
    let user_assets: Vec<_> = assets.into_iter()
        .filter(|a| a.frontmatter.scope == Scope::User)
        .collect();

    if !json {
        println!("\n✓ Found {} user-scoped assets (of {} total)", user_assets.len(), total_assets);
    }
    
    if user_assets.is_empty() {
        if !json {
            println!("Nothing to install.");
        }
        return Ok(());
    }

    // Compile to all targets
    let outputs = compile_assets(&user_assets, &[], &config)?;
    
    if !json {
        println!("✓ Compiled to {} output files", outputs.len());
    }

    // Set up sync options
    let options = SyncOptions {
        force,
        dry_run,
        targets: Vec::new(),
    };

    // Determine target root (Home directory)
    let dist_root = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine user home directory"))?;

    // Sync outputs to filesystem
    let result = sync_outputs(&dist_root, &outputs, &options)?;

    if json {
        let output = serde_json::json!({
            "event": "install",
            "status": if result.is_success() { "success" } else { "partial" },
            "written": result.written.len(),
            "skipped": result.skipped.len(),
            "errors": result.errors.len()
        });
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!("\n📊 Install Results:");
        if !result.written.is_empty() {
            println!("  ✓ Written: {} files", result.written.len());
            for path in &result.written {
                println!("    - {}", path);
            }
        }
        if !result.skipped.is_empty() {
            println!("  ⚠ Skipped: {} files", result.skipped.len());
        }
        if !result.errors.is_empty() {
            println!("  ✗ Errors: {}", result.errors.len());
        }
        println!();
    }

    Ok(())
}

fn cmd_sync(source: &PathBuf, remote: Option<String>, force: bool, dry_run: bool, json: bool) -> Result<()> {
    use calvin::sync::{compile_assets, sync_outputs, sync_with_fs, SyncOptions};

    if !json {
        println!("📦 Calvin Sync");
        println!("Source: {}", source.display());
        if let Some(r) = &remote {
             println!("Remote: {}", r);
        }
        if force {
            println!("Mode: Force overwrite");
        }
        if dry_run {
            println!("Mode: Dry run");
        }
    }

    // Load configuration
    let config_path = source.join("config.toml");
    let config = calvin::config::Config::load(&config_path).unwrap_or_default();

    // Parse source directory
    let assets = calvin::parser::parse_directory(source)?;
    
    if !json {
        println!("\n✓ Parsed {} assets", assets.len());
    }

    // Compile to all targets
    let outputs = compile_assets(&assets, &[], &config)?;
    
    if !json {
        println!("✓ Compiled to {} output files", outputs.len());
    }

    // Set up sync options
    let options = SyncOptions {
        force,
        dry_run,
        targets: Vec::new(),
    };

    let result = if let Some(remote_str) = remote {
         let (host, path) = if let Some((h, p)) = remote_str.split_once(':') {
            (h, p)
        } else {
            (remote_str.as_str(), ".")
        };
        
        let fs = calvin::fs::RemoteFileSystem::new(host);
        let dist_root = std::path::PathBuf::from(path);
        sync_with_fs(&dist_root, &outputs, &options, &fs)?
    } else {
        // Determine project root (parent of source directory)
        let project_root = source.parent().unwrap_or(std::path::Path::new("."));
        sync_outputs(project_root, &outputs, &options)?
    };

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

fn cmd_watch(source: &PathBuf, json: bool) -> Result<()> {
    use calvin::watcher::{watch, WatchOptions, WatchEvent};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    // Determine project root
    let project_root = source.parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // Load configuration
    let config_path = source.join("config.toml");
    let config = calvin::config::Config::load(&config_path).unwrap_or_default();

    let options = WatchOptions {
        source: source.clone(),
        project_root,
        targets: vec![],
        json,
        config, 
    };

    // Set up Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();
    
    ctrlc::set_handler(move || {
        running_clone.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl+C handler");

    if !json {
        println!("👀 Calvin Watch");
        println!("Source: {}", source.display());
        println!("Press Ctrl+C to stop\n");
    }

    // Start watching
    watch(options, running, |event| {
        if json {
            println!("{}", event.to_json());
        } else {
            match event {
                WatchEvent::Started { source } => {
                    println!("📂 Watching: {}", source);
                }
                WatchEvent::FileChanged { path } => {
                    println!("📝 Changed: {}", path);
                }
                WatchEvent::SyncStarted => {
                    println!("🔄 Syncing...");
                }
                WatchEvent::SyncComplete { written, skipped, errors } => {
                    if errors > 0 {
                        println!("⚠ Sync: {} written, {} skipped, {} errors", 
                            written, skipped, errors);
                    } else {
                        println!("✓ Sync: {} written, {} skipped", written, skipped);
                    }
                }
                WatchEvent::Error { message } => {
                    eprintln!("✗ Error: {}", message);
                }
                WatchEvent::Shutdown => {
                    println!("\n👋 Shutting down...");
                }
            }
        }
    })?;

    Ok(())
}

fn cmd_diff(source: &PathBuf, json: bool) -> Result<()> {
    use calvin::sync::compile_assets;
    use std::fs;

    if !json {
        println!("📊 Calvin Diff");
        println!("Source: {}", source.display());
        println!();
    }

    // Load configuration
    let config_path = source.join("config.toml");
    let config = calvin::config::Config::load(&config_path).unwrap_or_default();

    // Parse source directory
    let assets = calvin::parser::parse_directory(source)?;
    
    // Compile to get expected outputs
    let outputs = compile_assets(&assets, &[], &config)?;

    // Determine project root
    let project_root = source.parent()
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
        println!("Summary: {} new, {} modified, {} unchanged",
            new_files.len(),
            modified_files.len(),
            unchanged_files.len()
        );
    }

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

fn cmd_audit(mode: &str, strict_warnings: bool, json: bool) -> Result<()> {
    use calvin::config::SecurityMode;
    use calvin::security::{run_doctor, CheckStatus};

    let security_mode = match mode {
        "yolo" => SecurityMode::Yolo,
        "balanced" => SecurityMode::Balanced,
        _ => SecurityMode::Strict, // Default to strict for CI
    };

    let project_root = std::env::current_dir()?;

    if !json {
        println!("🔒 Calvin Security Audit");
        println!("Mode: {:?}", security_mode);
        if strict_warnings {
            println!("Strict: failing on warnings");
        }
        println!();
    }

    let report = run_doctor(&project_root, security_mode);

    // Determine exit status
    let has_issues = if strict_warnings {
        report.errors() > 0 || report.warnings() > 0
    } else {
        report.errors() > 0
    };

    if json {
        let output = serde_json::json!({
            "event": "audit",
            "mode": format!("{:?}", security_mode),
            "passes": report.passes(),
            "warnings": report.warnings(),
            "errors": report.errors(),
            "success": !has_issues
        });
        println!("{}", serde_json::to_string(&output)?);
    } else {
        for check in &report.checks {
            let icon = match check.status {
                CheckStatus::Pass => "✓",
                CheckStatus::Warning => "⚠",
                CheckStatus::Error => "✗",
            };
            println!("{} [{}] {}: {}", icon, check.platform, check.name, check.message);
        }
        
        println!();
        println!("Result: {} passed, {} warnings, {} errors",
            report.passes(),
            report.warnings(),
            report.errors()
        );
    }

    if has_issues {
        if !json {
            println!();
            println!("🔴 Audit FAILED - security issues detected");
        }
        std::process::exit(1);
    } else {
        if !json {
            println!();
            println!("🟢 Audit PASSED");
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
        
        if let Commands::Sync { source, force, dry_run, .. } = cli.command {
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

    #[test]
    fn test_cli_parse_audit() {
        let cli = Cli::try_parse_from(["calvin", "audit"]).unwrap();
        if let Commands::Audit { mode, strict_warnings } = cli.command {
            assert_eq!(mode, "strict"); // Default
            assert!(!strict_warnings);
        } else {
            panic!("Expected Audit command");
        }
    }

    #[test]
    fn test_cli_parse_audit_with_options() {
        let cli = Cli::try_parse_from([
            "calvin", "audit", 
            "--mode", "balanced",
            "--strict-warnings"
        ]).unwrap();
        if let Commands::Audit { mode, strict_warnings } = cli.command {
            assert_eq!(mode, "balanced");
            assert!(strict_warnings);
        } else {
            panic!("Expected Audit command");
        }
    }

    #[test]
    fn test_cli_parse_diff() {
        let cli = Cli::try_parse_from(["calvin", "diff", "--source", "my-pack"]).unwrap();
        if let Commands::Diff { source } = cli.command {
            assert_eq!(source, PathBuf::from("my-pack"));
        } else {
            panic!("Expected Diff command");
        }
    }

    #[test]
    fn test_cli_parse_watch() {
        let cli = Cli::try_parse_from(["calvin", "watch", "--source", ".promptpack"]).unwrap();
        if let Commands::Watch { source } = cli.command {
            assert_eq!(source, PathBuf::from(".promptpack"));
        } else {
            panic!("Expected Watch command");
        }
    }

    #[test]
    fn test_cli_parse_install() {
        let cli = Cli::try_parse_from(["calvin", "install", "--user"]).unwrap();
        if let Commands::Install { user, .. } = cli.command {
            assert!(user);
        } else {
            panic!("Expected Install command");
        }
    }

    #[test]
    fn test_cli_parse_sync_remote() {
        let cli = Cli::try_parse_from(["calvin", "sync", "--remote", "user@host"]).unwrap();
        if let Commands::Sync { remote, .. } = cli.command {
            assert_eq!(remote, Some("user@host".to_string()));
        } else {
            panic!("Expected Sync command");
        }
    }
}
