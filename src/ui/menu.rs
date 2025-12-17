use calvin::Target;
use std::path::Path;

/// All available targets for interactive selection (excludes Target::All)
pub const ALL_TARGETS: [Target; 5] = [
    Target::ClaudeCode,
    Target::Cursor,
    Target::VSCode,
    Target::Codex,
    Target::Antigravity,
];

/// Get display name for a target
fn target_display_name(t: &Target) -> &'static str {
    match t {
        Target::ClaudeCode => "Claude Code (.claude/)",
        Target::Cursor => "Cursor (.cursor/)",
        Target::VSCode => "VS Code (.vscode/)",
        Target::Codex => "Codex (.codex/)",
        Target::Antigravity => "Antigravity/Gemini (.gemini/)",
        Target::All => "All platforms",
    }
}

/// Get kebab-case name for a target (for config.toml)
fn target_kebab(t: &Target) -> &'static str {
    match t {
        Target::ClaudeCode => "claude-code",
        Target::Cursor => "cursor",
        Target::VSCode => "vscode",
        Target::Codex => "codex",
        Target::Antigravity => "antigravity",
        Target::All => "all",
    }
}

/// Interactive target selection with remembered defaults from config
/// Returns None if user aborts (selects nothing)
/// 
/// DP-7: Configuration Remembers - saves user's choice if different from config
pub fn select_targets_interactive(
    config: &calvin::config::Config,
    json: bool,
) -> Option<Vec<Target>> {
    select_targets_interactive_with_save(config, json, None)
}

/// Interactive target selection with optional config saving
pub fn select_targets_interactive_with_save(
    config: &calvin::config::Config,
    json: bool,
    config_path: Option<&Path>,
) -> Option<Vec<Target>> {
    use dialoguer::MultiSelect;

    if json || !atty::is(atty::Stream::Stdin) {
        // Non-interactive mode: use config or all targets
        let targets = if config.targets.enabled.is_empty()
            || config.targets.enabled.contains(&Target::All)
        {
            ALL_TARGETS.to_vec()
        } else {
            config.targets.enabled.clone()
        };
        return Some(targets);
    }

    let items: Vec<&str> = ALL_TARGETS.iter().map(target_display_name).collect();

    // Set defaults based on previously enabled targets in config
    let defaults: Vec<bool> = ALL_TARGETS
        .iter()
        .map(|t| config.targets.enabled.contains(t) || config.targets.enabled.contains(&Target::All))
        .collect();

    println!("\n📋 Select target platforms (use space to toggle, enter to confirm):");
    let selection = MultiSelect::new()
        .items(&items)
        .defaults(&defaults)
        .interact()
        .unwrap_or_default();

    if selection.is_empty() {
        println!("No targets selected. Aborted.");
        return None;
    }

    let selected: Vec<Target> = selection.iter().map(|&i| ALL_TARGETS[i]).collect();
    
    // DP-7: Configuration Remembers - save if different
    if let Some(path) = config_path {
        let config_targets: std::collections::HashSet<_> = config.targets.enabled.iter().collect();
        let selected_set: std::collections::HashSet<_> = selected.iter().collect();
        
        if config_targets != selected_set {
            if let Err(e) = save_targets_to_config(path, &selected) {
                eprintln!("⚠️  Could not save target selection: {}", e);
            } else {
                println!("💾 Saved target selection to config.toml");
            }
        }
    }
    
    println!("Selected targets: {:?}", selected);
    Some(selected)
}

/// Save selected targets to config.toml
fn save_targets_to_config(config_path: &Path, targets: &[Target]) -> std::io::Result<()> {
    // Read existing config
    let content = std::fs::read_to_string(config_path).unwrap_or_default();
    
    // Build new targets line
    let targets_str = targets
        .iter()
        .map(target_kebab)
        .collect::<Vec<_>>()
        .join("\", \"");
    let new_targets_line = format!("enabled = [\"{}\"]", targets_str);
    
    // Replace or add targets line
    let new_content = if content.contains("[targets]") {
        // Replace existing enabled line
        let mut result = String::new();
        let mut in_targets_section = false;
        let mut replaced = false;
        
        for line in content.lines() {
            if line.trim() == "[targets]" {
                in_targets_section = true;
                result.push_str(line);
                result.push('\n');
            } else if in_targets_section && line.trim().starts_with("enabled") {
                result.push_str(&new_targets_line);
                result.push('\n');
                replaced = true;
                in_targets_section = false;
            } else if line.starts_with('[') && line != "[targets]" {
                in_targets_section = false;
                result.push_str(line);
                result.push('\n');
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }
        
        if !replaced {
            // Add enabled line after [targets]
            result = result.replace("[targets]\n", &format!("[targets]\n{}\n", new_targets_line));
        }
        
        result
    } else {
        // Add [targets] section
        format!("{}\n[targets]\n{}\n", content.trim_end(), new_targets_line)
    };
    
    std::fs::write(config_path, new_content)
}

