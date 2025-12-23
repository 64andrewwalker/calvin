//! Clean command handler
//!
//! Removes deployed files tracked in the lockfile.

use std::path::Path;

use anyhow::Result;

use calvin::application::clean::{CleanOptions, CleanUseCase};
use calvin::domain::ports::{FileSystem, LockfileRepository};
use calvin::domain::value_objects::Scope;
use calvin::infrastructure::{LocalFs, TomlLockfileRepository};
use calvin::presentation::ColorWhen;

use crate::ui::context::UiContext;
use crate::ui::views::clean::{render_clean_header, render_clean_preview, render_clean_result};

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
        (false, false) => None, // All scopes
        (true, true) => unreachable!("clap conflicts_with should prevent this"),
    };

    let options = CleanOptions {
        scope,
        dry_run,
        force,
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

    // Create use case and execute
    let use_case = CleanUseCase::new(lockfile_repo, fs);
    let result = use_case.execute(&lockfile_path, &options);

    // Output results
    if json {
        println!(
            r#"{{"type":"clean_start","scope":"{}","file_count":{}}}"#,
            scope
                .map(|s| format!("{:?}", s))
                .unwrap_or_else(|| "all".to_string()),
            result.total_count()
        );
        for path in &result.deleted {
            println!(r#"{{"type":"file_deleted","path":"{}"}}"#, path.display());
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
    } else {
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
                render_clean_preview(&result, ui.caps.supports_color, ui.caps.supports_unicode)
            );
            println!();

            // Ask for confirmation
            use dialoguer::Confirm;
            let confirmed = Confirm::new()
                .with_prompt(format!("Delete {} files?", result.deleted.len()))
                .default(false)
                .interact()?;

            if !confirmed {
                println!("Aborted.");
                return Ok(());
            }

            // Actually execute the delete
            let result = use_case.execute_confirmed(&lockfile_path, &options);
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
                render_clean_preview(&result, ui.caps.supports_color, ui.caps.supports_unicode)
            );
            println!();
            print!(
                "{}",
                render_clean_result(
                    &result,
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
                        &result,
                        false,
                        ui.caps.supports_color,
                        ui.caps.supports_unicode
                    )
                );
            } else {
                // Execute confirmed
                let result = use_case.execute_confirmed(&lockfile_path, &options);
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

    Ok(())
}
