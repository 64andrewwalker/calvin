use super::*;
use crate::fs::FileSystem;
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
    assert!(outputs.iter().all(|o| o.path.starts_with(".claude")));
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

#[test]
fn test_sync_user_scope() {
    use crate::models::{Frontmatter, Scope};

    // Setup
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().to_path_buf();
    let config = crate::config::Config::default();

    let mut fm_user = Frontmatter::new("User action");
    fm_user.scope = Scope::User;
    let asset_user = PromptAsset::new("user-cmd", "user.md", fm_user, "content");

    let mut fm_proj = Frontmatter::new("Project action");
    fm_proj.scope = Scope::Project;
    let asset_proj = PromptAsset::new("proj-cmd", "proj.md", fm_proj, "content");

    // Emulate install --user: Filter for user scope
    let assets = vec![asset_user, asset_proj];
    let user_assets: Vec<_> = assets
        .into_iter()
        .filter(|a| a.frontmatter.scope == Scope::User)
        .collect();

    assert_eq!(user_assets.len(), 1);

    // Compile
    let outputs = compile_assets(&user_assets, &[], &config).unwrap();

    // Sync to "home"
    let options = SyncOptions {
        force: true,
        dry_run: false,
        interactive: false,
        targets: vec![],
    };
    let result = sync_outputs(&home, &outputs, &options).unwrap();

    assert!(result.is_success());
    // User asset should be installed
    if cfg!(feature = "claude") {
        // Logic depends on adapter. Assuming Claude is default enabled.
    }

    // Check for generated files (adapters generate based on kind)
    assert!(home.join(".claude/commands/user-cmd.md").exists());
    assert!(!home.join(".claude/commands/proj-cmd.md").exists());

    // Lockfile should exist in .promptpack relative to home
    assert!(home.join(".promptpack/.calvin.lock").exists());
}

#[test]
fn test_sync_with_mock_fs() {
    use crate::fs::{FileSystem, MockFileSystem};

    let mock_fs = MockFileSystem::new();
    // Setup outputs
    let outputs = vec![OutputFile::new(".claude/test.md", "content")];
    let options = SyncOptions {
        force: true,
        dry_run: false,
        interactive: false,
        targets: vec![],
    };
    let root = Path::new("/mock/root");

    sync_with_fs(root, &outputs, &options, &mock_fs).unwrap();

    // Assert file exists in mock_fs
    assert!(mock_fs.exists(Path::new("/mock/root/.claude/test.md")));
    // Assert lockfile exists
    assert!(mock_fs.exists(Path::new("/mock/root/.promptpack/.calvin.lock")));
}

// === TDD: US-4 Interactive sync confirmation (Sprint 1 / P0) ===

#[test]
fn test_sync_interactive_overwrite_modified_file() {
    use crate::fs::MockFileSystem;

    struct Prompt {
        calls: usize,
    }
    impl SyncPrompter for Prompt {
        fn prompt_conflict(&mut self, _path: &str, _reason: ConflictReason) -> ConflictChoice {
            self.calls += 1;
            ConflictChoice::Overwrite
        }

        fn show_diff(&mut self, _diff: &str) {}
    }

    let mock_fs = MockFileSystem::new();
    let root = Path::new("/mock/root");

    // Initial sync to create lockfile + baseline file.
    let outputs_v1 = vec![OutputFile::new(".claude/settings.json", "generated v1\n")];
    let options_force = SyncOptions {
        force: true,
        dry_run: false,
        interactive: false,
        targets: vec![],
    };
    sync_with_fs(root, &outputs_v1, &options_force, &mock_fs).unwrap();

    // Simulate user modification.
    {
        let mut files = mock_fs.files.lock().unwrap();
        files.insert(
            PathBuf::from("/mock/root/.claude/settings.json"),
            "user edit\n".to_string(),
        );
    }

    // New generated output.
    let outputs_v2 = vec![OutputFile::new(".claude/settings.json", "generated v2\n")];
    let options = SyncOptions {
        force: false,
        dry_run: false,
        interactive: true,
        targets: vec![],
    };

    let mut prompter = Prompt { calls: 0 };
    let result = sync_with_fs_with_prompter(root, &outputs_v2, &options, &mock_fs, &mut prompter)
        .unwrap();

    assert_eq!(prompter.calls, 1);
    assert!(result.skipped.is_empty());
    assert!(result.errors.is_empty());

    let content = mock_fs
        .read_to_string(Path::new("/mock/root/.claude/settings.json"))
        .unwrap();
    assert_eq!(content, "generated v2\n");
}

