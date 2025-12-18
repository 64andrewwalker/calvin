# Sync Unification - TODO Tracker

> Status: Ready for TDD
> Total Estimated: 4-5 hours
> Start Date: TBD
> Completion Date: TBD

## Overview

Unify sync logic across all code paths (deploy, watch) to:
1. Use two-stage sync (plan → execute) everywhere
2. Enable hash-based incremental detection for all destinations
3. Consistent lockfile handling
4. Clean UI output (no raw rsync)

## Phase 1: Create SyncEngine (1.5-2 hours)

### 1.1 Create engine.rs structure
- [ ] Create `src/sync/engine.rs`
- [ ] Define `SyncEngineOptions` struct
- [ ] Define `SyncEngine` struct with fields
- [ ] Implement `SyncEngine::new()`
- [ ] Implement convenience constructors: `local()`, `home()`, `remote()`

### 1.2 Implement planning
- [ ] Implement `SyncEngine::plan()` 
  - [ ] Load lockfile from lockfile_path
  - [ ] Dispatch to `plan_sync` or `plan_sync_remote` based on destination

### 1.3 Implement execution
- [ ] Implement `SyncEngine::execute()`
- [ ] Implement `SyncEngine::execute_with_callback()`
- [ ] Implement `select_strategy()` (rsync vs file-by-file)
- [ ] Implement `update_lockfile()`

### 1.4 Implement convenience method
- [ ] Implement `SyncEngine::sync()` 
- [ ] Implement `SyncEngine::sync_with_callback()`
- [ ] Handle force/interactive/skip resolution

### 1.5 Tests (TDD - write first!)
- [ ] `engine_local_skips_unchanged_files`
- [ ] `engine_local_writes_new_files`
- [ ] `engine_local_detects_modified_files`
- [ ] `engine_local_updates_lockfile`
- [ ] `engine_force_overwrites_conflicts`
- [ ] `engine_home_uses_source_lockfile`
- [ ] `engine_select_strategy_rsync_for_many_files`

### 1.6 Export from mod.rs
- [ ] Add `pub mod engine;` to `src/sync/mod.rs`
- [ ] Add re-exports: `pub use engine::{SyncEngine, SyncEngineOptions};`

## Phase 2: Optimize Local Planning (30-45 min)

### 2.1 Add mtime optimization (optional, can skip if time-constrained)
- [ ] Add mtime field to lockfile entries
- [ ] Check mtime before computing hash
- [ ] Only hash if mtime changed

### 2.2 Add plan_sync_local (if needed)
- [ ] Evaluate if `plan_sync` already handles local well
- [ ] If not, add optimized `plan_sync_local` to plan.rs

## Phase 3: Update Watcher (45-60 min)

### 3.1 Modify perform_sync_incremental
- [ ] Import SyncEngine
- [ ] Replace rsync/sync_outputs calls with SyncEngine
- [ ] Pass `force: true` for watch behavior

### 3.2 Update watch command output
- [ ] Ensure "X written, Y skipped" shown instead of rsync output
- [ ] Update WatchEvent if needed

### 3.3 Tests
- [ ] Run existing watcher tests
- [ ] Add test: `watch_sync_skips_unchanged_files`
- [ ] Add test: `watch_uses_lockfile`

## Phase 4: Simplify DeployRunner (30-45 min)

### 4.1 Refactor sync_outputs
- [ ] Replace current implementation with SyncEngine
- [ ] Remove `sync_remote_fast` (SyncEngine handles it)
- [ ] Remove `load_lockfile`, `update_lockfile` methods (SyncEngine does it)
- [ ] Remove `select_strategy` method (SyncEngine does it)
- [ ] Remove `plan_sync`, `resolve_conflicts` methods (use SyncEngine)

### 4.2 Handle interactive mode
- [ ] SyncEngine.sync() with interactive needs prompter
- [ ] Either pass prompter to SyncEngine or resolve at caller level

### 4.3 Tests
- [ ] Run existing deploy tests
- [ ] Verify NDJSON output still works
- [ ] Verify remote deploy still works

## Phase 5: Clean Up (15-30 min)

### 5.1 Deprecate old API
- [ ] Add `#[deprecated]` to `sync_outputs()`
- [ ] Add `#[deprecated]` to `sync_with_fs()`
- [ ] Add deprecation warning message

### 5.2 Update documentation
- [ ] Update `docs/tdd-session-sync-refactor.md`
- [ ] Mark this TODO as complete

### 5.3 Final testing
- [ ] Run all tests: `cargo test`
- [ ] Manual test: `calvin deploy` to project
- [ ] Manual test: `calvin deploy --home`
- [ ] Manual test: `calvin deploy --remote ubuntu-server:~`
- [ ] Manual test: `calvin watch`
- [ ] Verify rsync output suppressed in non-verbose mode

## Acceptance Criteria

### Functional
- [ ] Local deploy shows "X written, Y skipped" (not raw rsync)
- [ ] Watch mode shows "X written, Y skipped" 
- [ ] Unchanged files are skipped (verified via lockfile)
- [ ] Remote deploy still works with batch planning
- [ ] Interactive conflict resolution still works
- [ ] NDJSON output for CI still works

### Performance
- [ ] Local deploy with 150 files and 0 changes: < 1s
- [ ] Watch initial sync: same or better than before
- [ ] Remote deploy: no regression

### Code Quality
- [ ] Net code reduction (target: -50 lines or more)
- [ ] Single code path for all destinations
- [ ] All tests pass (295+)

## Notes

- Keep `force: true` in watch mode for now (don't add conflict detection to watch)
- Lockfile for home deploy lives in `.promptpack/.calvin.lock` (source), not `~/.calvin.lock`
- SyncEngine should be usable standalone (not tied to DeployRunner)

## Blockers

None identified.

## Dependencies

- Phase 2 depends on Phase 1
- Phase 3 depends on Phase 1
- Phase 4 depends on Phase 1
- Phase 5 depends on all above

