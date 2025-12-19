use std::path::Path;

use anyhow::Result;
use dialoguer::{Confirm, Input, Select};
use is_terminal::IsTerminal;

use crate::cli::ColorWhen;
use crate::commands;
use crate::state::{detect_state, ProjectState};

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
        ProjectState::NoPromptPack => interactive_first_run(cwd, &ui, verbose),
        ProjectState::EmptyPromptPack => {
            interactive_existing_project(cwd, None, &ui, verbose, color, no_animation)
        }
        ProjectState::Configured(count) => {
            interactive_existing_project(cwd, Some(count.total), &ui, verbose, color, no_animation)
        }
    }
}

fn interactive_first_run(
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
        0 => setup_wizard(cwd, ui),
        1 => {
            print_learn(ui);
            if Confirm::new()
                .with_prompt("Ready to set up Calvin for this project?")
                .default(true)
                .interact()?
            {
                setup_wizard(cwd, ui)?;
            }
            Ok(())
        }
        2 => {
            print_commands();
            Ok(())
        }
        3 => commands::explain::cmd_explain(false, false, verbose),
        _ => Ok(()),
    }
}

fn interactive_existing_project(
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
        "[8] Explain yourself",
        "[9] Quit",
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
        7 => commands::explain::cmd_explain(false, false, verbose),
        _ => Ok(()),
    }
}

