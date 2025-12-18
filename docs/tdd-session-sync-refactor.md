# TDD Session: Sync Engine Refactor

> Date: 2024-12-18
> Status: ✅ Complete
> Feature: Two-stage sync for deploy command

## Requirements Covered

1. **Separate conflict detection from file transfer**
   - `plan_sync()` - stage 1: detect conflicts without writing
   - `execute_sync()` - stage 2: batch transfer

2. **Interactive conflict resolution**
   - `resolve_conflicts_interactive()` - prompt user for each conflict
   - After "overwrite all", use rsync batch transfer

3. **Automatic strategy selection**
   - Use rsync for 10+ files when available
   - Fall back to file-by-file when callback needed

4. **Remote deployment optimization**
   - `plan_sync_remote()` - batch SSH file status checks
   - Single SSH call for all file existence + hash checks

## Tests Written

### sync::plan (7 tests)
- `plan_new_files_go_to_write` - new files have no conflicts
- `plan_untracked_existing_files_are_conflicts` - untracked files need resolution
- `plan_modified_tracked_files_are_conflicts` - modified files need resolution
- `plan_unmodified_tracked_files_can_overwrite` - unchanged tracked files safe
- `overwrite_all_moves_conflicts_to_write` - SyncPlan.overwrite_all()
- `skip_all_moves_conflicts_to_skip` - SyncPlan.skip_all()
- `total_files_counts_all_categories` - SyncPlan.total_files()

### sync::execute (5 tests)
- `execute_empty_plan_returns_empty_result` - empty plan handling
- `execute_writes_files_to_local` - basic file writing
- `execute_with_callback_emits_events` - callback receives events
- `execute_preserves_skipped_files` - skipped list preserved
- `auto_strategy_uses_rsync_for_many_files` - strategy selection

### commands::deploy (2 tests)
- `runner_with_project_target` - local deployment
- `runner_with_remote_target` - remote deployment

## Implementation Decisions

### SyncPlan as Immutable Value
- `overwrite_all()` and `skip_all()` consume self and return new plan
- Makes conflict resolution explicit without side effects

### Conflict Detection Uses FileSystem Trait
- Allows testing with MockFileSystem
- Works for both local and remote destinations

### resolve_conflicts_interactive Reuses Existing Prompt Logic
- Uses same o/s/d/a/A key bindings from conflict.rs
- Generates unified diff for 'd' option

### Remote Path Expansion
- `~` paths must be expanded before file operations
- `expand_home()` called in both plan and execute phases
- Critical: single-quoted paths don't expand `~` in shell

## Refactoring Summary

### Phase 1: plan.rs ✅
- Created SyncPlan, Conflict, ConflictReason
- Created SyncDestination enum
- Implemented plan_sync() with lockfile checks

### Phase 2: execute.rs ✅
- Created SyncStrategy enum
- Implemented execute_sync() with rsync/file-by-file selection
- Added callback support for progress UI

### Phase 3: deploy module ✅
- Replaced deploy.rs (~1000 lines) with modular deploy/ directory
- New structure: cmd.rs, runner.rs, options.rs, targets.rs
- DeployRunner handles two-stage sync flow

### Phase 4: Remote optimization ✅
- Added `plan_sync_remote()` with batch SSH checks
- Added `RemoteFileSystem::batch_check_files()`
- Fixed path expansion bug in execute phase

## Files Changed

```
src/sync/plan.rs           - SyncPlan, plan_sync, plan_sync_remote, resolve_conflicts_interactive
src/sync/execute.rs        - execute_sync, SyncStrategy (with expand_home fix)
src/sync/mod.rs            - Export new types
src/sync/lockfile.rs       - Added hash_content() with SHA-256

src/commands/deploy/       - NEW modular structure
├── cmd.rs                 - Command entry point, NDJSON output
├── runner.rs              - DeployRunner with two-stage sync
├── options.rs             - DeployOptions
├── targets.rs             - DeployTarget, ScopePolicy
└── mod.rs                 - Module exports

src/fs.rs                  - Added batch_check_files(), expand_home()

DELETED:
- src/commands/deploy.rs   - Old monolithic implementation
- src/commands/sync_runner.rs - Replaced by deploy/runner.rs
```

## Bug Fixes

### Remote Path Expansion Bug ✅
**Problem**: In execute phase, `~` paths were not expanded, causing files to be written to wrong location.

**Root Cause**:
- `plan_sync_remote()` called `fs.expand_home(path)` → `/home/user/.agent/...`
- `execute_sync()` used raw `path` → `~/.agent/...`
- `quote_path()` uses single quotes → `'~/.agent/...'`
- Shell doesn't expand `~` in single quotes

**Fix**: Added `fs.expand_home(path)` in `execute_sync()` for remote destinations.

### Hash Consistency ✅
**Problem**: Local and remote hash calculations used different algorithms.

**Fix**: Standardized on SHA-256 with `sha256:` prefix format:
- `lockfile::hash_content()` - uses sha2 crate
- `batch_check_files()` - uses sha256sum/shasum on remote

## Performance

| Operation | Before | After |
|-----------|--------|-------|
| Remote planning (147 files) | ~147 SSH calls | 1 SSH call |
| Batch transfer (10+ files) | file-by-file | rsync |
| Conflict detection | During write | Before write |

## Metrics

- **Code reduction**: ~1000 lines → ~500 lines (50% reduction)
- **Tests**: 64 total, all passing
- **Remote deployment**: Works correctly in interactive mode
