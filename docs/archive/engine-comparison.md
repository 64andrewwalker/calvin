# New Engine vs Legacy Engine Comparison

## Overview

| Feature | New Engine (DeployUseCase) | Legacy Engine (DeployRunner) |
|---------|---------------------------|------------------------------|
| Location | `src/application/deploy.rs` | `src/commands/deploy/runner.rs` |
| Architecture | Clean layered (Domain/Application/Infrastructure) | Monolithic |
| File System | Uses `FileSystem` port | Uses `LocalFileSystem` / `RemoteFileSystem` directly |

## Feature Comparison

### ✅ Both Engines Support

| Feature | New Engine | Legacy Engine |
|---------|------------|---------------|
| Local project deploy | ✅ | ✅ |
| Local home deploy (`--home`) | ✅ | ✅ |
| Force mode (`--force`) | ✅ | ✅ |
| Dry run (`--dry-run`) | ✅ | ✅ |
| Target filtering (`--target`) | ✅ | ✅ |
| Conflict detection | ✅ (Planner service) | ✅ (plan_sync) |
| Lockfile management | ✅ (LockfileRepository) | ✅ (Lockfile module) |
| Orphan detection | ✅ (OrphanDetector service) | ✅ (orphan module) |
| Atomic writes | ✅ (via LocalFs) | ✅ (via writer module) |
| Home directory expansion (`~`) | ✅ (via LocalFs) | ✅ (via LocalFileSystem) |

### ⚠️ Different Implementation

| Feature | New Engine | Legacy Engine | Notes |
|---------|------------|---------------|-------|
| **Interactive conflict resolution** | ✅ (ConflictResolver port) | ✅ (resolve_conflicts_interactive) | New engine uses port abstraction |
| **JSON event streaming** | ✅ (DeployEventSink port + JsonEventSink) | ✅ (SyncEvent callback) | New engine uses port abstraction |
| **AGENTS.md generation** | ✅ (via post_compile) | ✅ (via post_compile) | Both call adapter.post_compile() |

### ❌ New Engine Gaps

| Feature | New Engine | Legacy Engine | Impact |
|---------|------------|---------------|--------|
| **Remote deploy (rsync)** | ⚠️ Port defined, not wired | ✅ Full support | **HIGH** - Remote deploys will fail |
| **Remote deploy (SSH)** | ⚠️ Port defined, not wired | ✅ Full support | **HIGH** - Remote deploys will fail |
| **rsync batch optimization** | ⚠️ Implemented in SyncDestination, not yet wired | ✅ (for >10 files) | MEDIUM - Performance |
| **Remote conflict resolution** | ❌ | ✅ (via RemoteFileSystem) | HIGH - No remote conflicts |

## Detailed Gap Analysis

### Remote Deployment (CRITICAL)

**Legacy Engine:**
```rust
// runner.rs line 95-100
match &self.target {
    DeployTarget::Project(_) | DeployTarget::Home => {
        self.sync_local_with_engine(outputs, callback)
    }
    DeployTarget::Remote(_) => self.sync_remote(outputs, callback),
}
```

The legacy engine has dedicated code paths for:
1. `sync_remote()` - Full planning + conflict resolution over SSH
2. `sync_remote_fast()` - rsync batch transfer for force mode

**New Engine:**
```rust
// application/deploy.rs - uses LocalFs directly
let file_system = LocalFs::new();
```

The new engine:
- Only uses `LocalFs` for file operations
- `SyncDestination` traits are defined but NOT integrated into `DeployUseCase`
- `RemoteDestination` exists but is never used

### Impact Assessment

1. **Remote deploys (`--remote host:path`)** - ❌ **BROKEN**
   - Bridge passes `Scope::Project` but doesn't handle remote case
   - `LocalFs` cannot write to remote hosts

2. **Performance for large deploys** - ⚠️ **Degraded**
   - Legacy engine uses rsync for >10 files
   - New engine writes file-by-file

## Recommendation

### Option A: Complete Remote Support in New Engine

1. Add `SyncDestination` parameter to `DeployUseCase`
2. Create factory method for remote destinations
3. Wire `RemoteDestination` implementation

### Option B: Keep Legacy Engine for Remote

1. Remove `DeployTarget::Remote` from bridge conversion
2. Always use legacy engine for remote deploys
3. Mark as known limitation until v0.4.0

### Option C: Route Remote to Legacy Engine (**IMPLEMENTED**)

Current implementation (as of this commit):
- Remote targets (`--remote host:path`) always use legacy DeployRunner
- Local targets (project/home) use new engine by default
- This ensures remote deploys work correctly via SSH/rsync

## Files to Review

- `src/commands/deploy/bridge.rs` - Conversion logic
- `src/application/deploy.rs` - New engine
- `src/commands/deploy/runner.rs` - Legacy engine
- `src/domain/ports/sync_destination.rs` - Unused port
- `src/infrastructure/sync/remote.rs` - Unused implementation

