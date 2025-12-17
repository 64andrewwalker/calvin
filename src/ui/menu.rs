use calvin::Target;

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

/// Interactive target selection with remembered defaults from config
/// Returns None if user aborts (selects nothing)
pub fn select_targets_interactive(
    config: &calvin::config::Config,
    json: bool,
    allow_prompt: bool,
) -> Option<Vec<Target>> {
    use dialoguer::MultiSelect;

    if json || !allow_prompt || !atty::is(atty::Stream::Stdin) {
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
    println!("Selected targets: {:?}", selected);
    Some(selected)
}
