# TDD Session: SC-6 Orphan Cleanup Implementation

> **Date**: 2025-12-19  
> **Feature**: Orphan file cleanup in deploy command  
> **Status**: Complete ✅

---

## Requirements Covered

1. **CLI Flag**: Add `--cleanup` flag to `calvin deploy`
2. **Delete Function**: Safely delete orphan files with Calvin signature
3. **Force Mode**: `--force` flag deletes all orphans regardless of signature
4. **Interactive Mode**: Ask user for confirmation before deletion
5. **JSON Support**: Emit orphan events for CI/CD
6. **Lockfile Update**: Remove deleted entries from lockfile

---

## TDD Cycles

### Cycle 1: delete_orphans Function

**RED**: 3 failing tests
- `delete_orphans_deletes_safe_files_only`
- `delete_orphans_force_deletes_all`
- `delete_orphans_skips_nonexistent`

**GREEN**: Implemented `delete_orphans()` in `src/sync/orphan.rs`
- Added `DeleteResult` struct
- Added `FileSystem::remove_file()` trait method
- Implemented for `LocalFileSystem`, `RemoteFileSystem`, `MockFileSystem`

**REFACTOR**: None needed

### Cycle 2: Orphan View Renderer

**RED**: 5 failing tests
- `orphan_list_shows_safe_and_unsafe`
- `orphan_list_shows_not_found`
- `orphan_summary_shows_deleted_count`
- `orphan_summary_shows_skipped_count`
- `orphan_summary_shows_both`

**GREEN**: Created `src/ui/views/orphan.rs`
- `render_orphan_list()` - displays orphan files with status icons
- `render_orphan_summary()` - shows deletion summary

**REFACTOR**: Fixed hardcoded UI token in doc comment

### Cycle 3: cmd_deploy Integration

**GREEN**: Integrated cleanup logic into `cmd_deploy()`
- Orphan detection after successful deploy
- Interactive confirmation using `dialoguer`
- JSON event emission
- Lockfile update after deletion

**REFACTOR**: 
- Fixed clippy warnings (`is_empty()` vs `len() > 0`)
- Simplified match with `unwrap_or_default()`

---

## Implementation Decisions

### 1. When to Skip Cleanup

- Remote targets: SSH deletion is complex, skipped for now
- Dry run mode: No actual deletion
- Failed deploys: Skip orphan detection

### 2. Cleanup Modes

| Mode | Behavior |
|------|----------|
| Default | Warn only, don't delete |
| Interactive | Show list, ask confirmation |
| `--cleanup` | Auto-delete safe files |
| `--cleanup --force` | Delete all orphans |

### 3. Safety Checks

- Only delete files with Calvin signature by default
- Check file existence before attempting deletion
- Continue on individual file errors (don't fail entire operation)

---

## Files Modified/Created

| File | Change |
|------|--------|
| `src/cli.rs` | Added `--cleanup` flag to Deploy command |
| `src/main.rs` | Pass cleanup parameter to cmd_deploy |
| `src/commands/deploy/cmd.rs` | Integrated orphan cleanup logic |
| `src/commands/interactive.rs` | Pass cleanup=false to cmd_deploy |
| `src/sync/orphan.rs` | Added `delete_orphans()`, `DeleteResult` |
| `src/fs.rs` | Added `remove_file()` to FileSystem trait |
| `src/ui/views/orphan.rs` | **NEW** - Orphan list and summary renderers |
| `src/ui/views/mod.rs` | Added orphan module |

---

## Tests Added

### Unit Tests (8 new)
1. `delete_orphans_deletes_safe_files_only`
2. `delete_orphans_force_deletes_all`
3. `delete_orphans_skips_nonexistent`
4. `orphan_list_shows_safe_and_unsafe`
5. `orphan_list_shows_not_found`
6. `orphan_summary_shows_deleted_count`
7. `orphan_summary_shows_skipped_count`
8. `orphan_summary_shows_both`

### CLI Tests (2 new)
1. `test_cli_parse_deploy` (updated)
2. `test_cli_parse_deploy_cleanup`

---

## Test Results

- **Total Tests**: 346
- **Passed**: 346
- **Failed**: 0
- **Clippy Warnings**: 0

---

## Usage Examples

```bash
# Default: warn about orphans
calvin deploy
# Output: ⚠ 3 orphan file(s) detected. Run with --cleanup to remove.

# Auto-delete safe files
calvin deploy --cleanup

# Delete all orphans (including without signature)
calvin deploy --cleanup --force

# Interactive mode asks for confirmation
calvin deploy
# Shows orphan list, prompts: "Delete 2 file(s) with Calvin signature? [y/N]"
```
