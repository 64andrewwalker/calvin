//! Calvin CLI - PromptOps compiler and synchronization tool
//!
//! Usage: calvin <COMMAND>

use anyhow::Result;
use clap::Parser;

mod commands;
mod ui;

use calvin::presentation::{Cli, ColorWhen, Commands};

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
    color: Option<ColorWhen>,
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
        Commands::Watch { source, home } => {
            commands::watch::cmd_watch(&source, home, json, color, no_animation)
        }
        Commands::Diff { source, home } => commands::debug::cmd_diff(&source, home, json),
        Commands::Parse { source } => commands::debug::cmd_parse(&source, json),
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
        Commands::Init {
            path,
            template,
            force,
        } => commands::init::cmd_init(&path, &template, force, json, verbose, color, no_animation),
    }
}

fn is_interactive_run(json: bool, yes: bool) -> bool {
    use is_terminal::IsTerminal;
    !json && !yes && std::io::stdin().is_terminal()
}
