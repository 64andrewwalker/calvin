use crate::ui::theme::CalvinTheme;
use calvin::application::layers::{LayerQueryResult, LayerSummary};
use calvin::Target;
use is_terminal::IsTerminal;
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

    if json || !std::io::stdin().is_terminal() {
        // Non-interactive mode: use config enabled targets
        let targets = config.enabled_targets();
        return Some(targets);
    }

    let items: Vec<&str> = ALL_TARGETS.iter().map(target_display_name).collect();

    // Get enabled targets from config for default selection
    let enabled = config.enabled_targets();

    // Set defaults based on previously enabled targets in config
    let defaults: Vec<bool> = ALL_TARGETS.iter().map(|t| enabled.contains(t)).collect();

    // Use CalvinTheme for ●/○ icons
    let theme = CalvinTheme::new(crate::ui::terminal::detect_capabilities().supports_unicode);

    println!("\nSelect target platforms (use space to toggle, enter to confirm):");
    let selection = MultiSelect::with_theme(&theme)
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
        let config_targets: std::collections::HashSet<_> = enabled.iter().collect();
        let selected_set: std::collections::HashSet<_> = selected.iter().collect();

        if config_targets != selected_set {
            if let Err(e) = save_targets_to_config(path, &selected) {
                eprintln!("Warning: Could not save target selection: {}", e);
            } else {
                println!("Saved target selection to config.toml");
            }
        }
    }

    println!("Selected targets: {:?}", selected);
    Some(selected)
}

/// Layer selection result
#[derive(Debug, Clone)]
pub struct LayerSelection {
    /// Names of selected layers
    pub selected_layers: Vec<String>,
    /// Whether to use user layer
    pub use_user_layer: bool,
    /// Whether to use project layer
    pub use_project_layer: bool,
}

/// Interactive layer selection
///
/// Returns None if user aborts (selects nothing)
pub fn select_layers_interactive(
    layers_result: &LayerQueryResult,
    json: bool,
) -> Option<LayerSelection> {
    use dialoguer::MultiSelect;

    if json || !std::io::stdin().is_terminal() {
        // Non-interactive mode: use all layers
        return Some(LayerSelection {
            selected_layers: layers_result
                .layers
                .iter()
                .map(|l| l.name.clone())
                .collect(),
            use_user_layer: layers_result.layers.iter().any(|l| l.layer_type == "user"),
            use_project_layer: layers_result
                .layers
                .iter()
                .any(|l| l.layer_type == "project"),
        });
    }

    if layers_result.layers.len() <= 1 {
        // Only one layer, no need to prompt
        return Some(LayerSelection {
            selected_layers: layers_result
                .layers
                .iter()
                .map(|l| l.name.clone())
                .collect(),
            use_user_layer: layers_result.layers.iter().any(|l| l.layer_type == "user"),
            use_project_layer: layers_result
                .layers
                .iter()
                .any(|l| l.layer_type == "project"),
        });
    }

    let items: Vec<String> = layers_result
        .layers
        .iter()
        .map(layer_display_name)
        .collect();

    // By default, select all layers
    let defaults: Vec<bool> = vec![true; layers_result.layers.len()];

    let theme = CalvinTheme::new(crate::ui::terminal::detect_capabilities().supports_unicode);

    println!("\nSelect layers to deploy (use space to toggle, enter to confirm):");
    let selection = MultiSelect::with_theme(&theme)
        .items(&items)
        .defaults(&defaults)
        .interact()
        .unwrap_or_default();

    if selection.is_empty() {
        println!("No layers selected. Aborted.");
        return None;
    }

    let selected_layers: Vec<String> = selection
        .iter()
        .map(|&i| layers_result.layers[i].name.clone())
        .collect();

    let use_user_layer = selection
        .iter()
        .any(|&i| layers_result.layers[i].layer_type == "user");
    let use_project_layer = selection
        .iter()
        .any(|&i| layers_result.layers[i].layer_type == "project");

    println!("Selected layers: {}", selected_layers.join(", "));
    Some(LayerSelection {
        selected_layers,
        use_user_layer,
        use_project_layer,
    })
}

/// Get display name for a layer
fn layer_display_name(layer: &LayerSummary) -> String {
    let path_display = truncate_middle(&layer.original_path.display().to_string(), 40);
    format!(
        "{:<8} {} ({} assets)",
        layer.layer_type, path_display, layer.asset_count
    )
}

/// Truncate path in the middle if too long
fn truncate_middle(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    let half = (max_len - 3) / 2;
    format!("{}...{}", &s[..half], &s[s.len() - half..])
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_layer_summary(name: &str, layer_type: &str, asset_count: usize) -> LayerSummary {
        LayerSummary {
            name: name.to_string(),
            layer_type: layer_type.to_string(),
            original_path: PathBuf::from(format!("/path/to/{}", name)),
            resolved_path: PathBuf::from(format!("/path/to/{}", name)),
            asset_count,
        }
    }

    #[test]
    fn test_layer_selection_single_layer_returns_all() {
        let result = LayerQueryResult {
            layers: vec![make_layer_summary("project", "project", 10)],
            merged_asset_count: 10,
            overridden_asset_count: 0,
        };

        // In non-interactive (json) mode, should return all layers
        let selection = select_layers_interactive(&result, true).unwrap();
        assert_eq!(selection.selected_layers, vec!["project"]);
        assert!(selection.use_project_layer);
        assert!(!selection.use_user_layer);
    }

    #[test]
    fn test_layer_selection_json_mode_returns_all_layers() {
        let result = LayerQueryResult {
            layers: vec![
                make_layer_summary("user", "user", 5),
                make_layer_summary("project", "project", 10),
            ],
            merged_asset_count: 15,
            overridden_asset_count: 0,
        };

        // In JSON mode, should return all layers without prompting
        let selection = select_layers_interactive(&result, true).unwrap();
        assert_eq!(selection.selected_layers, vec!["user", "project"]);
        assert!(selection.use_project_layer);
        assert!(selection.use_user_layer);
    }

    #[test]
    fn test_layer_display_name_formatting() {
        let layer = make_layer_summary("project", "project", 27);
        let display = layer_display_name(&layer);
        assert!(display.contains("project"));
        assert!(display.contains("27 assets"));
    }

    #[test]
    fn test_truncate_middle_short_string() {
        let s = "short";
        assert_eq!(truncate_middle(s, 10), "short");
    }

    #[test]
    fn test_truncate_middle_long_string() {
        let s = "this_is_a_very_long_path_that_needs_truncation";
        let truncated = truncate_middle(s, 20);
        assert!(truncated.len() <= 20);
        assert!(truncated.contains("..."));
    }
}
