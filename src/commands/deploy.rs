use std::path::{Path, PathBuf};

use anyhow::Result;
use calvin::Target;

use crate::ui::menu::{select_targets_interactive, ALL_TARGETS};
use crate::ui::output::{maybe_warn_allow_naked, print_config_warnings};

#[derive(Debug)]
enum DeployTarget {
    Project,
    Home,
    Remote(String),
}

#[derive(Debug, Clone, Copy)]
enum ScopePolicy {
    Keep,
    UserOnly,
    ForceUser,
}

#[allow(clippy::too_many_arguments)]
fn run_deploy(
    action: &'static str,
    event: &'static str,
    source: &Path,
    target: DeployTarget,
    scope_policy: ScopePolicy,
    cli_targets: &Option<Vec<Target>>,
    force: bool,
    interactive: bool,
    dry_run: bool,
    json: bool,
) -> Result<()> {
    use calvin::models::Scope;
    use calvin::sync::{compile_assets, sync_outputs, sync_with_fs, SyncOptions};

    if !json {
        println!("📦 Calvin {}", action);
        println!("Source: {}", source.display());
        match &target {
            DeployTarget::Project => {}
            DeployTarget::Home => println!("Target: Home (~/)"),
            DeployTarget::Remote(remote) => println!("Remote: {}", remote),
        }
        match scope_policy {
            ScopePolicy::Keep => {}
            ScopePolicy::UserOnly => println!("Mode: User scope (only scope: user assets)"),
            ScopePolicy::ForceUser => println!("Mode: Global (all assets to home directory)"),
        }
        if force {
            println!("Mode: Force overwrite");
        }
        if interactive {
            println!("Mode: Interactive");
        }
        if dry_run {
            println!("Mode: Dry run");
        }
    }

    // Load configuration early to get previous target selection
    let config_path = source.join("config.toml");
    let (config, warnings) = calvin::config::Config::load_with_warnings(&config_path)
        .unwrap_or((calvin::config::Config::default(), Vec::new()));
    if !json {
        print_config_warnings(&config_path, &warnings);
    }
    maybe_warn_allow_naked(&config, json);

    // Determine selected targets - interactive selection if not specified
    let selected_targets: Vec<Target> = if let Some(t) = cli_targets {
        if t.contains(&Target::All) {
            ALL_TARGETS.to_vec()
        } else {
            t.clone()
        }
    } else {
        // Use interactive selection with config defaults
        match select_targets_interactive(&config, json) {
            Some(t) => t,
            None => return Ok(()),
        }
    };

    // Check for .git directory (project scope deploy should be in a git repo)
    if matches!(target, DeployTarget::Project) {
        let project_root = source.parent().unwrap_or(Path::new("."));
        let git_dir = project_root.join(".git");
        if !git_dir.exists() {
            if !json {
                println!("\n⚠️  Warning: Not a git repository");
                println!("   Project scope files will be created in: {}", project_root.display());
                println!("   These files are typically committed to source control.");
            }
            if interactive && !force && !dry_run {
                print!("\nContinue anyway? [y/N]: ");
                use std::io::{self, Write};
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    if !json {
                        println!("Aborted.");
                    }
                    return Ok(());
                }
            }
        }
    }

    // Parse source directory
    let mut assets = calvin::parser::parse_directory(source)?;
    let total_assets = assets.len();

    match scope_policy {
        ScopePolicy::Keep => {
            if !json {
                println!("\n✓ Parsed {} assets", total_assets);
            }
        }
        ScopePolicy::ForceUser => {
            // Force all assets to User scope so they generate to home directories
            for asset in &mut assets {
                asset.frontmatter.scope = Scope::User;
            }
            if !json {
                println!("\n✓ Deploying {} assets to global directories", total_assets);
            }
        }
        ScopePolicy::UserOnly => {
            assets.retain(|a| a.frontmatter.scope == Scope::User);

            if !json {
                println!(
                    "\n✓ Found {} user-scoped assets (of {} total)",
                    assets.len(),
                    total_assets
                );
            }

            if assets.is_empty() {
                if !json {
                    println!("Nothing to install. Use --global to install all assets.");
                }
                return Ok(());
            }
        }
    }

    // Compile to selected targets
    let outputs = compile_assets(&assets, &selected_targets, &config)?;

    if !json {
        println!("✓ Compiled to {} output files", outputs.len());
    }

    // Set up sync options
    let options = SyncOptions {
        force,
        dry_run,
        interactive,
        targets: selected_targets,
    };

    let result = match target {
        DeployTarget::Project => {
            // Determine project root (parent of source directory)
            let project_root = source.parent().unwrap_or(Path::new("."));
            sync_outputs(project_root, &outputs, &options)?
        }
        DeployTarget::Home => {
            let dist_root = dirs::home_dir()
                .ok_or_else(|| anyhow::anyhow!("Could not determine user home directory"))?;
            sync_outputs(&dist_root, &outputs, &options)?
        }
        DeployTarget::Remote(remote_str) => {
            let (host, path) = if let Some((h, p)) = remote_str.split_once(':') {
                (h, p)
            } else {
                (remote_str.as_str(), ".")
            };
            let fs = calvin::fs::RemoteFileSystem::new(host);
            let dist_root = PathBuf::from(path);
            sync_with_fs(&dist_root, &outputs, &options, &fs)?
        }
    };

    if json {
        let output = serde_json::json!({
            "event": event,
            "status": if result.is_success() { "success" } else { "partial" },
            "written": result.written.len(),
            "skipped": result.skipped.len(),
            "errors": result.errors.len()
        });
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!("\n📊 {} Results:", action);
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

#[allow(clippy::too_many_arguments)]
pub fn cmd_deploy(
    source: &Path,
    home: bool,
    remote: Option<String>,
    targets: &Option<Vec<Target>>,
    force: bool,
    interactive: bool,
    dry_run: bool,
    json: bool,
    _verbose: u8,
) -> Result<()> {
    if home && remote.is_some() {
        anyhow::bail!("--home and --remote cannot be used together");
    }

    let target = if home {
        DeployTarget::Home
    } else if let Some(remote) = remote {
        DeployTarget::Remote(remote)
    } else {
        DeployTarget::Project
    };

    let scope_policy = if home {
        ScopePolicy::ForceUser
    } else {
        ScopePolicy::Keep
    };

    run_deploy(
        "Deploy",
        "deploy",
        source,
        target,
        scope_policy,
        targets,
        force,
        interactive,
        dry_run,
        json,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn cmd_install(
    source: &Path,
    user: bool,
    global: bool,
    cli_targets: &Option<Vec<Target>>,
    force: bool,
    interactive: bool,
    dry_run: bool,
    json: bool,
) -> Result<()> {
    if !user && !global {
        anyhow::bail!(
            "Install requires --user or --global flag. Use --global to install all assets to home directory, or --user for only scope: user assets."
        );
    }

    let scope_policy = if global {
        ScopePolicy::ForceUser
    } else {
        ScopePolicy::UserOnly
    };

    run_deploy(
        "Install",
        "install",
        source,
        DeployTarget::Home,
        scope_policy,
        cli_targets,
        force,
        interactive,
        dry_run,
        json,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn cmd_sync(
    source: &Path,
    remote: Option<String>,
    targets: &Option<Vec<Target>>,
    force: bool,
    interactive: bool,
    dry_run: bool,
    json: bool,
    _verbose: u8,
) -> Result<()> {
    let target = if let Some(remote) = remote {
        DeployTarget::Remote(remote)
    } else {
        DeployTarget::Project
    };

    run_deploy(
        "Sync",
        "sync",
        source,
        target,
        ScopePolicy::Keep,
        targets,
        force,
        interactive,
        dry_run,
        json,
    )
}
