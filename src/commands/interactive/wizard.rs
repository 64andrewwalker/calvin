//! Setup wizard for new projects

use std::path::Path;

use anyhow::Result;
use dialoguer::{MultiSelect, Select};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TemplateChoice {
    Review,
    Test,
    Refactor,
    Docs,
    Empty,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum SecurityChoice {
    Balanced,
    Strict,
    Minimal,
}

pub fn setup_wizard(cwd: &Path, ui: &crate::ui::context::UiContext) -> Result<()> {
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

fn select_templates(ui: &crate::ui::context::UiContext) -> Result<Vec<TemplateChoice>> {
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

pub(crate) fn write_promptpack(
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

pub(crate) fn write_config(
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
