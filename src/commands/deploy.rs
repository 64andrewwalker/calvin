use std::path::{Path, PathBuf};

use anyhow::Result;
use calvin::Target;

use crate::cli::ColorWhen;
use crate::ui::menu::ALL_TARGETS;
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
    command_name: &'static str,
    source: &Path,
    target: DeployTarget,
    scope_policy: ScopePolicy,
    cli_targets: &Option<Vec<Target>>,
    force: bool,
    interactive: bool,
    dry_run: bool,
    json: bool,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    use calvin::models::Scope;
    use calvin::sync::{
        compile_assets, sync_outputs, sync_outputs_with_callback, sync_with_fs,
        sync_with_fs_with_callback, SyncEvent, SyncOptions,
    };

    // Load configuration early to drive UI settings and remembered target selection.
    let config_path = source.join("config.toml");
    let (config, warnings) = calvin::config::Config::load_with_warnings(&config_path)
        .unwrap_or((calvin::config::Config::default(), Vec::new()));
    let ui = crate::ui::context::UiContext::new(json, verbose, color, no_animation, &config);
    use crate::ui::primitives::icon::Icon;

    if json {
        let mut out = std::io::stdout().lock();
        let target_value = match &target {
            DeployTarget::Project => "project",
            DeployTarget::Home => "home",
            DeployTarget::Remote(_) => "remote",
        };
        let remote_value = match &target {
            DeployTarget::Remote(r) => Some(r.as_str()),
            _ => None,
        };
        let _ = crate::ui::json::write_event(
            &mut out,
            &serde_json::json!({
                "event": "start",
                "command": command_name,
                "source": source.display().to_string(),
                "target": target_value,
                "remote": remote_value,
                "dry_run": dry_run,
                "force": force,
            }),
        );
    }

    if !json {
        let mut modes: Vec<String> = Vec::new();
        match scope_policy {
            ScopePolicy::Keep => {}
            ScopePolicy::UserOnly => modes.push("User scope (only scope: user assets)".to_string()),
            ScopePolicy::ForceUser => modes.push("Global (all assets to home directory)".to_string()),
        }
        if force {
            modes.push("Force overwrite".to_string());
        }
        if interactive {
            modes.push("Interactive".to_string());
        }
        if dry_run {
            modes.push("Dry run".to_string());
        }

        let target_label = match &target {
            DeployTarget::Home => Some("Home (~/)"),
            _ => None,
        };
        let remote_label = match &target {
            DeployTarget::Remote(remote) => Some(remote.as_str()),
            _ => None,
        };

        print!(
            "{}",
            crate::ui::views::deploy::render_deploy_header(
                action,
                source,
                target_label,
                remote_label,
                &modes,
                ui.color,
                ui.unicode
            )
        );
    }

    print_config_warnings(&config_path, &warnings, &ui);
    maybe_warn_allow_naked(&config, &ui);

    // Determine selected targets - interactive selection if not specified
    let selected_targets: Vec<Target> = if let Some(t) = cli_targets {
        if t.contains(&Target::All) {
            ALL_TARGETS.to_vec()
        } else {
            t.clone()
        }
    } else {
        // Use interactive selection with config defaults
        // DP-7: Save selection if different from config
        let config_path = source.join("config.toml");
        match crate::ui::menu::select_targets_interactive_with_save(
            &config,
            json,
            if config_path.exists() { Some(&config_path) } else { None },
        ) {
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
                println!(
                    "\n{} Warning: Not a git repository",
                    Icon::Warning.colored(ui.color, ui.unicode)
                );
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
    let show_parse_spinner = ui.animation && !json;
    let mut parse_spinner = if show_parse_spinner {
        use crate::ui::animation::spinner::{Spinner, SpinnerStyle};

        let style = match std::env::var("CALVIN_SPINNER_STYLE").ok().as_deref() {
            Some("dots") => SpinnerStyle::Dots,
            Some("line") => SpinnerStyle::Line,
            Some("arrow") => SpinnerStyle::Arrow,
            _ => SpinnerStyle::Braille,
        };

        let mut spinner = Spinner::with_style(style, "");
        spinner.set_message(format!("Scanning {}...", source.display()));
        spinner.tick();
        println!("{}", spinner.render(ui.unicode));
        Some(spinner)
    } else {
        None
    };

    let mut assets = match calvin::parser::parse_directory(source) {
        Ok(assets) => assets,
        Err(err) => {
            if let Some(spinner) = parse_spinner.take() {
                eprintln!(
                    "{}",
                    spinner.fail("Failed to scan PromptPack", ui.color, ui.unicode)
                );
            }
            return Err(err.into());
        }
    };
    let total_assets = assets.len();

    if let Some(spinner) = parse_spinner.take() {
        println!(
            "{}",
            spinner.succeed(
                &format!("Parsed {} assets", total_assets),
                ui.color,
                ui.unicode
            )
        );
    }

    match scope_policy {
        ScopePolicy::Keep => {
            if !json && !show_parse_spinner {
                println!(
                    "\n{} Parsed {} assets",
                    Icon::Success.colored(ui.color, ui.unicode),
                    total_assets
                );
            }
        }
        ScopePolicy::ForceUser => {
            // Force all assets to User scope so they generate to home directories
            for asset in &mut assets {
                asset.frontmatter.scope = Scope::User;
            }
            if !json {
                println!(
                    "\n{} Deploying {} assets to global directories",
                    Icon::Success.colored(ui.color, ui.unicode),
                    total_assets
                );
            }
        }
        ScopePolicy::UserOnly => {
            assets.retain(|a| a.frontmatter.scope == Scope::User);

            if !json {
                println!(
                    "\n{} Found {} user-scoped assets (of {} total)",
                    Icon::Success.colored(ui.color, ui.unicode),
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
        println!(
            "{} Compiled to {} output files",
            Icon::Success.colored(ui.color, ui.unicode),
            outputs.len()
        );
    }

    let target_count = selected_targets.len();

    // Set up sync options
    let options = SyncOptions {
        force,
        dry_run,
        interactive,
        targets: selected_targets,
    };

    let live_sync_ui = ui.animation && !interactive && !json;

    let result = match target {
        DeployTarget::Project => {
            // Determine project root (parent of source directory)
            let project_root = source.parent().unwrap_or(Path::new("."));
            if json {
                let mut out = std::io::stdout().lock();
                let total = outputs.len() as u64;
                let mut done: u64 = 0;

                let mut callback = |event: SyncEvent| {
                    match &event {
                        SyncEvent::ItemStart { index, path } => {
                            let _ = crate::ui::json::write_event(
                                &mut out,
                                &serde_json::json!({
                                    "event": "item_start",
                                    "command": command_name,
                                    "index": index,
                                    "path": path,
                                }),
                            );
                        }
                        SyncEvent::ItemWritten { index, path } => {
                            done = done.saturating_add(1);
                            let _ = crate::ui::json::write_event(
                                &mut out,
                                &serde_json::json!({
                                    "event": "item_written",
                                    "command": command_name,
                                    "index": index,
                                    "path": path,
                                }),
                            );
                            let _ = crate::ui::json::write_event(
                                &mut out,
                                &serde_json::json!({
                                    "event": "progress",
                                    "command": command_name,
                                    "current": done,
                                    "total": total,
                                }),
                            );
                        }
                        SyncEvent::ItemSkipped { index, path } => {
                            done = done.saturating_add(1);
                            let _ = crate::ui::json::write_event(
                                &mut out,
                                &serde_json::json!({
                                    "event": "item_skipped",
                                    "command": command_name,
                                    "index": index,
                                    "path": path,
                                }),
                            );
                            let _ = crate::ui::json::write_event(
                                &mut out,
                                &serde_json::json!({
                                    "event": "progress",
                                    "command": command_name,
                                    "current": done,
                                    "total": total,
                                }),
                            );
                        }
                        SyncEvent::ItemError {
                            index,
                            path,
                            message,
                        } => {
                            done = done.saturating_add(1);
                            let _ = crate::ui::json::write_event(
                                &mut out,
                                &serde_json::json!({
                                    "event": "item_error",
                                    "command": command_name,
                                    "index": index,
                                    "path": path,
                                    "message": message,
                                }),
                            );
                            let _ = crate::ui::json::write_event(
                                &mut out,
                                &serde_json::json!({
                                    "event": "progress",
                                    "command": command_name,
                                    "current": done,
                                    "total": total,
                                }),
                            );
                        }
                    }
                };

                sync_outputs_with_callback(project_root, &outputs, &options, &mut callback)?
            } else if live_sync_ui {
                use crate::ui::live_region::LiveRegion;
                use crate::ui::animation::progress::{ProgressBar, ProgressStyle};
                use crate::ui::components::stream::{ItemStatus, StreamOutput};

                let mut region = LiveRegion::new();
                let mut list = StreamOutput::with_visible_count(8);
                for output in &outputs {
                    list.add(output.path.display().to_string());
                }
                let initial_anchor = outputs.len().min(8).saturating_sub(1);
                list.set_anchor(initial_anchor);

                let mut bar = ProgressBar::with_message(outputs.len() as u64, "Writing");
                if ui.unicode {
                    bar.set_style(ProgressStyle::Blocks);
                }
                let width = ui.caps.width.saturating_sub(32).clamp(10, 40);
                bar.set_width(width);

                let mut stdout = std::io::stdout().lock();

                let initial = format!("{}{}", list.render(ui.color, ui.unicode), bar.render(ui.unicode));
                region.update(&mut stdout, &initial)?;

                let mut callback = |event: SyncEvent| {
                    match event {
                        SyncEvent::ItemStart { index, .. } => list.update(index, ItemStatus::InProgress),
                        SyncEvent::ItemWritten { index, .. } => {
                            list.update(index, ItemStatus::Success);
                            bar.inc(1);
                        }
                        SyncEvent::ItemSkipped { index, .. } => {
                            list.update(index, ItemStatus::Warning);
                            bar.inc(1);
                        }
                        SyncEvent::ItemError { index, message, .. } => {
                            list.update(index, ItemStatus::Error);
                            list.update_detail(index, message);
                            bar.inc(1);
                        }
                    }
                    let content = format!("{}{}", list.render(ui.color, ui.unicode), bar.render(ui.unicode));
                    let _ = region.update(&mut stdout, &content);
                };

                let result = sync_outputs_with_callback(project_root, &outputs, &options, &mut callback)?;
                region.clear(&mut stdout)?;
                result
            } else {
                sync_outputs(project_root, &outputs, &options)?
            }
        }
        DeployTarget::Home => {
            let dist_root = dirs::home_dir()
                .ok_or_else(|| anyhow::anyhow!("Could not determine user home directory"))?;
            if json {
                let mut out = std::io::stdout().lock();
                let total = outputs.len() as u64;
                let mut done: u64 = 0;

                let mut callback = |event: SyncEvent| {
                    match &event {
                        SyncEvent::ItemStart { index, path } => {
                            let _ = crate::ui::json::write_event(
                                &mut out,
                                &serde_json::json!({
                                    "event": "item_start",
                                    "command": command_name,
                                    "index": index,
                                    "path": path,
                                }),
                            );
                        }
                        SyncEvent::ItemWritten { index, path } => {
                            done = done.saturating_add(1);
                            let _ = crate::ui::json::write_event(
                                &mut out,
                                &serde_json::json!({
                                    "event": "item_written",
                                    "command": command_name,
                                    "index": index,
                                    "path": path,
                                }),
                            );
                            let _ = crate::ui::json::write_event(
                                &mut out,
                                &serde_json::json!({
                                    "event": "progress",
                                    "command": command_name,
                                    "current": done,
                                    "total": total,
                                }),
                            );
                        }
                        SyncEvent::ItemSkipped { index, path } => {
                            done = done.saturating_add(1);
                            let _ = crate::ui::json::write_event(
                                &mut out,
                                &serde_json::json!({
                                    "event": "item_skipped",
                                    "command": command_name,
                                    "index": index,
                                    "path": path,
                                }),
                            );
                            let _ = crate::ui::json::write_event(
                                &mut out,
                                &serde_json::json!({
                                    "event": "progress",
                                    "command": command_name,
                                    "current": done,
                                    "total": total,
                                }),
                            );
                        }
                        SyncEvent::ItemError {
                            index,
                            path,
                            message,
                        } => {
                            done = done.saturating_add(1);
                            let _ = crate::ui::json::write_event(
                                &mut out,
                                &serde_json::json!({
                                    "event": "item_error",
                                    "command": command_name,
                                    "index": index,
                                    "path": path,
                                    "message": message,
                                }),
                            );
                            let _ = crate::ui::json::write_event(
                                &mut out,
                                &serde_json::json!({
                                    "event": "progress",
                                    "command": command_name,
                                    "current": done,
                                    "total": total,
                                }),
                            );
                        }
                    }
                };

                sync_outputs_with_callback(&dist_root, &outputs, &options, &mut callback)?
            } else if live_sync_ui {
                use crate::ui::live_region::LiveRegion;
                use crate::ui::animation::progress::{ProgressBar, ProgressStyle};
                use crate::ui::components::stream::{ItemStatus, StreamOutput};

                let mut region = LiveRegion::new();
                let mut list = StreamOutput::with_visible_count(8);
                for output in &outputs {
                    list.add(output.path.display().to_string());
                }
                let initial_anchor = outputs.len().min(8).saturating_sub(1);
                list.set_anchor(initial_anchor);

                let mut bar = ProgressBar::new(outputs.len() as u64);
                bar.set_message("Writing");
                if ui.unicode {
                    bar.set_style(ProgressStyle::Blocks);
                }
                let width = ui.caps.width.saturating_sub(32).clamp(10, 40);
                bar.set_width(width);

                let mut done: u64 = 0;
                let mut stdout = std::io::stdout().lock();

                let initial = format!("{}{}", list.render(ui.color, ui.unicode), bar.render(ui.unicode));
                region.update(&mut stdout, &initial)?;

                let mut callback = |event: SyncEvent| {
                    match event {
                        SyncEvent::ItemStart { index, .. } => list.update(index, ItemStatus::InProgress),
                        SyncEvent::ItemWritten { index, .. } => {
                            list.update(index, ItemStatus::Success);
                            done = done.saturating_add(1);
                            bar.set(done);
                        }
                        SyncEvent::ItemSkipped { index, .. } => {
                            list.update(index, ItemStatus::Warning);
                            done = done.saturating_add(1);
                            bar.set(done);
                        }
                        SyncEvent::ItemError { index, message, .. } => {
                            list.update(index, ItemStatus::Error);
                            list.update_detail(index, message);
                            done = done.saturating_add(1);
                            bar.set(done);
                        }
                    }
                    let content = format!("{}{}", list.render(ui.color, ui.unicode), bar.render(ui.unicode));
                    let _ = region.update(&mut stdout, &content);
                };

                let result = sync_outputs_with_callback(&dist_root, &outputs, &options, &mut callback)?;
                region.clear(&mut stdout)?;
                result
            } else {
                sync_outputs(&dist_root, &outputs, &options)?
            }
        }
        DeployTarget::Remote(remote_str) => {
            // Use rsync for fast batch transfer
            if calvin::sync::remote::has_rsync() {
                if !json {
                    println!(
                        "\n{} Using rsync for fast batch transfer...",
                        Icon::Remote.colored(ui.color, ui.unicode)
                    );
                }
                calvin::sync::remote::sync_remote_rsync(
                    &remote_str,
                    &outputs,
                    &options,
                    json,
                )?
            } else {
                // Fallback to slow SSH method
                let (host, path) = if let Some((h, p)) = remote_str.split_once(':') {
                    (h, p)
                } else {
                    (remote_str.as_str(), ".")
                };
                
                if !json {
                    println!(
                        "\n{} rsync not found, falling back to slow SSH method...",
                        Icon::Warning.colored(ui.color, ui.unicode)
                    );
                    println!("   Install rsync for 10x faster transfers.");
                    println!(
                        "\n{} Syncing {} files to remote (this may take a while)...",
                        Icon::Progress.colored(ui.color, ui.unicode),
                        outputs.len()
                    );
                }
                
                let fs = calvin::fs::RemoteFileSystem::new(host);
                let dist_root = PathBuf::from(path);
                if json {
                    let mut out = std::io::stdout().lock();
                    let total = outputs.len() as u64;
                    let mut done: u64 = 0;

                    let mut callback = |event: SyncEvent| {
                        match &event {
                            SyncEvent::ItemStart { index, path } => {
                                let _ = crate::ui::json::write_event(
                                    &mut out,
                                    &serde_json::json!({
                                        "event": "item_start",
                                        "command": command_name,
                                        "index": index,
                                        "path": path,
                                    }),
                                );
                            }
                            SyncEvent::ItemWritten { index, path } => {
                                done = done.saturating_add(1);
                                let _ = crate::ui::json::write_event(
                                    &mut out,
                                    &serde_json::json!({
                                        "event": "item_written",
                                        "command": command_name,
                                        "index": index,
                                        "path": path,
                                    }),
                                );
                                let _ = crate::ui::json::write_event(
                                    &mut out,
                                    &serde_json::json!({
                                        "event": "progress",
                                        "command": command_name,
                                        "current": done,
                                        "total": total,
                                    }),
                                );
                            }
                            SyncEvent::ItemSkipped { index, path } => {
                                done = done.saturating_add(1);
                                let _ = crate::ui::json::write_event(
                                    &mut out,
                                    &serde_json::json!({
                                        "event": "item_skipped",
                                        "command": command_name,
                                        "index": index,
                                        "path": path,
                                    }),
                                );
                                let _ = crate::ui::json::write_event(
                                    &mut out,
                                    &serde_json::json!({
                                        "event": "progress",
                                        "command": command_name,
                                        "current": done,
                                        "total": total,
                                    }),
                                );
                            }
                            SyncEvent::ItemError {
                                index,
                                path,
                                message,
                            } => {
                                done = done.saturating_add(1);
                                let _ = crate::ui::json::write_event(
                                    &mut out,
                                    &serde_json::json!({
                                        "event": "item_error",
                                        "command": command_name,
                                        "index": index,
                                        "path": path,
                                        "message": message,
                                    }),
                                );
                                let _ = crate::ui::json::write_event(
                                    &mut out,
                                    &serde_json::json!({
                                        "event": "progress",
                                        "command": command_name,
                                        "current": done,
                                        "total": total,
                                    }),
                                );
                            }
                        }
                    };

                    sync_with_fs_with_callback(&dist_root, &outputs, &options, &fs, &mut callback)?
                } else if live_sync_ui {
                    use crate::ui::live_region::LiveRegion;
                    use crate::ui::animation::progress::{ProgressBar, ProgressStyle};
                    use crate::ui::components::stream::{ItemStatus, StreamOutput};

                    let mut region = LiveRegion::new();
                    let mut list = StreamOutput::with_visible_count(8);
                    for output in &outputs {
                        list.add(output.path.display().to_string());
                    }
                    let initial_anchor = outputs.len().min(8).saturating_sub(1);
                    list.set_anchor(initial_anchor);

                    let mut bar = ProgressBar::new(outputs.len() as u64);
                    bar.set_message("Uploading");
                    if ui.unicode {
                        bar.set_style(ProgressStyle::Blocks);
                    }
                    let width = ui.caps.width.saturating_sub(32).clamp(10, 40);
                    bar.set_width(width);

                    let mut done: u64 = 0;
                    let mut stdout = std::io::stdout().lock();

                    let initial = format!("{}{}", list.render(ui.color, ui.unicode), bar.render(ui.unicode));
                    region.update(&mut stdout, &initial)?;

                    let mut callback = |event: SyncEvent| {
                        match event {
                            SyncEvent::ItemStart { index, .. } => list.update(index, ItemStatus::InProgress),
                            SyncEvent::ItemWritten { index, .. } => {
                                list.update(index, ItemStatus::Success);
                                done = done.saturating_add(1);
                                bar.set(done);
                            }
                            SyncEvent::ItemSkipped { index, .. } => {
                                list.update(index, ItemStatus::Warning);
                                done = done.saturating_add(1);
                                bar.set(done);
                            }
                            SyncEvent::ItemError { index, message, .. } => {
                                list.update(index, ItemStatus::Error);
                                list.update_detail(index, message);
                                done = done.saturating_add(1);
                                bar.set(done);
                            }
                        }
                        let content = format!("{}{}", list.render(ui.color, ui.unicode), bar.render(ui.unicode));
                        let _ = region.update(&mut stdout, &content);
                    };

                    let result =
                        sync_with_fs_with_callback(&dist_root, &outputs, &options, &fs, &mut callback)?;
                    region.clear(&mut stdout)?;
                    result
                } else {
                    sync_with_fs(&dist_root, &outputs, &options, &fs)?
                }
            }
        }
    };

    if json {
        let mut out = std::io::stdout().lock();
        let _ = crate::ui::json::write_event(
            &mut out,
            &serde_json::json!({
                "event": "complete",
                "command": command_name,
                "status": if result.is_success() { "success" } else { "partial" },
                "written": result.written.len(),
                "skipped": result.skipped.len(),
                "errors": result.errors.len(),
            }),
        );
    } else {
        print!(
            "{}",
            crate::ui::views::deploy::render_deploy_summary(
                action,
                assets.len(),
                target_count,
                &result,
                ui.color,
                ui.unicode
            )
        );
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
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
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
        verbose,
        color,
        no_animation,
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
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
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
        verbose,
        color,
        no_animation,
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
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
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
        verbose,
        color,
        no_animation,
    )
}