#[test]
fn test_sync_interactive_skip_modified_file() {
    use crate::fs::MockFileSystem;

    struct Prompt;
    impl SyncPrompter for Prompt {
        fn prompt_conflict(&mut self, _path: &str, _reason: ConflictReason) -> ConflictChoice {
            ConflictChoice::Skip
        }

        fn show_diff(&mut self, _diff: &str) {}
    }

    let mock_fs = MockFileSystem::new();
    let root = Path::new("/mock/root");

    let outputs_v1 = vec![OutputFile::new(".claude/settings.json", "generated v1\n")];
    let options_force = SyncOptions {
        force: true,
        dry_run: false,
        interactive: false,
        targets: vec![],
    };
    sync_with_fs(root, &outputs_v1, &options_force, &mock_fs).unwrap();

    {
        let mut files = mock_fs.files.lock().unwrap();
        files.insert(
            PathBuf::from("/mock/root/.claude/settings.json"),
            "user edit\n".to_string(),
        );
    }

    let outputs_v2 = vec![OutputFile::new(".claude/settings.json", "generated v2\n")];
    let options = SyncOptions {
        force: false,
        dry_run: false,
        interactive: true,
        targets: vec![],
    };

    let mut prompter = Prompt;
    let result = sync_with_fs_with_prompter(root, &outputs_v2, &options, &mock_fs, &mut prompter)
        .unwrap();

    assert_eq!(result.written.len(), 0);
    assert_eq!(result.skipped, vec![".claude/settings.json".to_string()]);

    let content = mock_fs
        .read_to_string(Path::new("/mock/root/.claude/settings.json"))
        .unwrap();
    assert_eq!(content, "user edit\n");
}

#[test]
fn test_sync_interactive_diff_then_overwrite() {
    use crate::fs::MockFileSystem;

    struct Prompt {
        step: usize,
        diffs: Vec<String>,
    }
    impl SyncPrompter for Prompt {
        fn prompt_conflict(&mut self, _path: &str, _reason: ConflictReason) -> ConflictChoice {
            let choice = match self.step {
                0 => ConflictChoice::Diff,
                _ => ConflictChoice::Overwrite,
            };
            self.step += 1;
            choice
        }

        fn show_diff(&mut self, diff: &str) {
            self.diffs.push(diff.to_string());
        }
    }

    let mock_fs = MockFileSystem::new();
    let root = Path::new("/mock/root");

    let outputs_v1 = vec![OutputFile::new(".claude/settings.json", "generated v1\n")];
    let options_force = SyncOptions {
        force: true,
        dry_run: false,
        interactive: false,
        targets: vec![],
    };
    sync_with_fs(root, &outputs_v1, &options_force, &mock_fs).unwrap();

    {
        let mut files = mock_fs.files.lock().unwrap();
        files.insert(
            PathBuf::from("/mock/root/.claude/settings.json"),
            "user edit\n".to_string(),
        );
    }

    let outputs_v2 = vec![OutputFile::new(".claude/settings.json", "generated v2\n")];
    let options = SyncOptions {
        force: false,
        dry_run: false,
        interactive: true,
        targets: vec![],
    };

    let mut prompter = Prompt {
        step: 0,
        diffs: Vec::new(),
    };
    sync_with_fs_with_prompter(root, &outputs_v2, &options, &mock_fs, &mut prompter).unwrap();

    assert_eq!(prompter.diffs.len(), 1);
    assert!(prompter.diffs[0].contains("-user edit"));
    assert!(prompter.diffs[0].contains("+generated v2"));
}

