//! Asset compilation service
//!
//! This module provides the `compile_assets` function that compiles PromptAssets
//! using the new infrastructure adapters.
//!
//! ## Usage
//!
//! ```ignore
//! use calvin::application::compile_assets;
//! use calvin::models::PromptAsset;
//! use calvin::config::Config;
//!
//! let outputs = compile_assets(&assets, &[], &Config::default())?;
//! ```
//!
//! ## Migration Note
//!
//! This function accepts legacy `PromptAsset` and `Target` types for backward
//! compatibility. For new code using domain types, use the `compile_assets` method
//! on `DeployUseCase` or `DiffUseCase` directly.

use std::path::PathBuf;

use crate::domain::entities::{Asset, OutputFile};
use crate::domain::value_objects::Target as DomainTarget;
use crate::error::CalvinResult;
use crate::infrastructure::adapters::all_adapters;
use crate::models::{PromptAsset, Target};

/// Convert legacy Target to domain Target
fn to_domain_target(target: &Target) -> DomainTarget {
    match target {
        Target::ClaudeCode => DomainTarget::ClaudeCode,
        Target::Cursor => DomainTarget::Cursor,
        Target::VSCode => DomainTarget::VSCode,
        Target::Antigravity => DomainTarget::Antigravity,
        Target::Codex => DomainTarget::Codex,
        Target::All => DomainTarget::All,
    }
}

/// Generate Cursor command file content (same format as ClaudeCode)
fn generate_cursor_command_content(asset: &Asset, footer: &str) -> String {
    let has_description = !asset.description().trim().is_empty();

    if has_description {
        format!(
            "{}\n\n{}\n\n{}",
            asset.description(),
            asset.content().trim(),
            footer
        )
    } else {
        format!("{}\n\n{}", asset.content().trim(), footer)
    }
}

