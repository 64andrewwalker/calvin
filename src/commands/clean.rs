//! Clean command handler
//!
//! Removes deployed files tracked in the lockfile.
//!
//! # Size Justification
//!
//! calvin-no-split: This command keeps CLI flow, UI rendering, and interactive selection logic in
//! one place for now. Refactor/split after the multi-layer migration stabilizes.

use std::path::{Path, PathBuf};

use anyhow::Result;
use is_terminal::IsTerminal;

use calvin::application::clean::{CleanOptions, CleanResult, CleanUseCase};
use calvin::application::global_lockfile_path;
use calvin::application::resolve_lockfile_path;
use calvin::domain::entities::Lockfile;
use calvin::domain::ports::{FileSystem, LockfileRepository};
use calvin::domain::value_objects::Scope;
use calvin::infrastructure::{LocalFs, TomlLockfileRepository};
use calvin::presentation::ColorWhen;

use crate::ui::context::UiContext;
use crate::ui::json::emit_event;
use crate::ui::json::events::{
    CleanAllCompleteEvent, CleanAllStartEvent, CleanCompleteEvent, CleanErrorEvent,
    CleanStartEvent, FileDeletedEvent, FileSkippedEvent, ProjectCompleteEvent, ProjectErrorEvent,
    ProjectSkippedEvent,
};
use crate::ui::views::clean::{render_clean_header, render_clean_preview, render_clean_result};
use crate::ui::widgets::tree_menu::{build_tree_from_lockfile, run_interactive, TreeMenu};

