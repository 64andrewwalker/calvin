use anyhow::Result;

pub fn cmd_explain(brief: bool, json: bool, verbose: u8) -> Result<()> {
    if json {
        crate::ui::json::emit(serde_json::json!({
            "event": "start",
            "command": "explain",
            "brief": brief,
            "verbose": verbose
        }))?;

        let output = serde_json::json!({
            "name": "calvin",
            "version": env!("CARGO_PKG_VERSION"),
            "purpose": "PromptOps compiler for AI coding assistants",
            "directory": ".promptpack/",
            "structure": {
                "config.toml": "Configuration file",
                "actions/": "Slash commands (/review, /test)",
                "policies/": "Long-term rules (code style, security)",
                "agents/": "Sub-agent definitions",
                "mcp/": "MCP server configs (optional)"
            },
            "frontmatter": {
                "required": ["description"],
                "optional": ["scope", "targets", "apply"]
            },
            "commands": {
                "calvin deploy": "Deploy prompts to project outputs (e.g. .claude/, .cursor/)",
                "calvin deploy --home": "Deploy all prompts to home directory targets (~/...)",
                "calvin deploy --remote user@host:/path": "Deploy to a remote destination via SSH",
                "calvin check": "Validate configuration and security (non-zero exit on violations)",
                "calvin watch": "Watch and redeploy on changes",
                "calvin diff": "Preview what would change",
                "calvin version": "Show version and adapter versions"
            },
            "examples": {
                "deploy_project": "calvin deploy",
                "deploy_home": "calvin deploy --home",
                "deploy_remote": "calvin deploy --remote user@host:/path/to/project",
                "check": "calvin check --mode strict --strict-warnings"
            }
        });

        crate::ui::json::emit(serde_json::json!({
            "event": "complete",
            "command": "explain",
            "data": output
        }))?;
        return Ok(());
    }

    println!("Calvin v{}", env!("CARGO_PKG_VERSION"));
    println!("PromptOps compiler for AI coding assistants.\n");

    if brief {
        println!("KEY COMMANDS:");
        println!("  calvin deploy [--home] [--remote user@host:/path]");
        println!("  calvin check [--mode balanced|strict|yolo] [--strict-warnings]");
        println!("  calvin watch");
        println!("  calvin diff");
        println!("  calvin version");
        return Ok(());
    }

    println!("PURPOSE:");
    println!(
        "  Maintain AI prompts/rules in one place (.promptpack/), then deploy to multiple tools.\n"
    );

    println!("DIRECTORY STRUCTURE:");
    println!("  .promptpack/");
    println!("  |-- config.toml       Configuration (targets, security, defaults)");
    println!("  |-- actions/          Slash commands (/review, /test, ...)");
    println!("  |-- policies/         Long-term rules (code style, security)");
    println!("  |-- agents/           Sub-agent definitions");
    println!("  +-- mcp/              MCP server configs (optional)\n");

    println!("FRONTMATTER (YAML):");
    println!("  ---");
    println!("  description: Required. What this prompt does.");
    println!("  scope: project | user (default: project)");
    println!("  targets: [claude-code, cursor, vscode, antigravity, codex, opencode] (optional)");
    println!("  apply: \"*.rs\" (optional)");
    println!("  ---\n");

    println!("KEY COMMANDS:");
    println!("  calvin deploy");
    println!("  calvin deploy --home");
    println!("  calvin deploy --remote user@host:/path");
    println!("  calvin check");
    println!("  calvin watch");
    println!("  calvin diff");
    println!("  calvin version\n");

    if verbose > 0 {
        println!("EXAMPLES:");
        println!("  # Interactive setup (first run)");
        println!("  calvin\n");
        println!("  # Deploy to this project");
        println!("  calvin deploy\n");
        println!("  # Deploy only to specific targets");
        println!("  calvin deploy --targets claude-code,cursor\n");
        println!("  # Non-interactive deploy (auto-confirm overwrites)");
        println!("  calvin deploy --yes\n");
        println!("  # Preview what would change");
        println!("  calvin diff\n");
        println!("  # Deploy everything to home directory targets (~/.claude/, ~/.codex/, ...)");
        println!("  calvin deploy --home --yes\n");
        println!("  # Deploy to remote destination via SSH");
        println!("  calvin deploy --remote user@host:/path --yes\n");
        println!("  # CI-friendly check (strict mode, fail on warnings)");
        println!("  calvin check --mode strict --strict-warnings\n");
    }

    Ok(())
}
