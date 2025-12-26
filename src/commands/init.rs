//! Init command - create a new .promptpack directory
//!
//! Templates:
//! - minimal: Only config.toml and README
//! - standard: config.toml, README, one example policy and action
//! - full: All directories with comprehensive examples

use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::ui::primitives::icon::Icon;
use crate::ui::terminal::detect_capabilities;
use calvin::presentation::ColorWhen;

/// Template for init command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Template {
    Minimal,
    Standard,
    Full,
}

impl Template {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "minimal" | "min" => Some(Self::Minimal),
            "standard" | "std" => Some(Self::Standard),
            "full" => Some(Self::Full),
            _ => None,
        }
    }
}

/// Initialize a new .promptpack directory
#[allow(clippy::too_many_arguments)]
pub fn cmd_init(
    path: &Path,
    user: bool,
    template: &str,
    force: bool,
    json: bool,
    _verbose: u8,
    color: Option<ColorWhen>,
    _no_animation: bool,
) -> Result<()> {
    if user {
        return cmd_init_user(force, json, color);
    }

    let caps = detect_capabilities();
    let supports_color = match color {
        Some(ColorWhen::Always) => true,
        Some(ColorWhen::Never) => false,
        Some(ColorWhen::Auto) | None => caps.supports_color,
    };
    let supports_unicode = caps.supports_unicode;
    let promptpack_dir = path.join(".promptpack");

    // Check if already exists
    if promptpack_dir.exists() && !force {
        if json {
            let _ = crate::ui::json::emit(serde_json::json!({
                "event": "error",
                "command": "init",
                "kind": "already_exists",
                "path": promptpack_dir.display().to_string(),
                "message": ".promptpack directory already exists"
            }));
        }
        bail!(
            ".promptpack already exists at {}. Use --force to overwrite.",
            promptpack_dir.display()
        );
    }

    // Parse template
    let template = Template::from_str(template).ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid template '{}'. Valid options: minimal, standard, full",
            template
        )
    })?;

    // Create directory structure
    create_promptpack(&promptpack_dir, template)?;

    // Output success
    if json {
        let _ = crate::ui::json::emit(serde_json::json!({
            "event": "complete",
            "command": "init",
            "path": promptpack_dir.display().to_string(),
            "template": format!("{:?}", template).to_lowercase(),
        }));
    } else {
        println!(
            "{} Created {} with {:?} template",
            Icon::Success.colored(supports_color, supports_unicode),
            promptpack_dir.display(),
            template
        );
        println!();
        println!(
            "{} Next: Run `calvin deploy` to compile and deploy",
            Icon::Arrow.colored(supports_color, supports_unicode)
        );
    }

    Ok(())
}

fn cmd_init_user(force: bool, json: bool, color: Option<ColorWhen>) -> Result<()> {
    let caps = detect_capabilities();
    let supports_color = match color {
        Some(ColorWhen::Always) => true,
        Some(ColorWhen::Never) => false,
        Some(ColorWhen::Auto) | None => caps.supports_color,
    };
    let supports_unicode = caps.supports_unicode;

    let user_layer = std::env::var("CALVIN_SOURCES_USER_LAYER_PATH")
        .ok()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(calvin::config::default_user_layer_path);

    if user_layer.exists() && !force {
        if json {
            let _ = crate::ui::json::emit(serde_json::json!({
                "event": "error",
                "command": "init",
                "kind": "already_exists",
                "path": user_layer.display().to_string(),
                "message": "user layer already exists"
            }));
        }
        bail!(
            "User layer already exists at {}. Use --force to overwrite.",
            user_layer.display()
        );
    }

    fs::create_dir_all(user_layer.join("policies"))?;
    fs::create_dir_all(user_layer.join("actions"))?;
    fs::create_dir_all(user_layer.join("agents"))?;

    if !user_layer.join("config.toml").exists() || force {
        fs::write(user_layer.join("config.toml"), USER_CONFIG_TEMPLATE)
            .context("Failed to create user config.toml")?;
    }

    if json {
        let _ = crate::ui::json::emit(serde_json::json!({
            "event": "complete",
            "command": "init",
            "path": user_layer.display().to_string(),
            "template": "user",
        }));
    } else {
        // PRD §12.5: Show tree structure of created directories
        // Using simple indented list format to comply with no_legacy_borders test
        println!(
            "{} User Layer Initialized",
            Icon::Success.colored(supports_color, supports_unicode)
        );
        println!();
        println!("Created: {}/", user_layer.display());
        println!("  - config.toml");
        println!("  - policies/");
        println!("  - actions/");
        println!("  - agents/");
        println!();
        println!("Next steps:");
        println!("  1. Add your global prompts to {}/", user_layer.display());
        println!("  2. Any project can now use these prompts");
        println!("  3. Project-level .promptpack/ will override if needed");
    }

    Ok(())
}