/// Execute the clean command
#[allow(clippy::too_many_arguments)]
pub fn cmd_clean(
    source: &Path,
    home: bool,
    project: bool,
    all: bool,
    dry_run: bool,
    yes: bool,
    force: bool,
    json: bool,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    if all {
        return cmd_clean_all(dry_run, yes, force, json, verbose, color, no_animation);
    }

    let project_root = std::env::current_dir()?;

    // Load config for UI context
    let config = calvin::config::Config::load_or_default(Some(&project_root));

    // Build UI context for proper icon rendering
    let ui = UiContext::new(json, verbose, color, no_animation, &config);

    // Determine scope
    // --home means clean only home
    // --project means clean only project
    // None of the above: interactive mode (unless --yes or --dry-run)
    let scope = match (home, project) {
        (true, false) => Some(Scope::User),
        (false, true) => Some(Scope::Project),
        (false, false) => None, // Interactive mode
        _ => unreachable!("clap conflicts_with should prevent this"),
    };

    // Determine if a scope was explicitly specified (for interactive mode detection)
    let scope_specified = home || project;

    // Build use case dependencies
    let lockfile_repo = TomlLockfileRepository::new();
    let fs = LocalFs::new();

    // Precompute lockfile paths.
    // - Project deployments: `{cwd}/calvin.lock` (with legacy migration from `.promptpack/.calvin.lock`)
    // - Home deployments: `{HOME}/.calvin/calvin.lock` (global)
    let global_lockfile_path = global_lockfile_path();
    let (project_lockfile_path, project_migration_note) =
        resolve_lockfile_path(&project_root, source, &lockfile_repo);

    // Interactive tree menu mode: allow choosing between project/home when both exist.
    //
    // This is important for multi-layer scenarios where a directory may have no `.promptpack/`,
    // but still has project deployments (via `calvin.lock`) and/or global home deployments.
    let use_tree_menu =
        !scope_specified && !json && std::io::stdin().is_terminal() && !yes && !dry_run;

    if use_tree_menu {
        let project_exists = fs.exists(&project_lockfile_path);
        let home_exists = global_lockfile_path
            .as_ref()
            .is_some_and(|path| fs.exists(path));

        let selection = match (project_exists, home_exists) {
            (false, false) => {
                eprintln!(
                    "No deployments found.\n  - Project lockfile: {}\n  - Home lockfile: {}",
                    project_lockfile_path.display(),
                    global_lockfile_path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "<unavailable>".to_string())
                );
                return Ok(());
            }
            (true, false) => 0, // project
            (false, true) => 1, // home
            (true, true) => {
                use dialoguer::Select;
                let items = vec!["Project (./) deployments", "Home (~) deployments", "Cancel"];

                Select::new()
                    .with_prompt("What would you like to clean?")
                    .items(&items)
                    .default(0)
                    .interact()?
            }
        };

        match selection {
            0 => {
                if !json {
                    if let Some(note) = project_migration_note {
                        eprintln!("ℹ {}", note);
                    }
                    print!(
                        "{}",
                        render_clean_header(
                            source,
                            Some(Scope::Project),
                            false,
                            ui.caps.supports_color,
                            ui.caps.supports_unicode
                        )
                    );
                }

                let lockfile = lockfile_repo.load(&project_lockfile_path)?;
                if lockfile.is_empty() {
                    println!("No files to clean. Lockfile is empty.");
                    return Ok(());
                }

                return run_interactive_clean(&lockfile, &project_lockfile_path, force, &fs, &ui);
            }
            1 => {
                let Some(home_lockfile_path) = global_lockfile_path else {
                    anyhow::bail!("Failed to resolve home directory for global lockfile");
                };

                if !json {
                    print!(
                        "{}",
                        render_clean_header(
                            source,
                            Some(Scope::User),
                            false,
                            ui.caps.supports_color,
                            ui.caps.supports_unicode
                        )
                    );
                }

                let lockfile = lockfile_repo.load(&home_lockfile_path)?;
                if lockfile.is_empty() {
                    println!("No files to clean. Lockfile is empty.");
                    return Ok(());
                }

                return run_interactive_clean(&lockfile, &home_lockfile_path, force, &fs, &ui);
            }
            _ => {
                println!("Aborted.");
                return Ok(());
            }
        }
    }

    // Non-tree-menu mode: pick lockfile path based on explicit scope.
    let (lockfile_path, migration_note) = if scope == Some(Scope::User) {
        let Some(path) = global_lockfile_path.as_ref() else {
            anyhow::bail!("Failed to resolve home directory for global lockfile");
        };
        (path.clone(), None)
    } else {
        (project_lockfile_path, project_migration_note)
    };
    if !json {
        if let Some(note) = migration_note {
            eprintln!("ℹ {}", note);
        }
    }

    // Check if lockfile exists
    if !fs.exists(&lockfile_path) {
        if json {
            emit_event(&CleanCompleteEvent::new(0, 0, 0).with_message("No lockfile found"))?;
        } else {
            match scope {
                Some(Scope::User) => {
                    eprintln!(
                        "No home deployments found. Lockfile does not exist: {}",
                        lockfile_path.display()
                    );
                    eprintln!("Run `calvin deploy --home` first to create a lockfile.");
                }
                Some(Scope::Project) => {
                    eprintln!(
                        "No project deployments found. Lockfile does not exist: {}",
                        lockfile_path.display()
                    );
                    eprintln!("Run `calvin deploy` first to create a lockfile.");
                }
                None => {
                    let home_exists = global_lockfile_path
                        .as_ref()
                        .is_some_and(|path| fs.exists(path));
                    eprintln!(
                        "No project deployments found. Lockfile does not exist: {}",
                        lockfile_path.display()
                    );
                    if home_exists {
                        eprintln!(
                            "Home deployments exist. Run `calvin clean --home` to clean them."
                        );
                    } else {
                        eprintln!("Run `calvin deploy` first to create a lockfile.");
                    }
                }
            }
        }
        return Ok(());
    }

    // Load lockfile
    let lockfile = match lockfile_repo.load(&lockfile_path) {
        Ok(lockfile) => lockfile,
        Err(e) => {
            if json {
                emit_event(&CleanErrorEvent::lockfile_error(e.to_string()))?;
                return Ok(());
            }
            return Err(e.into());
        }
    };

    if lockfile.is_empty() {
        if json {
            emit_event(&CleanCompleteEvent::new(0, 0, 0).with_message("No files to clean"))?;
        } else {
            println!("No files to clean. Lockfile is empty.");
        }
        return Ok(());
    }

    // Check if we should use interactive mode
    // Interactive mode: no scope explicitly specified, not json, is a TTY, not --yes
    let is_interactive =
        !scope_specified && !json && std::io::stdin().is_terminal() && !yes && !dry_run;

    if is_interactive {
        return run_interactive_clean(&lockfile, &lockfile_path, force, &fs, &ui);
    }

    // Non-interactive mode
    let options = CleanOptions::new()
        .with_scope(scope)
        .with_dry_run(dry_run)
        .with_force(force);

    // Create use case and execute
    let use_case = CleanUseCase::new(lockfile_repo, fs);
    let result = use_case.execute(&lockfile_path, &options);

    // Output results and track if we had errors
    let has_errors = if json {
        output_json(&result, scope);
        !result.is_success()
    } else {
        let confirmed_result = output_interactive(
            &result,
            source,
            scope,
            dry_run,
            yes,
            &lockfile_path,
            &options,
            &use_case,
            &ui,
        );
        confirmed_result.map(|r| !r.is_success()).unwrap_or(false)
    };

    // Exit with code 1 if there were errors
    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}

