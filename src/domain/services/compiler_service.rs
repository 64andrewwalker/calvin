//! Compiler Service
//!
//! Unified compilation logic for transforming Assets into OutputFiles.
//! This service consolidates platform-specific compilation logic that was
//! previously scattered across multiple use cases.
//!
//! ## Key Responsibility
//!
//! - `cursor_needs_commands`: When Cursor is the only target (no Claude Code),
//!   generate commands for Action/Agent assets to `.cursor/commands/`

use std::path::PathBuf;

use crate::domain::entities::{Asset, AssetKind, OutputFile};
use crate::domain::ports::TargetAdapter;
use crate::domain::value_objects::{Scope, Target};

/// Error type for compilation failures
#[derive(Debug, Clone)]
pub struct CompileError {
    pub message: String,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CompileError {}

/// Compiler Service
///
/// Provides unified compilation logic for all deployment scenarios.
/// Includes special handling for platform-specific quirks:
///
/// - **Cursor-only deployment**: When targets include Cursor but not Claude Code,
///   generates `.cursor/commands/` files for Action/Agent assets.
pub struct CompilerService {
    adapters: Vec<Box<dyn TargetAdapter>>,
}

impl CompilerService {
    /// Create a new compiler service with the given adapters
    pub fn new(adapters: Vec<Box<dyn TargetAdapter>>) -> Self {
        Self { adapters }
    }

    /// Get a reference to the adapters
    pub fn adapters(&self) -> &[Box<dyn TargetAdapter>] {
        &self.adapters
    }

    /// Check if Cursor should generate its own commands (static version)
    ///
    /// This is the core logic for determining when Cursor needs to generate
    /// its own commands instead of relying on Claude Code's commands.
    ///
    /// Returns true when:
    /// - Cursor is in the target list AND
    /// - Claude Code is NOT in the target list
    pub fn cursor_needs_commands(targets: &[Target]) -> bool {
        // Empty targets means all platforms - Claude Code is included
        if targets.is_empty() {
            return false;
        }

        // Check if Target::All is present
        if targets.iter().any(|t| t.is_all()) {
            return false;
        }

        let has_claude_code = targets.contains(&Target::ClaudeCode);
        let has_cursor = targets.contains(&Target::Cursor);

        has_cursor && !has_claude_code
    }

