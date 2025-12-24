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
use calvin::application::resolve_lockfile_path;
use calvin::domain::entities::Lockfile;
use calvin::domain::ports::{FileSystem, LockfileRepository};
use calvin::domain::value_objects::Scope;
use calvin::infrastructure::{LocalFs, TomlLockfileRepository};
use calvin::presentation::ColorWhen;

use crate::ui::context::UiContext;
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
    // Load config for UI context
    let config = calvin::config::Config::load_or_default(Some(source));

    // Build UI context for proper icon rendering
    let ui = UiContext::new(json, verbose, color, no_animation, &config);

    // Determine scope
    // --all means clean both home and project (no scope filter)
    // --home means clean only home
    // --project means clean only project
    // None of the above: interactive mode (unless --yes or --dry-run)
    let scope = match (home, project, all) {
        (true, false, false) => Some(Scope::User),
        (false, true, false) => Some(Scope::Project),
        (false, false, true) => None,  // All scopes, non-interactive
        (false, false, false) => None, // Interactive mode
        _ => unreachable!("clap conflicts_with should prevent this"),
    };

    // Determine if a scope was explicitly specified (for interactive mode detection)
    let scope_specified = home || project || all;

    // Build use case dependencies
    let lockfile_repo = TomlLockfileRepository::new();
    let fs = LocalFs::new();

    // Get lockfile path (project-root lockfile, with legacy migration)
    let project_root = source
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let (lockfile_path, migration_note) =
        resolve_lockfile_path(project_root, source, &lockfile_repo);
    if !json {
        if let Some(note) = migration_note {
            eprintln!("ℹ {}", note);
        }
    }

    // Check if lockfile exists
    if !fs.exists(&lockfile_path) {
        if json {
            println!(
                r#"{{"type":"clean_complete","deleted":0,"skipped":0,"message":"No lockfile found"}}"#
            );
        } else {
            eprintln!(
                "No deployments found. Lockfile does not exist: {}",
                lockfile_path.display()
            );
            eprintln!("Run `calvin deploy` first to create a lockfile.");
        }
        return Ok(());
    }

    // Load lockfile
    let lockfile = lockfile_repo.load_or_new(&lockfile_path);

    if lockfile.is_empty() {
        if json {
            println!(
                r#"{{"type":"clean_complete","deleted":0,"skipped":0,"message":"No files to clean"}}"#
            );
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
    println!(
        r#"{{"type":"clean_start","scope":"{}","file_count":{}}}"#,
        scope
            .map(|s| format!("{:?}", s))
            .unwrap_or_else(|| "all".to_string()),
        result.total_count()
    );
    for deleted in &result.deleted {
        // Escape path for JSON
        let path_str = deleted.path.display().to_string().replace('\\', "\\\\");
        let key_str = deleted.key.replace('\\', "\\\\").replace('"', "\\\"");
        println!(
            r#"{{"type":"file_deleted","path":"{}","key":"{}"}}"#,
            path_str, key_str
        );
    }
    for skipped in &result.skipped {
        // Escape path for JSON
        let path_str = skipped.path.display().to_string().replace('\\', "\\\\");
        let key_str = skipped.key.replace('\\', "\\\\").replace('"', "\\\"");
        println!(
            r#"{{"type":"file_skipped","path":"{}","key":"{}","reason":"{}"}}"#,
            path_str, key_str, skipped.reason
        );
    }
    println!(
        r#"{{"type":"clean_complete","deleted":{},"skipped":{},"errors":{}}}"#,
        result.deleted.len(),
        result.skipped.len(),
        result.error_count()
    );
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
