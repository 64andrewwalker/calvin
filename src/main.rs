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
    let color = cli.color;
    let no_animation = cli.no_animation;

    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(err) => {
            let err = anyhow::Error::from(err);
            ui::error::print_error(&err, json);
            std::process::exit(1);
        }
    };

    let result = match cli.command {
        None => commands::interactive::cmd_interactive(&cwd, json, verbose, color, no_animation),
        Some(command) => dispatch(command, json, verbose, color, no_animation),
    };

    if let Err(err) = result {
        ui::error::print_error(&err, json);
        ui::error::offer_open_in_editor(&err, json);
        std::process::exit(1);
    }
}

fn dispatch(
    command: Commands,
    json: bool,
    verbose: u8,
    color: Option<cli::ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    match command {
        Commands::Deploy {
            source,
            home,
            remote,
            force,
            yes,
            dry_run,
            cleanup,
            targets,
        } => commands::deploy::cmd_deploy(
            &source,
            home,
            remote,
            &targets,
            force || yes,
            is_interactive_run(json, yes),
            dry_run,
            cleanup,
            json,
            verbose,
            color,
            no_animation,
        ),
        Commands::Check {
            mode,
            strict_warnings,
        } => commands::check::cmd_check(&mode, strict_warnings, json, verbose, color, no_animation),
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
                force || yes,
                is_interactive_run(json, yes),
                dry_run,
                json,
                verbose,
                color,
                no_animation,
            )
        }
        Commands::Watch { source, home } => {
            commands::watch::cmd_watch(&source, home, json, color, no_animation)
        }
        Commands::Diff { source, home } => commands::debug::cmd_diff(&source, home, json),
        Commands::Doctor { mode } => {
            if !json {
                eprintln!("[WARN] `calvin doctor` is deprecated; use `calvin check`.");
            }
            commands::check::cmd_doctor(&mode, json, verbose, color, no_animation)
        }
        Commands::Audit {
            mode,
            strict_warnings,
        } => {
            if !json {
                eprintln!("[WARN] `calvin audit` is deprecated; use `calvin check`.");
            }
            commands::check::cmd_audit(&mode, strict_warnings, json, verbose, color, no_animation)
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
                force || yes,
                is_interactive_run(json, yes),
                dry_run,
                json,
                verbose,
                color,
                no_animation,
            )
        }
        Commands::Migrate {
            format,
            adapter,
            dry_run,
        } => commands::debug::cmd_migrate(
            format,
            adapter,
            dry_run,
            json,
            verbose,
            color,
            no_animation,
        ),
        Commands::Version => commands::debug::cmd_version(json, verbose, color, no_animation),
    }
}

fn is_interactive_run(json: bool, yes: bool) -> bool {
    use is_terminal::IsTerminal;
    !json && !yes && std::io::stdin().is_terminal()
}
