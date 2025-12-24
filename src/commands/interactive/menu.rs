//! Interactive menu selections

use std::path::Path;

use anyhow::Result;
use dialoguer::{Confirm, Input, Select};

use crate::commands;
use calvin::presentation::ColorWhen;

use super::wizard;

pub fn interactive_first_run(
    cwd: &Path,
    ui: &crate::ui::context::UiContext,
    verbose: u8,
) -> Result<()> {
    println!("No .promptpack/ directory found.\n");

    let items = vec![
        "[1] Set up Calvin for this project",
        "[2] Learn what Calvin does first",
        "[3] Show commands (for experts)",
        "[4] Explain yourself (for AI assistants)",
        "[5] Quit",
    ];

    let selection = Select::new()
        .with_prompt("What would you like to do?")
        .items(&items)
        .default(0)
        .interact()?;

    match selection {
        0 => wizard::setup_wizard(cwd, ui),
        1 => {
            super::print_learn(ui);
            if Confirm::new()
                .with_prompt("Ready to set up Calvin for this project?")
                .default(true)
                .interact()?
            {
                wizard::setup_wizard(cwd, ui)?;
            }
            Ok(())
        }
        2 => {
            super::print_commands();
            Ok(())
        }
        3 => commands::explain::cmd_explain(false, false, verbose),
        _ => Ok(()),
    }
}

pub fn interactive_existing_project(
    cwd: &Path,
    asset_count: Option<usize>,
    _ui: &crate::ui::context::UiContext,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    if let Some(n) = asset_count {
        println!("Found .promptpack/ with {} prompts\n", n);
    } else {
        println!("Found .promptpack/\n");
    }

    let items = vec![
        "[1] Deploy to this project",
        "[2] Deploy to home directory",
        "[3] Deploy to project + home (by scope)",
        "[4] Deploy to remote server",
        "[5] Preview changes",
        "[6] Watch mode",
        "[7] Check configuration",
        "[8] Clean deployed files",
        "[9] Explain yourself",
        "[0] Quit",
    ];

    let selection = Select::new()
        .with_prompt("What would you like to do?")
        .items(&items)
        .default(0)
        .interact()?;

    let source = cwd.join(".promptpack");

    match selection {
        0 => commands::deploy::cmd_deploy_with_explicit_target(
            &source,
            false,
            true, // explicit_project: user explicitly chose "Deploy to this project"
            None,
            &None,
            &[],
            false,
            false,
            false,
            true,
            false,
            false, // cleanup - interactive mode handles it separately
            false,
            verbose,
            color,
            no_animation,
        ),
        1 => commands::deploy::cmd_deploy_with_explicit_target(
            &source,
            true,
            false, // explicit_project: N/A, home is true
            None,
            &None,
            &[],
            false,
            false,
            false,
            true,
            false,
            false, // cleanup
            false,
            verbose,
            color,
            no_animation,
        ),
        2 => deploy_both(&source, verbose, color, no_animation),
        3 => {
            let remote: String = Input::new()
                .with_prompt("Remote destination (user@host:/path)")
                .interact_text()?;
            commands::deploy::cmd_deploy(
                &source,
                false,
                Some(remote),
                &None,
                &[],
                false,
                false,
                false,
                true,
                false,
                false, // cleanup
                false,
                verbose,
                color,
                no_animation,
            )
        }
        4 => commands::debug::cmd_diff(&source, false, false),
        5 => commands::watch::cmd_watch(&source, false, false, color, no_animation),
        6 => commands::check::cmd_check("balanced", false, false, verbose, color, no_animation),
        7 => commands::clean::cmd_clean(
            &source,
            false, // home
            false, // project
            false, // all
            false, // dry_run
            false, // yes - let interactive mode handle
            false, // force
            false, // json
            verbose,
            color,
            no_animation,
        ),
        8 => commands::explain::cmd_explain(false, false, verbose),
        _ => Ok(()),
    }
}

fn deploy_both(
    source: &Path,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    use calvin::application::DeployOptions as UseCaseOptions;
    use calvin::domain::value_objects::Scope;
    use calvin::presentation::factory::create_deploy_use_case;

    use crate::ui::views::deploy::{render_deploy_header, render_deploy_summary};

    let config = calvin::config::Config::load_or_default(Some(source));
    let ui = crate::ui::context::UiContext::new(false, verbose, color, no_animation, &config);

    // One header, two deploy phases.
    print!(
        "{}",
        render_deploy_header(
            "Deploy (Both)",
            source,
            Some("Project + Home"),
            None,
            &[String::from("Interactive")],
            ui.color,
            ui.unicode,
        )
    );

    // Deploy to Project scope
    let use_case = create_deploy_use_case();
    let project_options = UseCaseOptions::new(source)
        .with_scope(Scope::Project)
        .with_force(false)
        .with_interactive(true);
    let result_project = use_case.execute(&project_options);
    let asset_count_project = result_project.asset_count;

    print!(
        "{}",
        render_deploy_summary(
            "Deploy (Project)",
            asset_count_project,
            5,
            &result_project,
            ui.color,
            ui.unicode,
        )
    );

    // Deploy to User (Home) scope
    let home_options = UseCaseOptions::new(source)
        .with_scope(Scope::User)
        .with_force(false)
        .with_interactive(true);
    let result_home = use_case.execute(&home_options);
    let asset_count_home = result_home.asset_count;

    print!(
        "{}",
        render_deploy_summary(
            "Deploy (Home)",
            asset_count_home,
            5,
            &result_home,
            ui.color,
            ui.unicode,
        )
    );

    Ok(())
}
