use super::*;
use crate::models::{Frontmatter, PromptAsset};

#[test]
fn test_sync_options_default() {
    let opts = SyncOptions::default();
    assert!(!opts.force);
    assert!(!opts.dry_run);
    assert!(!opts.interactive);
    assert!(opts.targets.is_empty());
}

#[test]
fn test_sync_result_new() {
    let result = SyncResult::new();
    assert!(result.written.is_empty());
    assert!(result.skipped.is_empty());
    assert!(result.errors.is_empty());
    assert!(result.is_success());
}

#[test]
fn test_compile_assets_single() {
    let fm = Frontmatter::new("Test asset");
    let asset = PromptAsset::new("test", "test.md", fm, "Content");
    let config = crate::config::Config::default();

    let outputs = compile_assets(&[asset], &[], &config).unwrap();

    // Should generate output for all 5 adapters
    assert!(!outputs.is_empty());
}

#[test]
fn test_compile_assets_available_via_compile_module() {
    let fm = Frontmatter::new("Test asset");
    let asset = PromptAsset::new("test", "test.md", fm, "Content");
    let config = crate::config::Config::default();

    let outputs = super::compile::compile_assets(&[asset], &[], &config).unwrap();
    assert!(!outputs.is_empty());
}

#[test]
fn test_unified_diff_available_via_conflict_module() {
    let diff = super::conflict::unified_diff("file.txt", "old\n", "new\n");
    assert!(diff.contains("a/file.txt"));
    assert!(diff.contains("b/file.txt"));
}

#[test]
fn test_compile_assets_target_filter() {
    let fm = Frontmatter::new("Test asset");
    let asset = PromptAsset::new("test", "test.md", fm, "Content");
    let config = crate::config::Config::default();

    let outputs = compile_assets(&[asset], &[Target::ClaudeCode], &config).unwrap();

    // Should only generate Claude Code output
    assert!(outputs.iter().all(|o| o.path().starts_with(".claude")));
}

#[test]
fn test_validate_path_safety_normal() {
    let root = Path::new("/project");
    assert!(validate_path_safety(Path::new(".claude/settings.json"), root).is_ok());
    assert!(validate_path_safety(Path::new(".cursor/rules/test/RULE.md"), root).is_ok());
}

#[test]
fn test_validate_path_safety_user_paths() {
    let root = Path::new("/project");
    // User-level paths starting with ~ are always allowed
    assert!(validate_path_safety(Path::new("~/.codex/prompts/test.md"), root).is_ok());
    assert!(validate_path_safety(Path::new("~/.claude/commands/test.md"), root).is_ok());
}

#[test]
fn test_validate_path_safety_traversal() {
    let root = Path::new("/project");
    // Path traversal should be blocked
    assert!(validate_path_safety(Path::new("../etc/passwd"), root).is_err());
    assert!(validate_path_safety(Path::new("../../malicious"), root).is_err());
}

#[test]
fn test_expand_home_dir() {
    // Test that ~ is expanded (if HOME is set)
    if std::env::var("HOME").is_ok() {
        let expanded = expand_home_dir(Path::new("~/.codex/prompts"));
        assert!(!expanded.to_string_lossy().starts_with("~"));
        assert!(expanded.to_string_lossy().contains(".codex/prompts"));
    }

    // Non-home paths should pass through unchanged
    let unchanged = expand_home_dir(Path::new(".claude/settings.json"));
    assert_eq!(unchanged, Path::new(".claude/settings.json"));
}

// Note: Interactive sync tests have been migrated to sync/engine.rs tests.
// Those tests use SyncEngine with MockFileSystem and MockConflictResolver for
// better isolation and testability. See:
// - engine_mock_fs_* tests for basic sync behavior
// - engine_conflict_resolver_* tests for interactive conflict resolution

// --- Variants ---

#[test]
fn safety__rejects_dotdot_at_start() {
    let root = Path::new("/project");
    assert!(validate_path_safety(Path::new(".."), root).is_err());
    assert!(validate_path_safety(Path::new("../foo"), root).is_err());
}

#[test]
fn safety__rejects_absolute_paths_outside_root() {
    let root = Path::new("/project");
    // Absolute path /etc/passwd is definitely not inside /project
    // unless /project IS / (unlikely in test)
    assert!(validate_path_safety(Path::new("/etc/passwd"), root).is_err());
}

#[test]
fn safety__allows_tilde_slash() {
    let root = Path::new("/project");
    assert!(validate_path_safety(Path::new("~/foo"), root).is_ok());
}

// === TDD: Adapter output consistency ===

/// Test compile_assets behavior with cursor-only target (no ClaudeCode)
/// This is a special case where Cursor needs to generate commands
#[test]
fn compile_assets_cursor_only_generates_commands() {
    use crate::config::Config;
    use crate::models::{AssetKind, Frontmatter};
    use crate::sync::compile::compile_assets;

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
    let has_cursor_command = outputs.iter().any(|o| {
        o.path()
            .to_string_lossy()
            .contains(".cursor/commands/test-action.md")
    });

    assert!(
        has_cursor_command,
        "Cursor-only compile should generate commands. Got paths: {:?}",
        outputs.iter().map(|o| o.path()).collect::<Vec<_>>()
    );
}

/// Test that new adapters produce compatible output paths
#[test]
fn new_adapter_output_paths_match_legacy() {
    use crate::domain::entities::Asset;
    use crate::infrastructure::adapters::all_adapters as new_all_adapters;
    use crate::models::{AssetKind, Frontmatter};

    // Create a test POLICY asset (policies generate rules in Cursor)
    let mut legacy_fm = Frontmatter::new("Test policy");
    legacy_fm.kind = AssetKind::Policy;
    let legacy_asset = PromptAsset::new("test-policy", "test-policy.md", legacy_fm, "# Content");

    // Convert to new format
    let new_asset = Asset::from(legacy_asset.clone());

    // Get adapters
    let legacy_adapters = crate::adapters::all_adapters();
    let new_adapters = new_all_adapters();

    // Compare Cursor adapter output paths
    let legacy_cursor = legacy_adapters
        .iter()
        .find(|a| a.target() == Target::Cursor)
        .unwrap();
    let new_cursor = new_adapters
        .iter()
        .find(|a| a.target() == crate::domain::value_objects::Target::Cursor)
        .unwrap();

    let legacy_outputs = legacy_cursor.compile(&legacy_asset).unwrap();
    let new_outputs = new_cursor.compile(&new_asset).unwrap();

    // Both should produce at least one output
    assert!(
        !legacy_outputs.is_empty(),
        "Legacy cursor should produce output"
    );
    assert!(!new_outputs.is_empty(), "New cursor should produce output");

    // Paths should be similar (new uses .cursor/rules/<id>/RULE.md format)
    // Just check that both produce .cursor/rules/ paths
    for output in &legacy_outputs {
        assert!(
            output.path.to_string_lossy().contains(".cursor/rules"),
            "Legacy output should be in .cursor/rules: {:?}",
            output.path
        );
    }
    for output in &new_outputs {
        assert!(
            output.path().to_string_lossy().contains(".cursor/rules"),
            "New output should be in .cursor/rules: {:?}",
            output.path()
        );
    }
}
