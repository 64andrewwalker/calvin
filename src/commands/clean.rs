//! Clean command handler
//!
//! Removes deployed files tracked in the lockfile.

use std::path::{Path, PathBuf};

use anyhow::Result;
use is_terminal::IsTerminal;

use calvin::application::clean::{CleanOptions, CleanResult, CleanUseCase};
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
    let scope = match (home, project) {
        (true, false) => Some(Scope::User),
        (false, true) => Some(Scope::Project),
        (false, false) => None, // All scopes or interactive
        (true, true) => unreachable!("clap conflicts_with should prevent this"),
    };

    // Build use case dependencies
    let lockfile_repo = TomlLockfileRepository::new();
    let fs = LocalFs::new();

    // Get lockfile path
    let lockfile_path = source.join(".calvin.lock");

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
    // Interactive mode: no scope specified, not json, is a TTY, not --yes
    let is_interactive =
        scope.is_none() && !json && std::io::stdin().is_terminal() && !yes && !dry_run;

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

    // Output results
    if json {
        output_json(&result, scope);
    } else {
        output_interactive(
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
        println!(
            r#"{{"type":"file_deleted","path":"{}"}}"#,
            deleted.path.display()
        );
    }
    for skipped in &result.skipped {
        println!(
            r#"{{"type":"file_skipped","path":"{}","reason":"{}"}}"#,
            skipped.path.display(),
            skipped.reason
        );
    }
    println!(
        r#"{{"type":"clean_complete","deleted":{},"skipped":{}}}"#,
        result.deleted.len(),
        result.skipped.len()
    );
}

/// Output interactive results
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
) where
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
            return;
        }

        // Actually execute the delete
        let result = use_case.execute_confirmed(lockfile_path, options);
        print!(
            "{}",
            render_clean_result(
                &result,
                false,
                ui.caps.supports_color,
                ui.caps.supports_unicode
            )
        );
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
        } else {
            // Execute confirmed
            let result = use_case.execute_confirmed(lockfile_path, options);
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
    }
}
