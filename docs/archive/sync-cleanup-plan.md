# Sync Module Cleanup Plan

## Current Status (Post-Lockfile Migration)

The `sync/` module contains ~5000 lines of code across 12 files. Many functions have
been migrated to the new layered architecture, but the module remains for backward
compatibility.

## External Dependencies

### Active Usage (Must Keep or Migrate)

| File | Uses | Notes |
|------|------|-------|
| `watcher.rs` | `AssetPipeline`, `ScopePolicy`, `SyncResult`, `SyncEngine` | Watch command |
| `commands/debug.rs` | `Lockfile`, `LockfileNamespace`, `detect_orphans`, `AssetPipeline`, `ScopePolicy` | Debug command |
| `commands/deploy/cmd.rs` | `ScopePolicy` | Deploy command |
| `commands/deploy/bridge.rs` | `SyncResult` | Bridge to new engine |
| `commands/deploy/targets.rs` | `SyncDestination` | Target definitions |
| `ui/views/orphan.rs` | `OrphanFile` | UI display |
| `lib.rs` | `compile_assets`, `SyncEngine`, `SyncEngineOptions`, `SyncOptions`, `SyncResult` | Public API |
| `infrastructure/mod.rs` | `LocalHomeDestination`, `LocalProjectDestination`, `RemoteDestination` | Infrastructure |

### Already Migrated to New Architecture

| Legacy (sync/) | New Location | Status |
|----------------|--------------|--------|
| `LockfileNamespace` | `domain::value_objects::LockfileNamespace` | ✅ Re-exported |
| `lockfile_key()` | `domain::value_objects::lockfile_key()` | ✅ Re-exported |
| `hash_content()` | `domain::value_objects::ContentHash` | ✅ Delegated |
| `OutputFile` | `domain::entities::OutputFile` | ✅ Re-exported |
| `LocalFileSystem` | `infrastructure::fs::LocalFs` | ✅ Migrated |
| `RemoteFileSystem` | `infrastructure::fs::RemoteFs` | ✅ Migrated |

## Cleanup Strategy

### Phase 1: Migrate ScopePolicy (Priority: High)

`ScopePolicy` determines where files are deployed (project vs user home).
This should be part of the domain layer.

**Current Location**: `sync/scope.rs`
**New Location**: `domain/value_objects/scope_policy.rs`

Tasks:
- [ ] Create `domain::value_objects::ScopePolicy`
- [ ] Re-export from `sync/scope.rs`
- [ ] Update usages in `watcher.rs`, `debug.rs`, `cmd.rs`

### Phase 2: Migrate SyncResult (Priority: High)

`SyncResult` is the result type for sync operations. Should be in application layer.

**Current Location**: `sync/mod.rs`
**New Location**: `application/deploy/result.rs` or keep in sync but mark as legacy

Tasks:
- [ ] Create `application::deploy::DeployResult` (or similar)
- [ ] Add conversion from `DeployEvent` stream to `SyncResult`
- [ ] Update `bridge.rs` to use new type

### Phase 3: Migrate AssetPipeline (Priority: Medium)

`AssetPipeline` handles loading and compiling assets. This overlaps with
`AssetRepository` and adapter logic.

**Current Location**: `sync/pipeline.rs`
**New Location**: Functionality split between:
- `infrastructure::repositories::AssetRepository` - loading
- Adapters - compiling

Tasks:
- [ ] Review if `AssetPipeline` is still needed
- [ ] Update `watcher.rs` to use new abstractions
- [ ] Deprecate or remove

### Phase 4: Migrate SyncDestination (Priority: Medium)

`SyncDestination` defines where to deploy. Already have similar abstractions.

**Current Location**: `sync/plan.rs`
**New Location**: Already exists in `application/deploy` context

Tasks:
- [ ] Verify coverage in new engine
- [ ] Update `targets.rs` to use new abstractions

### Phase 5: Migrate OrphanFile and detect_orphans (Priority: Low)

Orphan detection is handled by `domain::services::orphan_detector`.

**Current Location**: `sync/orphan.rs`
**New Location**: `domain::services::orphan_detector`

Tasks:
- [ ] Ensure feature parity
- [ ] Update `ui/views/orphan.rs`
- [ ] Deprecate old module

### Phase 6: Cleanup engine.rs (Priority: Low)

The largest file (1597 lines). Contains `SyncEngine` which is the legacy sync engine.
The new `DeployUseCase` replaces this.

Tasks:
- [ ] Mark as deprecated
- [ ] Update `watcher.rs` to use new engine
- [ ] Eventually remove

## Recommended Execution Order

1. **ScopePolicy** - Simple value object, low risk
2. **SyncResult** - Used by bridge, important for compatibility  
3. **AssetPipeline** - Complex, but needed for watcher
4. **SyncDestination** - Already mostly migrated
5. **OrphanFile** - UI only, low priority
6. **engine.rs** - Final cleanup after watcher migrated

## Success Criteria

- All public types from `lib.rs` either:
  - Migrated to new architecture with re-exports for compatibility
  - Marked as deprecated with clear migration path
- `watcher.rs` uses new engine
- `debug.rs` uses domain types
- Test coverage maintained

## Notes

- Keep backward compatibility via re-exports
- Use TDD for all migrations
- Don't break existing commands

