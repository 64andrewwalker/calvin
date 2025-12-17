use std::path::PathBuf;

use crate::adapters::{all_adapters, OutputFile};
use crate::error::CalvinResult;
use crate::models::{AssetKind, PromptAsset, Scope, Target};

pub fn compile_assets(
    assets: &[PromptAsset],
    targets: &[Target],
    config: &crate::config::Config,
) -> CalvinResult<Vec<OutputFile>> {
    let mut outputs = Vec::new();

    // Check if Claude Code is in the target list.
    // This affects Cursor's behavior - if Claude Code is not selected, Cursor needs to generate commands.
    let has_claude_code = targets.is_empty() || targets.contains(&Target::ClaudeCode);
    let has_cursor = targets.is_empty() || targets.contains(&Target::Cursor);

    // Use cursor adapter with commands enabled if:
    // 1. Cursor is selected AND
    // 2. Claude Code is NOT selected
    let cursor_needs_commands = has_cursor && !has_claude_code;

    let adapters = all_adapters();

    for asset in assets {
        // Get effective targets for this asset
        let effective_targets = asset.frontmatter.effective_targets();

        for adapter in &adapters {
            let adapter_target = adapter.target();

            // Skip if target not enabled for this asset
            if !effective_targets.contains(&adapter_target) {
                continue;
            }

            // Skip if not in requested targets list (if specified)
            if !targets.is_empty() && !targets.contains(&adapter_target) {
                continue;
            }

            // Compile asset with this adapter
            let files = adapter.compile(asset)?;

            // Special handling for Cursor: add commands if Claude Code is not selected
            let files = if adapter_target == Target::Cursor && cursor_needs_commands {
                let mut all_files = files;
                // Generate commands for Cursor (same format as Claude Code)
                if matches!(asset.frontmatter.kind, AssetKind::Action | AssetKind::Agent) {
                    let commands_base = match asset.frontmatter.scope {
                        Scope::User => PathBuf::from("~/.cursor/commands"),
                        Scope::Project => PathBuf::from(".cursor/commands"),
                    };
                    let command_path = commands_base.join(format!("{}.md", asset.id));
                    let footer = adapter.footer(&asset.source_path.display().to_string());
                    let content = format!(
                        "{}\n\n{}\n\n{}",
                        asset.frontmatter.description,
                        asset.content.trim(),
                        footer
                    );
                    all_files.push(OutputFile::new(command_path, content));
                }
                all_files
            } else {
                files
            };

            outputs.extend(files);
        }
    }

    // Run post-compilation steps and security baselines
    for adapter in &adapters {
        let adapter_target = adapter.target();

        // Skip if not in requested targets list (if specified)
        if !targets.is_empty() && !targets.contains(&adapter_target) {
            continue;
        }

        // Post-compile (e.g. AGENTS.md)
        let post_outputs = adapter.post_compile(assets)?;
        outputs.extend(post_outputs);

        // Security baseline (e.g. settings.json, mcp.json)
        let baseline = adapter.security_baseline(config);
        outputs.extend(baseline);
    }

    // Sort for deterministic output
    outputs.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(outputs)
}