fn deploy_both(
    source: &Path,
    verbose: u8,
    color: Option<ColorWhen>,
    no_animation: bool,
) -> Result<()> {
    use calvin::sync::ScopePolicy;

    use crate::commands::deploy::{
        options::DeployOptions, runner::DeployRunner, targets::DeployTarget,
    };
    use crate::ui::views::deploy::{render_deploy_header, render_deploy_summary};

    let project_root = source.parent().unwrap_or(source).to_path_buf();
    let config = calvin::config::Config::load_or_default(Some(source));
    let ui = crate::ui::context::UiContext::new(false, verbose, color, no_animation, &config);

    let mut options = DeployOptions::new();
    options.force = false;
    options.interactive = true;
    options.dry_run = false;
    options.json = false;
    options.verbose = verbose;
    options.no_animation = no_animation;

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

    let runner_project = DeployRunner::new(
        source.to_path_buf(),
        DeployTarget::Project(project_root),
        ScopePolicy::ProjectOnly,
        options.clone(),
        ui,
    );
    let result_project = runner_project.run()?;
    let asset_count_project = result_project.written.len() / 4;
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

    let runner_home = DeployRunner::new(
        source.to_path_buf(),
        DeployTarget::Home,
        ScopePolicy::UserOnly,
        options,
        ui,
    );
    let result_home = runner_home.run()?;
    let asset_count_home = result_home.written.len() / 4;
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

fn setup_wizard(cwd: &Path, ui: &crate::ui::context::UiContext) -> Result<()> {
    print!(
        "{}",
        crate::ui::views::interactive::render_setup_intro(ui.color, ui.unicode)
    );
    println!();

    let targets = select_targets()?;
    let templates = select_templates(ui)?;
    let security = select_security_mode(ui)?;

    let promptpack = cwd.join(".promptpack");
    write_promptpack(&promptpack, &targets, templates, security)?;

    println!();
    print!(
        "{}",
        crate::ui::views::interactive::render_setup_complete(ui.color, ui.unicode)
    );

    Ok(())
}

fn select_targets() -> Result<Vec<calvin::Target>> {
    let config = calvin::config::Config::default();
    crate::ui::menu::select_targets_interactive(&config, false)
        .ok_or_else(|| anyhow::anyhow!("Aborted"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TemplateChoice {
    Review,
    Test,
    Refactor,
    Docs,
    Empty,
}

fn select_templates(ui: &crate::ui::context::UiContext) -> Result<Vec<TemplateChoice>> {
    use dialoguer::MultiSelect;

    print!(
        "{}",
        crate::ui::views::interactive::render_step_header(
            2,
            3,
            "Start with example prompts?",
            ui.color,
            ui.unicode
        )
    );
    println!();

    let items = vec![
        "review.md         /review command for PR reviews",
        "test.md           /test command to write tests",
        "refactor.md       /refactor command for cleanup",
        "docs.md           /docs command for documentation",
        "(empty)           Start with blank templates",
    ];

    let selection = MultiSelect::new()
        .items(&items)
        .defaults(&[true, true, false, false, false])
        .interact()?;

    let mut choices = Vec::new();
    for idx in selection {
        let choice = match idx {
            0 => TemplateChoice::Review,
            1 => TemplateChoice::Test,
            2 => TemplateChoice::Refactor,
            3 => TemplateChoice::Docs,
            _ => TemplateChoice::Empty,
        };
        choices.push(choice);
    }

    if choices.contains(&TemplateChoice::Empty) {
        return Ok(vec![TemplateChoice::Empty]);
    }

    Ok(choices)
}

#[derive(Debug, Clone, Copy)]
enum SecurityChoice {
    Balanced,
    Strict,
    Minimal,
}

fn select_security_mode(ui: &crate::ui::context::UiContext) -> Result<SecurityChoice> {
    print!(
        "{}",
        crate::ui::views::interactive::render_step_header(
            3,
            3,
            "Security preference",
            ui.color,
            ui.unicode
        )
    );
    println!();
    println!("Calvin can protect sensitive files from being read by AI.\n");

    let items = vec![
        "Balanced          Block: .env, private keys, .git (recommended)",
        "Strict            Block everything sensitive (for CI / regulated)",
        "Minimal           I'll configure security myself",
    ];

    let selection = Select::new().items(&items).default(0).interact()?;

    Ok(match selection {
        1 => SecurityChoice::Strict,
        2 => SecurityChoice::Minimal,
        _ => SecurityChoice::Balanced,
    })
}

fn write_promptpack(
    promptpack: &Path,
    targets: &[calvin::Target],
    templates: Vec<TemplateChoice>,
    security: SecurityChoice,
) -> Result<()> {
    std::fs::create_dir_all(promptpack.join("actions"))?;

    write_config(promptpack, targets, security)?;
    write_templates(promptpack, templates)?;

    Ok(())
}

fn write_config(
    promptpack: &Path,
    targets: &[calvin::Target],
    security: SecurityChoice,
) -> Result<()> {
    let (mode, allow_naked) = match security {
        SecurityChoice::Balanced => ("balanced", false),
        SecurityChoice::Strict => ("strict", false),
        SecurityChoice::Minimal => ("balanced", true),
    };

    let enabled = targets
        .iter()
        .map(|t| format!("\"{}\"", target_kebab(*t)))
        .collect::<Vec<_>>()
        .join(", ");

    let content = format!(
        "[targets]\n\
enabled = [{enabled}]\n\
\n\
[security]\n\
mode = \"{mode}\"\n\
allow_naked = {allow_naked}\n"
    );

    write_file_if_missing(&promptpack.join("config.toml"), &content)
}

fn write_templates(promptpack: &Path, templates: Vec<TemplateChoice>) -> Result<()> {
    if templates == vec![TemplateChoice::Empty] {
        return Ok(());
    }

    for template in templates {
        let (name, content) = match template {
            TemplateChoice::Review => ("review.md", template_review()),
            TemplateChoice::Test => ("test.md", template_test()),
            TemplateChoice::Refactor => ("refactor.md", template_refactor()),
            TemplateChoice::Docs => ("docs.md", template_docs()),
            TemplateChoice::Empty => continue,
        };

        write_file_if_missing(&promptpack.join("actions").join(name), content)?;
    }

    Ok(())
}

fn template_review() -> &'static str {
    r#"---
description: PR review helper
---
Review the selected code for correctness, security issues, and improvements.
Focus on actionable feedback and highlight edge cases.
"#
}

fn template_test() -> &'static str {
    r#"---
description: Test generator
---
Write tests for the selected code.
Include edge cases and error handling, and keep tests deterministic.
"#
}

fn template_refactor() -> &'static str {
    r#"---
description: Refactor helper
---
Propose a safe refactor plan, then apply minimal changes while preserving behavior.
"#
}

fn template_docs() -> &'static str {
    r#"---
description: Documentation helper
---
Draft documentation for the selected code. Prefer examples and clear steps.
"#
}