    /// Generate Cursor command file content (static version)
    ///
    /// Uses the same format as Claude Code commands for consistency.
    pub fn generate_command_content(asset: &Asset, footer: &str) -> String {
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

    /// Check if Cursor should generate its own commands (instance method)
    ///
    /// Returns true when:
    /// - Cursor is in the target list AND
    /// - Claude Code is NOT in the target list
    ///
    /// This is needed because Cursor can read Claude's commands from
    /// `~/.claude/commands/`, but when deploying Cursor-only, we need
    /// to generate commands directly to `.cursor/commands/`.
    pub fn should_cursor_generate_commands(&self, targets: &[Target]) -> bool {
        // Empty targets means all platforms - Claude Code is included
        if targets.is_empty() {
            return false;
        }

        // Check if Target::All is present
        if targets.iter().any(|t| t.is_all()) {
            return false;
        }

        let has_claude_code = targets.contains(&Target::ClaudeCode);
        let has_cursor = targets.contains(&Target::Cursor);

        has_cursor && !has_claude_code
    }

    /// Generate Cursor command file content
    ///
    /// Uses the same format as Claude Code commands for consistency.
    pub fn generate_cursor_command_content(&self, asset: &Asset, footer: &str) -> String {
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
    /// This method:
    /// 1. Filters adapters based on target list
    /// 2. Compiles each asset with each applicable adapter
    /// 3. Applies special handling for Cursor-only deployment
    /// 4. Runs post-compile for each adapter (e.g., AGENTS.md)
    pub fn compile(
        &self,
        assets: &[Asset],
        targets: &[Target],
    ) -> Result<Vec<OutputFile>, CompileError> {
        let mut outputs = Vec::new();

        // Determine which adapters to use
        let active_adapters: Vec<&Box<dyn TargetAdapter>> =
            if targets.is_empty() || targets.iter().any(|t| t.is_all()) {
                self.adapters.iter().collect()
            } else {
                self.adapters
                    .iter()
                    .filter(|a| targets.contains(&a.target()))
                    .collect()
            };

        // Check if Cursor needs to generate its own commands
        let cursor_needs_commands = self.should_cursor_generate_commands(targets);

        // Compile each asset with each adapter
        for asset in assets {
            // Get the effective targets for this asset (respects asset-level targets field)
            let asset_targets = asset.effective_targets();

            for adapter in &active_adapters {
                // Skip if this adapter's target is not enabled for this asset
                if !asset_targets.contains(&adapter.target()) {
                    continue;
                }

                match adapter.compile(asset) {
                    Ok(adapter_outputs) => outputs.extend(adapter_outputs),
                    Err(e) => {
                        return Err(CompileError {
                            message: format!(
                                "Adapter {} failed on {}: {}",
                                adapter.target().display_name(),
                                asset.id(),
                                e
                            ),
                        });
                    }
                }

                // Special handling for Cursor: add commands if Claude Code is not selected
                if adapter.target() == Target::Cursor
                    && cursor_needs_commands
                    && matches!(asset.kind(), AssetKind::Action | AssetKind::Agent)
                {
                    let commands_base = match asset.scope() {
                        Scope::User => PathBuf::from("~/.cursor/commands"),
                        Scope::Project => PathBuf::from(".cursor/commands"),
                    };
                    let command_path = commands_base.join(format!("{}.md", asset.id()));
                    let footer = adapter.footer(&asset.source_path_normalized());
                    let content = self.generate_cursor_command_content(asset, &footer);
                    outputs.push(OutputFile::new(command_path, content, Target::Cursor));
                }
            }
        }

        // Post-compile for each adapter (e.g., generate AGENTS.md)
        for adapter in &active_adapters {
            match adapter.post_compile(assets) {
                Ok(post_outputs) => outputs.extend(post_outputs),
                Err(e) => {
                    return Err(CompileError {
                        message: format!(
                            "Post-compile for {} failed: {}",
                            adapter.target().display_name(),
                            e
                        ),
                    });
                }
            }
        }

        Ok(outputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ports::target_adapter::{AdapterDiagnostic, AdapterError};

    // === Test Helpers ===

    fn create_test_service() -> CompilerService {
        CompilerService::new(vec![])
    }

    fn create_action_asset(id: &str, description: &str, content: &str) -> Asset {
        Asset::new(id, format!("actions/{}.md", id), description, content)
            .with_kind(AssetKind::Action)
    }

    fn create_policy_asset(id: &str, description: &str, content: &str) -> Asset {
        Asset::new(id, format!("policies/{}.md", id), description, content)
            .with_kind(AssetKind::Policy)
    }

    // Mock adapter for testing
    struct MockAdapter {
        target: Target,
    }

    impl TargetAdapter for MockAdapter {
        fn target(&self) -> Target {
            self.target
        }

        fn compile(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError> {
            Ok(vec![OutputFile::new(
                format!(".test/{}.md", asset.id()),
                asset.content().to_string(),
                self.target,
            )])
        }

        fn validate(&self, _output: &OutputFile) -> Vec<AdapterDiagnostic> {
            vec![]
        }
    }

    // === TDD 1.1: CompilerService 骨架 ===

    #[test]
    fn compiler_service_new_creates_instance() {
        let adapters: Vec<Box<dyn TargetAdapter>> = vec![];
        let service = CompilerService::new(adapters);
        assert!(service.adapters().is_empty());
    }

    #[test]
    fn compiler_service_with_adapters() {
        let adapters: Vec<Box<dyn TargetAdapter>> = vec![
            Box::new(MockAdapter {
                target: Target::Cursor,
            }),
            Box::new(MockAdapter {
                target: Target::ClaudeCode,
            }),
        ];
        let service = CompilerService::new(adapters);
        assert_eq!(service.adapters().len(), 2);
    }

    // === TDD 1.2: should_cursor_generate_commands ===

    #[test]
    fn cursor_needs_commands_when_cursor_only() {
        let service = create_test_service();
        // Cursor only - should generate commands
        assert!(service.should_cursor_generate_commands(&[Target::Cursor]));
    }

    #[test]
    fn cursor_no_commands_when_with_claude_code() {
        let service = create_test_service();
        // Cursor + Claude Code - Claude provides commands
        assert!(!service.should_cursor_generate_commands(&[Target::Cursor, Target::ClaudeCode]));
    }

    #[test]
    fn cursor_no_commands_when_empty_targets() {
        let service = create_test_service();
        // Empty targets means all - Claude Code included
        assert!(!service.should_cursor_generate_commands(&[]));
    }

    #[test]
    fn cursor_no_commands_when_target_all() {
        let service = create_test_service();
        // Target::All includes Claude Code
        assert!(!service.should_cursor_generate_commands(&[Target::All]));
    }

    #[test]
    fn cursor_needs_commands_with_vscode() {
        let service = create_test_service();
        // Cursor + VSCode (no Claude Code) - should generate
        assert!(service.should_cursor_generate_commands(&[Target::Cursor, Target::VSCode]));
    }

    // === TDD 1.3: generate_cursor_command_content ===

    #[test]
    fn generate_content_with_description() {
        let service = create_test_service();
        let asset = create_action_asset("test", "My description", "# Content");

        let content = service.generate_cursor_command_content(&asset, "<!-- footer -->");

        assert!(content.starts_with("My description"));
        assert!(content.contains("# Content"));
        assert!(content.ends_with("<!-- footer -->"));
    }

    #[test]
    fn generate_content_without_description() {
        let service = create_test_service();
        let asset = create_action_asset("test", "", "# Content");

        let content = service.generate_cursor_command_content(&asset, "<!-- footer -->");

        assert!(content.starts_with("# Content"));
        assert!(content.ends_with("<!-- footer -->"));
    }

    #[test]
    fn generate_content_trims_whitespace() {
        let service = create_test_service();
        let asset = create_action_asset("test", "  My description  ", "  # Content  ");

        let content = service.generate_cursor_command_content(&asset, "<!-- footer -->");

        // Description is trimmed when checking if empty, but kept as-is in output
        // Content is trimmed
        assert!(content.contains("# Content"));
    }

    // === TDD 1.4: compile 核心逻辑 ===

    #[test]
    fn compile_empty_assets_returns_empty() {
        let service = create_test_service();
        let outputs = service.compile(&[], &[]).unwrap();
        assert!(outputs.is_empty());
    }

    #[test]
    fn compile_with_mock_adapter() {
        let adapters: Vec<Box<dyn TargetAdapter>> = vec![Box::new(MockAdapter {
            target: Target::Cursor,
        })];
        let service = CompilerService::new(adapters);

        let asset = create_policy_asset("test", "desc", "content");
        let outputs = service.compile(&[asset], &[Target::Cursor]).unwrap();

        assert!(!outputs.is_empty());
    }

    #[test]
    fn compile_filters_by_target() {
        let adapters: Vec<Box<dyn TargetAdapter>> = vec![
            Box::new(MockAdapter {
                target: Target::Cursor,
            }),
            Box::new(MockAdapter {
                target: Target::ClaudeCode,
            }),
        ];
        let service = CompilerService::new(adapters);

        let asset = create_policy_asset("test", "desc", "content");

        // Only request Cursor
        let outputs = service.compile(&[asset], &[Target::Cursor]).unwrap();

        // Should only have Cursor output
        assert!(outputs.iter().all(|o| o.target() == Target::Cursor));
    }

    #[test]
    fn compile_cursor_only_generates_commands_for_action() {
        // Use real Cursor adapter to test command generation
        use crate::infrastructure::adapters::CursorAdapter;

        let adapters: Vec<Box<dyn TargetAdapter>> =
            vec![Box::new(CursorAdapter::new()) as Box<dyn TargetAdapter>];
        let service = CompilerService::new(adapters);

        let asset = create_action_asset("test-action", "desc", "content");

        let outputs = service.compile(&[asset], &[Target::Cursor]).unwrap();

        // Should have command output
        let has_command = outputs
            .iter()
            .any(|o| o.path().to_string_lossy().contains("commands"));
        assert!(
            has_command,
            "Cursor-only should generate commands. Got: {:?}",
            outputs.iter().map(|o| o.path()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn compile_cursor_with_claude_no_cursor_commands() {
        use crate::infrastructure::adapters::{ClaudeCodeAdapter, CursorAdapter};

        let adapters: Vec<Box<dyn TargetAdapter>> = vec![
            Box::new(CursorAdapter::new()) as Box<dyn TargetAdapter>,
            Box::new(ClaudeCodeAdapter::new()) as Box<dyn TargetAdapter>,
        ];
        let service = CompilerService::new(adapters);

        let asset = create_action_asset("test-action", "desc", "content");

        let outputs = service
            .compile(&[asset], &[Target::Cursor, Target::ClaudeCode])
            .unwrap();

        // Should NOT have .cursor/commands (Claude provides commands)
        let cursor_command = outputs
            .iter()
            .any(|o| o.path().to_string_lossy().contains(".cursor/commands"));
        assert!(
            !cursor_command,
            "Cursor should not generate commands when Claude Code is present"
        );

        // But should have .claude/commands
        let claude_command = outputs
            .iter()
            .any(|o| o.path().to_string_lossy().contains(".claude/commands"));
        assert!(claude_command, "Claude Code should generate commands");
    }

    #[test]
    fn compile_cursor_only_user_scope_generates_home_path() {
        use crate::infrastructure::adapters::CursorAdapter;

        let adapters: Vec<Box<dyn TargetAdapter>> =
            vec![Box::new(CursorAdapter::new()) as Box<dyn TargetAdapter>];
        let service = CompilerService::new(adapters);

        let asset = create_action_asset("test-action", "desc", "content").with_scope(Scope::User);

        let outputs = service.compile(&[asset], &[Target::Cursor]).unwrap();

        // Should have ~/.cursor/commands path
        let has_home_command = outputs
            .iter()
            .any(|o| o.path().to_string_lossy().contains("~/.cursor/commands"));
        assert!(
            has_home_command,
            "User scope should generate ~/.cursor/commands. Got: {:?}",
            outputs.iter().map(|o| o.path()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn compile_policy_no_cursor_commands() {
        use crate::infrastructure::adapters::CursorAdapter;

        let adapters: Vec<Box<dyn TargetAdapter>> =
            vec![Box::new(CursorAdapter::new()) as Box<dyn TargetAdapter>];
        let service = CompilerService::new(adapters);

        // Policy asset - should NOT generate commands, only rules
        let asset = create_policy_asset("test-policy", "desc", "content");

        let outputs = service.compile(&[asset], &[Target::Cursor]).unwrap();

        // Should have rule, not command
        let has_rule = outputs
            .iter()
            .any(|o| o.path().to_string_lossy().contains("rules"));
        let has_command = outputs
            .iter()
            .any(|o| o.path().to_string_lossy().contains("commands"));

        assert!(has_rule, "Policy should generate rule");
        assert!(!has_command, "Policy should NOT generate command");
    }
}
