# SyncEngine Migration Plan

## Current Status

**File**: `src/sync/engine.rs`  
**Size**: 1597 lines (492 impl, 1105 tests)  
**Main User**: `watcher.rs`

## SyncEngine Overview

### Purpose
`SyncEngine` provides a unified API for sync operations:
- Local project sync
- Home directory sync
- Remote SSH sync
- Watch mode sync

### Key Methods

| Method | Purpose |
|--------|---------|
| `local()` | Create engine for project directory |
| `home()` | Create engine for user home |
| `remote()` | Create engine for SSH remote |
| `plan()` | Stage 1: Detect changes and conflicts |
| `execute()` | Stage 2: Transfer files |
| `sync()` | Combined plan + execute |
| `sync_with_resolver()` | Sync with custom conflict resolver |

### Dependencies

- `plan_sync_with_namespace` from `sync/plan.rs`
- `execute_sync_with_callback` from `sync/execute.rs`
- `Lockfile` from `sync/lockfile.rs`
- `LocalFs`, `RemoteFs` from `infrastructure/fs/`

## Comparison with DeployUseCase

| Feature | SyncEngine | DeployUseCase |
|---------|------------|---------------|
| Local deploy | ✅ | ✅ |
| Home deploy | ✅ | ✅ |
| Remote deploy | ✅ | ✅ |
| Conflict detection | ✅ | ✅ |
| Interactive conflict resolution | ✅ (via resolver) | ✅ |
| Orphan detection | ❌ | ✅ |
| Event stream (JSON) | ❌ | ✅ |
| Watch mode support | ✅ (incremental) | ❌ (batch only) |

### Key Difference

**SyncEngine**:
- Works with `OutputFile[]` directly (already compiled)
- Designed for incremental updates in watch mode
- Has callback-based progress reporting

**DeployUseCase**:
- Loads assets, compiles, plans, and executes
- Full pipeline for CLI deploy command
- Has event-based progress reporting

## Migration Strategy

### Option A: Adapt Watcher to Use DeployUseCase

**Pros**:
- One code path for deployment
- Orphan detection in watch mode
- Event stream support

**Cons**:
- DeployUseCase does full reload every time
- May be slower for watch mode (incremental vs full)
- Need to adapt watcher's output handling

### Option B: Keep SyncEngine for Watcher (Recommended)

**Pros**:
- SyncEngine is already optimized for incremental sync
- Minimal changes required
- Tests already exist

**Cons**:
- Two deployment code paths to maintain
- Some duplication

### Recommended Approach

**Short-term** (Option B):
1. Mark `SyncEngine` as internal (not exported in `lib.rs`)
2. Keep it for watcher use only
3. Add deprecation notice

**Long-term** (Option A):
1. Add incremental support to DeployUseCase
2. Migrate watcher to use DeployUseCase
3. Delete SyncEngine

## Action Items

### Phase 1: Documentation (Now)
- [ ] Document SyncEngine's role and usage
- [ ] Document relationship with DeployUseCase

### Phase 2: Cleanup (Optional)
- [ ] Remove SyncEngine from public exports
- [ ] Mark as deprecated
- [ ] Move tests to integration test if needed

### Phase 3: Watcher Migration (Future)
- [ ] Add incremental asset detection to AssetRepository
- [ ] Add watch mode support to DeployUseCase
- [ ] Migrate watcher.rs to use DeployUseCase
- [ ] Delete SyncEngine

## Watcher Migration Details

Current watcher flow:
```
1. Watch for file changes
2. Re-parse changed assets
3. Compile to OutputFile[]
4. SyncEngine.sync(outputs)
```

New watcher flow (future):
```
1. Watch for file changes
2. Create DeployOptions with changed files hint
3. DeployUseCase.execute()
4. Handle DeployEvents for UI updates
```

### Watcher Dependencies on SyncEngine

From `watcher.rs`:
```rust
use crate::sync::engine::{SyncEngine, SyncEngineOptions};

let engine_options = SyncEngineOptions {
    force: false,
    interactive: false,
    dry_run: false,
    verbose: false,
};

if options.user_level {
    SyncEngine::home(&outputs, options.source.clone(), engine_options)
} else {
    SyncEngine::local(&outputs, options.project_root.clone(), engine_options)
}
```

Key requirements for watcher:
1. Incremental sync (only changed files)
2. Non-interactive mode
3. Local or home target
4. Progress callback for UI

## Conclusion

`SyncEngine` is well-designed and tested. The recommended approach is:
1. Keep it for watcher
2. Don't export it publicly
3. Migrate in future when DeployUseCase supports watch mode

This is a **low priority** cleanup - the code is stable and working.