/// Compile assets to output files
///
/// This function takes legacy model types and compiles them using the new
/// infrastructure adapters.
///
/// # Arguments
/// * `assets` - List of `PromptAsset` to compile
/// * `targets` - Optional filter for targets (empty = all)
/// * `config` - Configuration for security baselines and other settings
///
/// # Returns
/// A vector of `OutputFile` ready for deployment
pub fn compile_assets(
    assets: &[PromptAsset],
    targets: &[Target],
    config: &crate::config::Config,
) -> CalvinResult<Vec<OutputFile>> {
    let mut outputs = Vec::new();

    // Convert legacy assets to domain assets
    let domain_assets: Vec<Asset> = assets.iter().map(|a| Asset::from(a.clone())).collect();

    // Convert targets to domain targets
    let domain_targets: Vec<DomainTarget> = targets.iter().map(to_domain_target).collect();

    // Check if Claude Code is in the target list.
    // This affects Cursor's behavior - if Claude Code is not selected, Cursor needs to generate commands.
    let has_claude_code = targets.is_empty() || targets.contains(&Target::ClaudeCode);
    let has_cursor = targets.is_empty() || targets.contains(&Target::Cursor);

    // Use cursor adapter with commands enabled if:
    // 1. Cursor is selected AND
    // 2. Claude Code is NOT selected
    let cursor_needs_commands = has_cursor && !has_claude_code;

    // Get new infrastructure adapters
    let adapters = all_adapters();

    for asset in &domain_assets {
        // Get effective targets for this asset
        let effective_targets = asset.effective_targets();

        for adapter in &adapters {
            let adapter_target = adapter.target();

            // Skip if target not enabled for this asset
            if !effective_targets.contains(&adapter_target) {
                continue;
            }

            // Skip if not in requested targets list (if specified)
            if !domain_targets.is_empty() && !domain_targets.contains(&adapter_target) {
                continue;
            }

            // Compile asset with this adapter
            match adapter.compile(asset) {
                Ok(files) => {
                    outputs.extend(files);
                }
                Err(e) => {
                    return Err(crate::error::CalvinError::Compile {
                        message: e.to_string(),
                    });
                }
            }

            // Special handling for Cursor: add commands if Claude Code is not selected
            if adapter_target == DomainTarget::Cursor && cursor_needs_commands {
                use crate::domain::entities::AssetKind;
                use crate::domain::value_objects::Scope;

                if matches!(asset.kind(), AssetKind::Action | AssetKind::Agent) {
                    let commands_base = match asset.scope() {
                        Scope::User => PathBuf::from("~/.cursor/commands"),
                        Scope::Project => PathBuf::from(".cursor/commands"),
                    };
                    let command_path = commands_base.join(format!("{}.md", asset.id()));
                    let footer = adapter.footer(&asset.source_path_normalized());
                    let content = generate_cursor_command_content(asset, &footer);
                    outputs.push(OutputFile::new(command_path, content, DomainTarget::Cursor));
                }
            }
        }
    }

    // Run post-compilation steps for each adapter
    for adapter in &adapters {
        let adapter_target = adapter.target();

        // Skip if not in requested targets list (if specified)
        if !domain_targets.is_empty() && !domain_targets.contains(&adapter_target) {
            continue;
        }

        // Post-compile (e.g. AGENTS.md)
        match adapter.post_compile(&domain_assets) {
            Ok(post_outputs) => {
                outputs.extend(post_outputs);
            }
            Err(e) => {
                return Err(crate::error::CalvinError::Compile {
                    message: e.to_string(),
                });
            }
        }

        // Security baseline - convert config errors
        match adapter.security_baseline(config) {
            Ok(baseline) => {
                outputs.extend(baseline);
            }
            Err(e) => {
                return Err(crate::error::CalvinError::Compile {
                    message: e.to_string(),
                });
            }
        }
    }

    // Sort for deterministic output
    outputs.sort_by(|a, b| a.path().cmp(b.path()));

    Ok(outputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::models::{AssetKind, Frontmatter};

    #[test]
    fn compile_assets_single() {
        let fm = Frontmatter::new("Test asset");
        let asset = PromptAsset::new("test", "test.md", fm, "Content");
        let config = Config::default();

        let outputs = compile_assets(&[asset], &[], &config).unwrap();

        // Should generate output for all 5 adapters
        assert!(!outputs.is_empty());
    }

    #[test]
    fn compile_assets_target_filter() {
        let fm = Frontmatter::new("Test asset");
        let asset = PromptAsset::new("test", "test.md", fm, "Content");
        let config = Config::default();

        let outputs = compile_assets(&[asset], &[Target::ClaudeCode], &config).unwrap();

        // Should only generate Claude Code output
        assert!(outputs.iter().all(|o| o.path().starts_with(".claude")));
    }

    /// Test compile_assets behavior with cursor-only target (no ClaudeCode)
    /// This is a special case where Cursor needs to generate commands
    #[test]
    fn compile_assets_cursor_only_generates_commands() {
        // Create an ACTION asset - actions generate commands
        let mut fm = Frontmatter::new("Test action");
        fm.kind = AssetKind::Action;
        let asset = PromptAsset::new(
            "test-action",
            "actions/test-action.md",
            fm,
            "# Action content",
        );

        let config = Config::default();

        // Compile for Cursor only (without ClaudeCode)
        let outputs = compile_assets(&[asset], &[Target::Cursor], &config).unwrap();

        // Should have command output for Cursor
        // Use component-based comparison for cross-platform compatibility
        // (Windows may use mixed separators when PathBuf::from() preserves / but join() uses \)
        let has_cursor_command = outputs.iter().any(|o| {
            let path = o.path();
            let components: Vec<_> = path.components().collect();
            // Check path ends with: .cursor/commands/test-action.md
            components.len() >= 3
                && components[components.len() - 3]
                    .as_os_str()
                    .to_string_lossy()
                    .contains(".cursor")
                && components[components.len() - 2].as_os_str() == "commands"
                && components[components.len() - 1].as_os_str() == "test-action.md"
        });

        assert!(
            has_cursor_command,
            "Cursor-only compile should generate commands. Got paths: {:?}",
            outputs.iter().map(|o| o.path()).collect::<Vec<_>>()
        );
    }
}
