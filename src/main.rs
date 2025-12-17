//! Calvin CLI - PromptOps compiler and synchronization tool
//!
//! Usage: calvin <COMMAND>

use anyhow::Result;
use clap::Parser;

mod cli;
mod commands;
mod state;
mod ui;

use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();
    let json = cli.json;
    let verbose = cli.verbose;

    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(err) => {
            let err = anyhow::Error::from(err);
            ui::error::print_error(&err, json);
            std::process::exit(1);
        }
    };

    let result = match cli.command {
        None => commands::interactive::cmd_interactive(&cwd, json, verbose),
        Some(command) => dispatch(command, json, verbose),
    };

    if let Err(err) = result {
        ui::error::print_error(&err, json);
        ui::error::offer_open_in_editor(&err, json);
        std::process::exit(1);
    }
}

fn dispatch(command: Commands, json: bool, verbose: u8) -> Result<()> {
    match command {
        Commands::Deploy {
            source,
            home,
            remote,
            force,
            yes,
            dry_run,
            targets,
        } => commands::deploy::cmd_deploy(
            &source,
            home,
            remote,
            &targets,
            force,
            !yes,
            dry_run,
            json,
            verbose,
        ),
        Commands::Check {
            mode,
            strict_warnings,
        } => commands::check::cmd_check(&mode, strict_warnings, json, verbose),
        Commands::Explain { brief } => commands::explain::cmd_explain(brief, json, verbose),
        Commands::Sync {
            source,
            remote,
            force,
            yes,
            dry_run,
            targets,
        } => {
            if !json {
                eprintln!("[WARN] `calvin sync` is deprecated; use `calvin deploy`.");
            }
            commands::deploy::cmd_sync(
                &source,
                remote,
                &targets,
                force,
                !yes,
                dry_run,
                json,
                verbose,
            )
        }
        Commands::Watch { source } => commands::watch::cmd_watch(&source, json),
        Commands::Diff { source } => commands::debug::cmd_diff(&source, json),
        Commands::Doctor { mode } => {
            if !json {
                eprintln!("[WARN] `calvin doctor` is deprecated; use `calvin check`.");
            }
            commands::check::cmd_doctor(&mode, json, verbose)
        }
        Commands::Audit {
            mode,
            strict_warnings,
        } => {
            if !json {
                eprintln!("[WARN] `calvin audit` is deprecated; use `calvin check`.");
            }
            commands::check::cmd_audit(&mode, strict_warnings, json, verbose)
        }
        Commands::Parse { source } => commands::debug::cmd_parse(&source, json),
        Commands::Install {
            source,
            user,
            global,
            targets,
            force,
            yes,
            dry_run,
        } => {
            if !json {
                eprintln!("[WARN] `calvin install` is deprecated; use `calvin deploy` or `calvin deploy --home`.");
            }
            commands::deploy::cmd_install(
                &source,
                user,
                global,
                &targets,
                force,
                !yes,
                dry_run,
                json,
            )
        }
        Commands::Migrate {
            format,
            adapter,
            dry_run,
        } => commands::debug::cmd_migrate(format, adapter, dry_run, json),
        Commands::Version => commands::debug::cmd_version(json),
    }
}