fn create_promptpack(dir: &Path, template: Template) -> Result<()> {
    // Create base directories
    fs::create_dir_all(dir).context("Failed to create .promptpack directory")?;

    // Always create config.toml and README
    fs::write(dir.join("config.toml"), CONFIG_TEMPLATE).context("Failed to create config.toml")?;
    fs::write(dir.join("README.md"), README_TEMPLATE).context("Failed to create README.md")?;

    match template {
        Template::Minimal => {
            // Nothing else for minimal
        }
        Template::Standard => {
            // Create policies and actions with one example each
            fs::create_dir_all(dir.join("policies")).context("Failed to create policies/")?;
            fs::create_dir_all(dir.join("actions")).context("Failed to create actions/")?;

            fs::write(dir.join("policies/code-style.md"), POLICY_EXAMPLE)
                .context("Failed to create example policy")?;
            fs::write(dir.join("actions/hello.md"), ACTION_EXAMPLE)
                .context("Failed to create example action")?;
        }
        Template::Full => {
            // Create all directories with examples
            for subdir in ["policies", "actions", "agents", "mcp"] {
                fs::create_dir_all(dir.join(subdir))
                    .context(format!("Failed to create {}/", subdir))?;
            }

            fs::write(dir.join("policies/code-style.md"), POLICY_EXAMPLE)
                .context("Failed to create example policy")?;
            fs::write(dir.join("policies/security.md"), SECURITY_POLICY_EXAMPLE)
                .context("Failed to create security policy")?;
            fs::write(dir.join("actions/hello.md"), ACTION_EXAMPLE)
                .context("Failed to create example action")?;
            fs::write(dir.join("actions/review.md"), REVIEW_ACTION_EXAMPLE)
                .context("Failed to create review action")?;
            fs::write(dir.join("agents/reviewer.md"), AGENT_EXAMPLE)
                .context("Failed to create example agent")?;
            fs::write(dir.join("mcp/.gitkeep"), "").context("Failed to create mcp/.gitkeep")?;
        }
    }

    Ok(())
}

// Template content strings
const USER_CONFIG_TEMPLATE: &str = r#"# Calvin User Layer Configuration
# This promptpack applies to all projects (unless disabled).

[format]
version = "1.0"
"#;

const CONFIG_TEMPLATE: &str = r#"# Calvin Configuration
# See: https://github.com/calvin-cli/calvin/docs/configuration.md

[format]
version = "1.0"

[targets]
# Specify which platforms to deploy to (default: all)
# Valid values: claude-code (or "claude"), cursor, vscode, antigravity, codex
# enabled = ["claude-code", "cursor"]

[security]
# Security mode: yolo, balanced, strict
mode = "balanced"
allow_naked = false

[sync]
atomic_writes = true
respect_lockfile = true

[output]
verbosity = "normal"

"#;

const README_TEMPLATE: &str = r#"# PromptPack

This directory contains your AI assistant rules, commands, and workflows.

## Structure

- `policies/` - Long-term rules (code style, security, etc.)
- `actions/` - Slash commands and workflows
- `agents/` - Sub-agent definitions
- `mcp/` - MCP server configurations

## Usage

```bash
# Compile and deploy to all platforms
calvin deploy

# Preview changes
calvin diff

# Watch for changes
calvin watch

# Validate configuration
calvin check
```

## Frontmatter

Each markdown file requires YAML frontmatter:

```yaml
---
id: my-policy
title: My Policy
kind: policy  # policy, action, or agent
scope: project  # project or user
targets: all  # all, claude, cursor, vscode, antigravity, codex
---
```

