//! Interactive command module
//!
//! Provides an interactive CLI experience for first-time and existing users.

mod menu;
mod state;
#[cfg(test)]
mod tests;
mod wizard;

use std::path::Path;

use anyhow::Result;
use is_terminal::IsTerminal;

use calvin::presentation::ColorWhen;
use state::{detect_state, ProjectState};

pub fn cmd_interactive(
    cwd: &Path,
    json: bool,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    let state = detect_state(cwd);
    let config = calvin::config::Config::load_or_default(Some(cwd));
    let ui = crate::ui::context::UiContext::new(json, verbose, color, no_animation, &config);

    if json {
        let output = match state {
            ProjectState::NoPromptPack => serde_json::json!({
                "event": "interactive",
                "state": "no_promptpack",
            }),
            ProjectState::EmptyPromptPack => serde_json::json!({
                "event": "interactive",
                "state": "empty_promptpack",
            }),
            ProjectState::Configured(count) => serde_json::json!({
                "event": "interactive",
                "state": "configured",
                "assets": { "total": count.total }
            }),
        };
        crate::ui::json::emit(output)?;
        return Ok(());
    }

    if !std::io::stdin().is_terminal() {
        println!("No command provided.");
        println!("Try: `calvin deploy` or `calvin --help`");
        return Ok(());
    }

    print_banner(&ui);

    match state {
        ProjectState::NoPromptPack => menu::interactive_first_run(cwd, &ui, verbose),
        ProjectState::EmptyPromptPack => {
            menu::interactive_existing_project(cwd, None, &ui, verbose, color, no_animation)
        }
        ProjectState::Configured(count) => menu::interactive_existing_project(
            cwd,
            Some(count.total),
            &ui,
            verbose,
            color,
            no_animation,
        ),
    }
}

fn print_banner(ui: &crate::ui::context::UiContext) {
    print!(
        "{}",
        crate::ui::views::interactive::render_banner(ui.color, ui.unicode)
    );
}

pub(crate) fn print_learn(ui: &crate::ui::context::UiContext) {
    print!(
        "{}",
        crate::ui::views::interactive::render_section_header(
            "The Problem Calvin Solves",
            ui.color,
            ui.unicode
        )
    );
    println!();
    println!("You use AI coding assistants (Claude, Cursor, Copilot...).");
    println!("Each one stores rules/commands in different locations.");
    println!("Maintaining them separately is tedious and error-prone.\n");
    print!(
        "{}",
        crate::ui::views::interactive::render_section_header("The Solution", ui.color, ui.unicode)
    );
    println!();
    println!("With Calvin, you write once in `.promptpack/`, then deploy everywhere:");
    println!("  `calvin deploy`\n");
}

pub(crate) fn print_commands() {
    println!("Commands:");
    println!("  calvin deploy            Deploy to this project");
    println!("  calvin deploy --home     Deploy to home directory");
    println!("  calvin deploy --remote   Deploy to remote destination");
    println!("  calvin check             Validate configuration and security");
    println!("  calvin watch             Watch and deploy on changes");
    println!("  calvin diff              Preview changes");
    println!("  calvin explain           Explain Calvin usage\n");
}