#[test]
fn test_sync_interactive_overwrite_all_applies_to_multiple_conflicts() {
    use crate::fs::MockFileSystem;

    struct Prompt {
        calls: usize,
    }
    impl SyncPrompter for Prompt {
        fn prompt_conflict(&mut self, _path: &str, _reason: ConflictReason) -> ConflictChoice {
            self.calls += 1;
            ConflictChoice::OverwriteAll
        }

        fn show_diff(&mut self, _diff: &str) {}
    }

    let mock_fs = MockFileSystem::new();
    let root = Path::new("/mock/root");

    let outputs_v1 = vec![
        OutputFile::new(".claude/a.md", "generated a1\n"),
        OutputFile::new(".claude/b.md", "generated b1\n"),
    ];
    let options_force = SyncOptions {
        force: true,
        dry_run: false,
        interactive: false,
        targets: vec![],
    };
    sync_with_fs(root, &outputs_v1, &options_force, &mock_fs).unwrap();

    {
        let mut files = mock_fs.files.lock().unwrap();
        files.insert(PathBuf::from("/mock/root/.claude/a.md"), "user a\n".to_string());
        files.insert(PathBuf::from("/mock/root/.claude/b.md"), "user b\n".to_string());
    }

    let outputs_v2 = vec![
        OutputFile::new(".claude/a.md", "generated a2\n"),
        OutputFile::new(".claude/b.md", "generated b2\n"),
    ];
    let options = SyncOptions {
        force: false,
        dry_run: false,
        interactive: true,
        targets: vec![],
    };

    let mut prompter = Prompt { calls: 0 };
    sync_with_fs_with_prompter(root, &outputs_v2, &options, &mock_fs, &mut prompter).unwrap();

    // Only the first conflict should prompt when using "all".
    assert_eq!(prompter.calls, 1);

    let a = mock_fs.read_to_string(Path::new("/mock/root/.claude/a.md")).unwrap();
    let b = mock_fs.read_to_string(Path::new("/mock/root/.claude/b.md")).unwrap();
    assert_eq!(a, "generated a2\n");
    assert_eq!(b, "generated b2\n");
}

#[test]
fn test_sync_interactive_abort_returns_error() {
    use crate::fs::MockFileSystem;

    struct Prompt;
    impl SyncPrompter for Prompt {
        fn prompt_conflict(&mut self, _path: &str, _reason: ConflictReason) -> ConflictChoice {
            ConflictChoice::Abort
        }

        fn show_diff(&mut self, _diff: &str) {}
    }

    let mock_fs = MockFileSystem::new();
    let root = Path::new("/mock/root");

    let outputs_v1 = vec![OutputFile::new(".claude/settings.json", "generated v1\n")];
    let options_force = SyncOptions {
        force: true,
        dry_run: false,
        interactive: false,
        targets: vec![],
    };
    sync_with_fs(root, &outputs_v1, &options_force, &mock_fs).unwrap();

    {
        let mut files = mock_fs.files.lock().unwrap();
        files.insert(
            PathBuf::from("/mock/root/.claude/settings.json"),
            "user edit\n".to_string(),
        );
    }

    let outputs_v2 = vec![OutputFile::new(".claude/settings.json", "generated v2\n")];
    let options = SyncOptions {
        force: false,
        dry_run: false,
        interactive: true,
        targets: vec![],
    };

    let mut prompter = Prompt;
    let err = sync_with_fs_with_prompter(root, &outputs_v2, &options, &mock_fs, &mut prompter)
        .expect_err("should abort");
    assert!(err.to_string().to_lowercase().contains("abort"));
}