fn cmd_clean_all(
    dry_run: bool,
    yes: bool,
    force: bool,
    json: bool,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    use crate::ui::blocks::header::CommandHeader;
    use crate::ui::blocks::summary::ResultSummary;
    use crate::ui::primitives::icon::Icon;
    use crate::ui::primitives::text::ColoredText;
    use crate::ui::widgets::list::{ItemStatus, StatusList};
    use calvin::domain::ports::RegistryError;
    use calvin::presentation::factory::create_registry_use_case;

    let cwd = std::env::current_dir()?;
    let config = calvin::config::Config::load_or_default(Some(&cwd));
    let ui = UiContext::new(json, verbose, color, no_animation, &config);

    let registry = create_registry_use_case();
    let projects = registry.list_projects().map_err(|e| match e {
        RegistryError::Corrupted { path, .. } => {
            anyhow::Error::new(calvin::CalvinError::RegistryCorrupted { path })
        }
        _ => anyhow::Error::new(e),
    })?;

    if projects.is_empty() {
        if json {
            emit_event(&CleanAllCompleteEvent::empty())?;
        } else {
            println!(
                "{} {}",
                Icon::Pending.colored(ui.caps.supports_color, ui.caps.supports_unicode),
                ColoredText::dim("No projects in registry.").render(ui.caps.supports_color)
            );
            println!(
                "{}",
                ColoredText::dim("Run `calvin deploy` in a project to register it.")
                    .render(ui.caps.supports_color)
            );
        }
        return Ok(());
    }

    if json {
        emit_event(&CleanAllStartEvent::new(projects.len()))?;
    } else {
        let mut header = CommandHeader::new(Icon::Trash, "Calvin Clean All");
        header.add("Projects", projects.len().to_string());
        print!(
            "{}",
            header.render(ui.caps.supports_color, ui.caps.supports_unicode)
        );
        println!();
        println!(
            "{}",
            ColoredText::dim("Will clean these projects:").render(ui.caps.supports_color)
        );
        for project in &projects {
            println!(
                "  {} {}",
                Icon::Pending.colored(ui.caps.supports_color, ui.caps.supports_unicode),
                project.path.display()
            );
        }
        println!();

        if !yes {
            use dialoguer::Confirm;
            let confirmed = Confirm::new()
                .with_prompt("Clean all projects?")
                .default(false)
                .interact()?;
            if !confirmed {
                println!(
                    "{} {}",
                    Icon::Warning.colored(ui.caps.supports_color, ui.caps.supports_unicode),
                    ColoredText::warning("Aborted.").render(ui.caps.supports_color)
                );
                return Ok(());
            }
        }
    }

    let lockfile_repo = TomlLockfileRepository::new();
    let fs = LocalFs::new();
    let use_case = CleanUseCase::new(lockfile_repo, fs);

    let options = CleanOptions::new()
        .with_scope(None)
        .with_dry_run(dry_run)
        .with_force(force);

    let original_cwd = std::env::current_dir()?;

    let mut list = StatusList::with_visible_count(10);
    for project in &projects {
        list.add(project.path.display().to_string());
    }

    let mut total_deleted = 0usize;
    let mut error_count = 0usize;

    for (idx, project) in projects.iter().enumerate() {
        if !json {
            list.update(idx, ItemStatus::InProgress);
            print!(
                "{}",
                list.render(ui.caps.supports_color, ui.caps.supports_unicode)
            );
        }

        if !project.lockfile.exists() {
            error_count += 1;
            if json {
                emit_event(&ProjectSkippedEvent::missing_lockfile(
                    project.path.display().to_string(),
                ))?;
            } else {
                list.update(idx, ItemStatus::Warning);
                list.update_detail(idx, "missing lockfile".to_string());
            }
            continue;
        }

        if let Err(e) = std::env::set_current_dir(&project.path) {
            error_count += 1;
            if json {
                emit_event(&ProjectErrorEvent::new(
                    project.path.display().to_string(),
                    e.to_string(),
                ))?;
            } else {
                list.update(idx, ItemStatus::Error);
                list.update_detail(idx, "chdir failed".to_string());
            }
            continue;
        }

        let result = if dry_run {
            use_case.execute(&project.lockfile, &options)
        } else {
            use_case.execute_confirmed(&project.lockfile, &options)
        };

        total_deleted += result.deleted.len();

        if result.is_success() {
            if json {
                emit_event(&ProjectCompleteEvent::new(
                    project.path.display().to_string(),
                    result.deleted.len(),
                    result.skipped.len(),
                    result.error_count(),
                ))?;
            } else {
                list.update(idx, ItemStatus::Success);
                list.update_detail(idx, format!("{} files", result.deleted.len()));
            }
        } else {
            error_count += 1;
            if json {
                emit_event(&ProjectCompleteEvent::new(
                    project.path.display().to_string(),
                    result.deleted.len(),
                    result.skipped.len(),
                    result.error_count(),
                ))?;
            } else {
                list.update(idx, ItemStatus::Error);
                list.update_detail(idx, "errors".to_string());
            }
        }

        let _ = std::env::set_current_dir(&original_cwd);
    }

    let _ = std::env::set_current_dir(&original_cwd);

    if json {
        emit_event(&CleanAllCompleteEvent::new(
            projects.len(),
            total_deleted,
            error_count,
        ))?;
        if error_count > 0 {
            std::process::exit(1);
        }
        return Ok(());
    }

    println!();
    let mut summary = if error_count == 0 {
        ResultSummary::success("Clean Complete")
    } else {
        ResultSummary::partial("Clean Completed with Errors")
    };
    summary.add_stat("projects", projects.len());
    summary.add_stat("files removed", total_deleted);
    if error_count > 0 {
        summary.add_warning(format!("{} projects had errors", error_count));
    }
    print!(
        "{}",
        summary.render(ui.caps.supports_color, ui.caps.supports_unicode)
    );

    if error_count > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// Run interactive clean with TreeMenu
fn run_interactive_clean(
    lockfile: &Lockfile,
    lockfile_path: &Path,
    force: bool,
    fs: &LocalFs,
    ui: &UiContext,
) -> Result<()> {
    use calvin::domain::services::has_calvin_signature;

    // Build tree from lockfile entries, filtering out non-cleanable files
    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    let mut skipped_count = 0;

    for (key, _entry) in lockfile.entries() {
        let path_str = if let Some((_, p)) = Lockfile::parse_key(key) {
            p.to_string()
        } else {
            continue;
        };

        // Expand home directory if needed
        let path = if path_str.starts_with("~/") {
            fs.expand_home(&PathBuf::from(&path_str))
        } else {
            PathBuf::from(&path_str)
        };

        // Filter out files that cannot be cleaned (unless force mode)
        if !force {
            // Skip missing files
            if !fs.exists(&path) {
                skipped_count += 1;
                continue;
            }

            // Skip files without Calvin signature
            if let Ok(content) = fs.read(&path) {
                if !has_calvin_signature(&content) {
                    skipped_count += 1;
                    continue;
                }
            } else {
                skipped_count += 1;
                continue;
            }
        }

        entries.push((key.to_string(), path));
    }

    if entries.is_empty() {
        if skipped_count > 0 {
            println!(
                "No cleanable files found ({} files skipped - missing or not managed by Calvin).",
                skipped_count
            );
        } else {
            println!("No files to clean.");
        }
        return Ok(());
    }

    let root = build_tree_from_lockfile(entries);
    let mut menu = TreeMenu::new(root);

    // Run the interactive menu
    match run_interactive(&mut menu, ui.caps.supports_unicode) {
        Ok(Some(selected_keys)) => {
            if selected_keys.is_empty() {
                println!("No files selected. Aborted.");
                return Ok(());
            }

            println!("\nSelected {} files for deletion.", selected_keys.len());

            // Ask for final confirmation
            use dialoguer::Confirm;
            let confirmed = Confirm::new()
                .with_prompt(format!("Delete {} files?", selected_keys.len()))
                .default(false)
                .interact()?;

            if !confirmed {
                println!("Aborted.");
                return Ok(());
            }

            // Execute deletion for selected keys only
            let lockfile_repo = TomlLockfileRepository::new();
            let fs = LocalFs::new();
            let use_case = CleanUseCase::new(lockfile_repo, fs);
            let options = CleanOptions::new()
                .with_force(force)
                .with_selected_keys(selected_keys);

            let result = use_case.execute_confirmed(lockfile_path, &options);

            print!(
                "{}",
                render_clean_result(
                    &result,
                    false,
                    ui.caps.supports_color,
                    ui.caps.supports_unicode
                )
            );
        }
        Ok(None) => {
            println!("Aborted.");
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    Ok(())
}

/// Output results as JSON
fn output_json(result: &CleanResult, scope: Option<Scope>) {
    let scope_str = scope
        .map(|s| format!("{:?}", s))
        .unwrap_or_else(|| "all".to_string());

    let _ = emit_event(&CleanStartEvent::new(&scope_str, result.total_count()));

    for deleted in &result.deleted {
        let _ = emit_event(&FileDeletedEvent::new(
            deleted.path.display().to_string(),
            deleted.key.clone(),
        ));
    }
    for skipped in &result.skipped {
        let _ = emit_event(&FileSkippedEvent::new(
            skipped.path.display().to_string(),
            skipped.key.clone(),
            skipped.reason.to_string(),
        ));
    }

    let _ = emit_event(&CleanCompleteEvent::new(
        result.deleted.len(),
        result.skipped.len(),
        result.error_count(),
    ));
}

/// Output interactive results
/// Returns the final CleanResult after confirmation/execution
#[allow(clippy::too_many_arguments)]
fn output_interactive<LR, FS>(
    result: &CleanResult,
    source: &Path,
    scope: Option<Scope>,
    dry_run: bool,
    yes: bool,
    lockfile_path: &Path,
    options: &CleanOptions,
    use_case: &CleanUseCase<LR, FS>,
    ui: &UiContext,
) -> Option<CleanResult>
where
    LR: LockfileRepository,
    FS: FileSystem,
{
    // Render header
    print!(
        "{}",
        render_clean_header(
            source,
            scope,
            dry_run,
            ui.caps.supports_color,
            ui.caps.supports_unicode
        )
    );

    // Interactive mode - ask for confirmation unless --yes
    if !dry_run && !yes && !result.deleted.is_empty() {
        // Show preview
        println!();
        print!(
            "{}",
            render_clean_preview(result, ui.caps.supports_color, ui.caps.supports_unicode)
        );
        println!();

        // Ask for confirmation
        use dialoguer::Confirm;
        let confirmed = Confirm::new()
            .with_prompt(format!("Delete {} files?", result.deleted.len()))
            .default(false)
            .interact()
            .unwrap_or(false);

        if !confirmed {
            println!("Aborted.");
            return None;
        }

        // Actually execute the delete
        let final_result = use_case.execute_confirmed(lockfile_path, options);
        print!(
            "{}",
            render_clean_result(
                &final_result,
                false,
                ui.caps.supports_color,
                ui.caps.supports_unicode
            )
        );
        Some(final_result)
    } else if dry_run {
        // Dry run mode
        println!();
        print!(
            "{}",
            render_clean_preview(result, ui.caps.supports_color, ui.caps.supports_unicode)
        );
        println!();
        print!(
            "{}",
            render_clean_result(
                result,
                true,
                ui.caps.supports_color,
                ui.caps.supports_unicode
            )
        );
        Some(result.clone())
    } else {
        // --yes mode or empty result
        if result.deleted.is_empty() && result.skipped.is_empty() {
            print!(
                "{}",
                render_clean_result(
                    result,
                    false,
                    ui.caps.supports_color,
                    ui.caps.supports_unicode
                )
            );
            Some(result.clone())
        } else {
            // Execute confirmed
            let final_result = use_case.execute_confirmed(lockfile_path, options);
            print!(
                "{}",
                render_clean_result(
                    &final_result,
                    false,
                    ui.caps.supports_color,
                    ui.caps.supports_unicode
                )
            );
            Some(final_result)
        }
    }
}