See [Calvin documentation](https://github.com/calvin-cli/calvin) for more.
"#;

const POLICY_EXAMPLE: &str = r#"---
id: code-style
title: Code Style Guidelines
kind: policy
scope: project
targets: all
---

# Code Style

Follow these coding standards:

1. Use meaningful variable names
2. Keep functions small and focused
3. Write comments for complex logic
4. Format code consistently
"#;

const SECURITY_POLICY_EXAMPLE: &str = r#"---
id: security
title: Security Guidelines
kind: policy
scope: project
targets: all
---

# Security

Follow these security practices:

1. Never commit secrets or credentials
2. Validate all user input
3. Use parameterized queries for databases
4. Keep dependencies updated
"#;

const ACTION_EXAMPLE: &str = r#"---
id: hello
title: Hello World
kind: action
scope: project
targets: all
---

# Hello

This is an example slash command.

Usage: `/hello [name]`

Say hello to the user and introduce yourself as their AI assistant.
"#;

const REVIEW_ACTION_EXAMPLE: &str = r#"---
id: review
title: Code Review
kind: action
scope: project
targets: all
---

# Code Review

Review the current code for:

1. Correctness
2. Performance
3. Security vulnerabilities
4. Code style issues

Provide actionable feedback with specific line numbers.
"#;

const AGENT_EXAMPLE: &str = r#"---
id: reviewer
title: Code Reviewer
kind: agent
scope: project
targets: claude
---

# Code Reviewer Agent

You are a senior code reviewer. Your job is to:

1. Find bugs and security issues
2. Suggest improvements
3. Ensure code quality standards
4. Be constructive and helpful

Focus on the most impactful feedback first.
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn template_from_str_works() {
        assert_eq!(Template::from_str("minimal"), Some(Template::Minimal));
        assert_eq!(Template::from_str("min"), Some(Template::Minimal));
        assert_eq!(Template::from_str("standard"), Some(Template::Standard));
        assert_eq!(Template::from_str("std"), Some(Template::Standard));
        assert_eq!(Template::from_str("full"), Some(Template::Full));
        assert_eq!(Template::from_str("invalid"), None);
    }

    #[test]
    fn create_minimal_template() {
        let dir = tempdir().unwrap();
        let promptpack = dir.path().join(".promptpack");

        create_promptpack(&promptpack, Template::Minimal).unwrap();

        assert!(promptpack.join("config.toml").exists());
        assert!(promptpack.join("README.md").exists());
        assert!(!promptpack.join("policies").exists());
        assert!(!promptpack.join("actions").exists());
    }

    #[test]
    fn create_standard_template() {
        let dir = tempdir().unwrap();
        let promptpack = dir.path().join(".promptpack");

        create_promptpack(&promptpack, Template::Standard).unwrap();

        assert!(promptpack.join("config.toml").exists());
        assert!(promptpack.join("README.md").exists());
        assert!(promptpack.join("policies/code-style.md").exists());
        assert!(promptpack.join("actions/hello.md").exists());
    }

    #[test]
    fn create_full_template() {
        let dir = tempdir().unwrap();
        let promptpack = dir.path().join(".promptpack");

        create_promptpack(&promptpack, Template::Full).unwrap();

        assert!(promptpack.join("config.toml").exists());
        assert!(promptpack.join("README.md").exists());
        assert!(promptpack.join("policies/code-style.md").exists());
        assert!(promptpack.join("policies/security.md").exists());
        assert!(promptpack.join("actions/hello.md").exists());
        assert!(promptpack.join("actions/review.md").exists());
        assert!(promptpack.join("agents/reviewer.md").exists());
        assert!(promptpack.join("mcp/.gitkeep").exists());
    }

    #[test]
    fn cmd_init_creates_promptpack() {
        let dir = tempdir().unwrap();

        cmd_init(dir.path(), false, "standard", false, true, 0, None, false).unwrap();

        assert!(dir.path().join(".promptpack/config.toml").exists());
    }

    #[test]
    fn cmd_init_fails_if_exists() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".promptpack")).unwrap();

        let result = cmd_init(dir.path(), false, "standard", false, true, 0, None, false);
        assert!(result.is_err());
    }

    #[test]
    fn cmd_init_force_overwrites() {
        let dir = tempdir().unwrap();
        let promptpack = dir.path().join(".promptpack");
        fs::create_dir_all(&promptpack).unwrap();
        fs::write(promptpack.join("old-file.txt"), "old content").unwrap();

        cmd_init(dir.path(), false, "standard", true, true, 0, None, false).unwrap();

        assert!(promptpack.join("config.toml").exists());
    }
}
