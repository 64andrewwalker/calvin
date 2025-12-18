# TDD Session: Sync Engine Refactor

> Date: 2024-12-18
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

## Refactoring Notes

### Phase 1: plan.rs ✅
- Created SyncPlan, Conflict, ConflictReason
- Created SyncDestination enum
- Implemented plan_sync() with lockfile checks

### Phase 2: execute.rs ✅
- Created SyncStrategy enum
- Implemented execute_sync() with rsync/file-by-file selection
- Added callback support for progress UI

### Phase 3: deploy.rs integration (WIP)
- Created sync_runner.rs helper module
- Need to refactor deploy.rs Remote branch to use new API
- Key benefit: After "overwrite all", rsync batch transfer

## Files Changed

```
src/sync/plan.rs         - NEW (SyncPlan, plan_sync, resolve_conflicts_interactive)
src/sync/execute.rs      - NEW (execute_sync, SyncStrategy)
src/sync/mod.rs          - Export new types
src/sync/lockfile.rs     - Added hash_content(), get(), entries()
src/commands/sync_runner.rs - NEW (TwoStageSyncOptions, run_two_stage_sync)
src/commands/mod.rs      - Added sync_runner module
docs/design/sync-engine-refactor.md - Design documentation
```

## Next Steps

1. [ ] Modify deploy.rs Remote branch to use two-stage API
2. [ ] Test with real remote deployment
3. [ ] Measure performance improvement (rsync vs SSH per-file)