fn write_file_if_missing(path: &Path, content: &str) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    std::fs::write(path, content)?;
    Ok(())
}

fn target_kebab(target: calvin::Target) -> &'static str {
    match target {
        calvin::Target::ClaudeCode => "claude-code",
        calvin::Target::Cursor => "cursor",
        calvin::Target::VSCode => "vscode",
        calvin::Target::Antigravity => "antigravity",
        calvin::Target::Codex => "codex",
        calvin::Target::All => "all",
    }
}

fn print_banner(ui: &crate::ui::context::UiContext) {
    print!(
        "{}",
        crate::ui::views::interactive::render_banner(ui.color, ui.unicode)
    );
}

fn print_learn(ui: &crate::ui::context::UiContext) {
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

fn print_commands() {
    println!("Commands:");
    println!("  calvin deploy            Deploy to this project");
    println!("  calvin deploy --home     Deploy to home directory");
    println!("  calvin deploy --remote   Deploy to remote destination");
    println!("  calvin check             Validate configuration and security");
    println!("  calvin watch             Watch and deploy on changes");
    println!("  calvin diff              Preview changes");
    println!("  calvin explain           Explain Calvin usage\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_write_promptpack_creates_config_and_templates() {
        let dir = tempdir().unwrap();
        let promptpack = dir.path().join(".promptpack");

        write_promptpack(
            &promptpack,
            &[calvin::Target::ClaudeCode, calvin::Target::Cursor],
            vec![TemplateChoice::Review],
            SecurityChoice::Balanced,
        )
        .unwrap();

        let config = std::fs::read_to_string(promptpack.join("config.toml")).unwrap();
        assert!(config.contains("[targets]"));
        assert!(config.contains("claude-code"));
        assert!(config.contains("cursor"));
        assert!(config.contains("mode = \"balanced\""));

        let review = std::fs::read_to_string(promptpack.join("actions/review.md")).unwrap();
        assert!(review.contains("description: PR review helper"));
    }

    #[test]
    fn test_write_promptpack_empty_templates_creates_no_action_files() {
        let dir = tempdir().unwrap();
        let promptpack = dir.path().join(".promptpack");

        write_promptpack(
            &promptpack,
            &[calvin::Target::ClaudeCode],
            vec![TemplateChoice::Empty],
            SecurityChoice::Balanced,
        )
        .unwrap();

        let entries = std::fs::read_dir(promptpack.join("actions")).unwrap();
        let count = entries.count();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_write_config_does_not_overwrite_existing_file() {
        let dir = tempdir().unwrap();
        let promptpack = dir.path().join(".promptpack");
        std::fs::create_dir_all(&promptpack).unwrap();

        let config_path = promptpack.join("config.toml");
        std::fs::write(&config_path, "sentinel\n").unwrap();

        write_config(
            &promptpack,
            &[calvin::Target::ClaudeCode],
            SecurityChoice::Strict,
        )
        .unwrap();

        let config = std::fs::read_to_string(config_path).unwrap();
        assert_eq!(config, "sentinel\n");
    }

    #[test]
    fn test_write_config_minimal_sets_allow_naked_true() {
        let dir = tempdir().unwrap();
        let promptpack = dir.path().join(".promptpack");
        std::fs::create_dir_all(&promptpack).unwrap();

        write_config(
            &promptpack,
            &[calvin::Target::ClaudeCode],
            SecurityChoice::Minimal,
        )
        .unwrap();

        let config = std::fs::read_to_string(promptpack.join("config.toml")).unwrap();
        assert!(config.contains("allow_naked = true"));
    }
}
