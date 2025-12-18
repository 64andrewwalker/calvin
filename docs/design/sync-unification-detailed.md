# Sync Logic Unification - Detailed Design

> Date: 2024-12-18
> Status: TDD Ready
> Estimated: 4-5 hours total

## Current State Analysis

### File Structure (Current)

```
src/
├── sync/
│   ├── mod.rs           # sync_outputs(), sync_with_fs() - OLD file-by-file API
│   ├── plan.rs          # plan_sync(), plan_sync_remote() - NEW planning API
│   ├── execute.rs       # execute_sync() - NEW execution API
│   ├── lockfile.rs      # Lockfile management
│   ├── writer.rs        # atomic_write(), hash_content()
│   ├── remote.rs        # sync_remote_rsync(), sync_local_rsync()
│   ├── compile.rs       # compile_assets()
│   └── conflict.rs      # Conflict prompts
├── commands/
│   └── deploy/
│       ├── runner.rs    # DeployRunner - uses NEW two-stage API
│       └── cmd.rs       # Entry point
├── watcher.rs           # Uses OLD sync_outputs() or sync_local_rsync()
└── fs.rs                # FileSystem trait
```

### Code Path Analysis

#### Path 1: Deploy Remote (Interactive/Force)
```
cmd_deploy → DeployRunner.sync_outputs()
  → plan_sync_remote() [batch SSH check, 1 call]
  → resolve_conflicts_interactive() [if interactive]
  → execute_sync_with_callback() [rsync or file-by-file]
  → update_lockfile()

✅ Uses two-stage sync
✅ Hash-based incremental detection
✅ Lockfile updated correctly
✅ Clean UI ("X written, Y skipped")
```

#### Path 2: Deploy Remote (Force)
```
cmd_deploy → DeployRunner.sync_outputs()
  → sync_remote_fast() [skip planning]
  → sync_remote_rsync() [rsync batch]
  → update_lockfile()

⚠️ Skips planning (intentional for speed)
⚠️ Always transfers all files (rsync optimizes internally)
⚠️ Raw rsync output shown
```

#### Path 3: Deploy Local/Home
```
cmd_deploy → DeployRunner.sync_outputs()
  → plan_sync() [uses LocalFileSystem]
  → execute_sync_with_callback()
    → select_strategy() → SyncStrategy::Rsync (if > 10 files)
    → sync_local_rsync() [rsync batch]

⚠️ plan_sync() works but...
⚠️ execute → rsync ignores plan.to_skip!
⚠️ Raw rsync output shown
⚠️ Lockfile updated only for files in result.written
```

#### Path 4: Watch Mode
```
cmd_watch → watcher::watch()
  → perform_sync_incremental()
    → parse_incremental() [reparse changed files]
    → compile_assets()
    → if has_rsync && len > 10:
        sync_local_rsync() [rsync batch]
      else:
        sync_outputs() [OLD API, file-by-file]

❌ No two-stage sync
❌ No hash-based skip (rsync uses mtime)
❌ No lockfile integration
❌ Raw rsync output shown
❌ Reports "147 written" even when 0 changed
```

### Key Problems

1. **sync_local_rsync ignores SyncPlan**
   - `execute.rs` calls `sync_local_rsync(&plan.to_write, ...)` ✅
   - But `watcher.rs` calls `sync_local_rsync(&outputs, ...)` - all files!

2. **Watch doesn't use lockfile**
   - `IncrementalCache` only tracks source file changes
   - Doesn't track if destination files are up-to-date

3. **sync_outputs (OLD) and execute_sync (NEW) both exist**
   - Some callers use old API, some use new
   - Inconsistent behavior

## Proposed File Structure

```
src/
├── sync/
│   ├── mod.rs           # Re-exports, deprecate sync_outputs()
│   ├── engine.rs        # NEW: SyncEngine (unified API)
│   ├── plan.rs          # plan_sync(), plan_sync_local(), plan_sync_remote()
│   ├── execute.rs       # execute_sync() (keep as-is)
│   ├── lockfile.rs      # Lockfile (keep as-is)
│   ├── writer.rs        # atomic_write(), hash_content() (keep as-is)
│   ├── remote.rs        # rsync helpers (keep as-is)
│   ├── compile.rs       # compile_assets() (keep as-is)
│   └── conflict.rs      # Conflict prompts (keep as-is)
├── commands/
│   └── deploy/
│       ├── runner.rs    # Simplify: use SyncEngine
│       └── cmd.rs       # Entry point (keep as-is)
├── watcher.rs           # Use SyncEngine
└── fs.rs                # FileSystem trait (keep as-is)
```

## Implementation Plan

### Phase 1: Create SyncEngine (NEW FILE)

**File**: `src/sync/engine.rs`

```rust
//! Unified sync engine for all destinations
//!
//! Provides a single API for:
//! - Local project sync
//! - Home directory sync
//! - Remote SSH sync
//! - Watch mode sync

use std::path::{Path, PathBuf};
use crate::adapters::OutputFile;
use crate::error::CalvinResult;
use crate::sync::{
    SyncPlan, SyncDestination, SyncResult, SyncEvent, SyncStrategy,
    Lockfile, plan_sync, plan_sync_remote, execute_sync_with_callback,
};
use crate::fs::{FileSystem, LocalFileSystem, RemoteFileSystem};

/// Options for SyncEngine
#[derive(Debug, Clone, Default)]
pub struct SyncEngineOptions {
    /// Force overwrite without checking conflicts
    pub force: bool,
    /// Prompt user for conflict resolution
    pub interactive: bool,
    /// Don't actually write files
    pub dry_run: bool,
    /// Show verbose transfer output (rsync details)
    pub verbose: bool,
    /// Suppress rsync raw output (use summary only)
    pub quiet_transfer: bool,
}

/// Unified sync engine
pub struct SyncEngine<'a> {
    outputs: &'a [OutputFile],
    destination: SyncDestination,
    lockfile_path: PathBuf,
    options: SyncEngineOptions,
}

impl<'a> SyncEngine<'a> {
    pub fn new(
        outputs: &'a [OutputFile],
        destination: SyncDestination,
        lockfile_path: PathBuf,
        options: SyncEngineOptions,
    ) -> Self {
        Self { outputs, destination, lockfile_path, options }
    }
    
    /// Create for local destination
    pub fn local(outputs: &'a [OutputFile], root: PathBuf, options: SyncEngineOptions) -> Self {
        let lockfile_path = root.join(".promptpack/.calvin.lock");
        Self::new(outputs, SyncDestination::Local(root), lockfile_path, options)
    }
    
    /// Create for home destination  
    pub fn home(outputs: &'a [OutputFile], source: &Path, options: SyncEngineOptions) -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let lockfile_path = source.join(".calvin.lock");
        Self::new(outputs, SyncDestination::Local(home), lockfile_path, options)
    }
    
    /// Create for remote destination
    pub fn remote(outputs: &'a [OutputFile], host: String, path: PathBuf, source: &Path, options: SyncEngineOptions) -> Self {
        let lockfile_path = source.join(".calvin.lock");
        Self::new(outputs, SyncDestination::Remote { host, path }, lockfile_path, options)
    }
    
    /// Stage 1: Plan sync (detect changes and conflicts)
    pub fn plan(&self) -> CalvinResult<SyncPlan> {
        let fs = LocalFileSystem;
        let lockfile = Lockfile::load_or_new(&self.lockfile_path, &fs);
        
        match &self.destination {
            SyncDestination::Local(_) => {
                plan_sync(self.outputs, &self.destination, &lockfile, &fs)
            }
            SyncDestination::Remote { host, .. } => {
                let remote_fs = RemoteFileSystem::new(host);
                plan_sync_remote(self.outputs, &self.destination, &lockfile, &remote_fs)
            }
        }
    }
    
    /// Stage 2: Execute sync plan
    pub fn execute(&self, plan: SyncPlan) -> CalvinResult<SyncResult> {
        self.execute_with_callback::<fn(SyncEvent)>(plan, None)
    }
    
    /// Stage 2: Execute with progress callback
    pub fn execute_with_callback<F>(
        &self,
        plan: SyncPlan,
        callback: Option<F>,
    ) -> CalvinResult<SyncResult>
    where
        F: FnMut(SyncEvent),
    {
        if self.options.dry_run {
            return Ok(SyncResult {
                written: plan.to_write.iter().map(|o| o.path.display().to_string()).collect(),
                skipped: plan.to_skip,
                errors: vec![],
            });
        }
        
        let strategy = self.select_strategy(&plan);
        let result = execute_sync_with_callback(&plan, &self.destination, strategy, callback)?;
        
        // Update lockfile
        self.update_lockfile(&plan.to_write, &result);
        
        Ok(result)
    }
    
    /// Convenience: plan → auto-resolve → execute
    pub fn sync(&self) -> CalvinResult<SyncResult> {
        self.sync_with_callback::<fn(SyncEvent)>(None)
    }
    
    /// Convenience: plan → auto-resolve → execute with callback
    pub fn sync_with_callback<F>(&self, callback: Option<F>) -> CalvinResult<SyncResult>
    where
        F: FnMut(SyncEvent),
    {
        let plan = self.plan()?;
        
        let resolved = if self.options.force {
            plan.overwrite_all()
        } else if plan.conflicts.is_empty() {
            plan
        } else if self.options.interactive {
            // Interactive resolution (would need to pass prompter)
            // For now, skip conflicts in non-interactive
            plan.skip_all()
        } else {
            plan.skip_all()
        };
        
        self.execute_with_callback(resolved, callback)
    }
    
    fn select_strategy(&self, plan: &SyncPlan) -> SyncStrategy {
        if plan.to_write.len() > 10 && crate::sync::remote::has_rsync() {
            SyncStrategy::Rsync
        } else {
            SyncStrategy::FileByFile
        }
    }
    
    fn update_lockfile(&self, outputs: &[OutputFile], result: &SyncResult) {
        use std::collections::HashSet;
        
        let fs = LocalFileSystem;
        let mut lockfile = Lockfile::load_or_new(&self.lockfile_path, &fs);
        let written_set: HashSet<&str> = result.written.iter().map(|s| s.as_str()).collect();
        
        for output in outputs {
            let path_str = output.path.display().to_string();
            if written_set.contains(path_str.as_str()) {
                let hash = crate::sync::lockfile::hash_content(&output.content);
                lockfile.set_hash(&path_str, &hash);
            }
        }
        
        let _ = lockfile.save(&self.lockfile_path, &fs);
    }
}
```

### Phase 2: Add plan_sync for Local

**File**: `src/sync/plan.rs` (add function)

```rust
/// Plan sync for local destinations (optimized)
///
/// Uses mtime-first strategy:
/// 1. Check file mtime against lockfile timestamp
/// 2. Only compute hash if mtime changed
pub fn plan_sync_local(
    outputs: &[OutputFile],
    dest: &SyncDestination,
    lockfile: &Lockfile,
) -> CalvinResult<SyncPlan> {
    let fs = crate::fs::LocalFileSystem;
    
    // For local, we can use regular plan_sync which already does hash checking
    // But add mtime optimization in the future
    plan_sync(outputs, dest, lockfile, &fs)
}
```

### Phase 3: Update Watcher to Use SyncEngine

**File**: `src/watcher.rs` (modify `perform_sync_incremental`)

```rust
fn perform_sync_incremental(
    options: &WatchOptions,
    changed_files: &[PathBuf],
    cache: &mut IncrementalCache,
) -> CalvinResult<SyncResult> {
    use crate::sync::engine::{SyncEngine, SyncEngineOptions};
    
    let assets = parse_incremental(&options.source, changed_files, cache)?;
    let outputs = compile_assets(&assets, &options.targets, &options.config)?;
    
    let engine_options = SyncEngineOptions {
        force: true, // Watch mode overwrites
        interactive: false,
        dry_run: false,
        verbose: false,
        quiet_transfer: !options.json, // Suppress rsync in non-JSON mode
    };
    
    let engine = if options.deploy_to_home {
        SyncEngine::home(&outputs, &options.source, engine_options)
    } else {
        SyncEngine::local(&outputs, options.project_root.clone(), engine_options)
    };
    
    engine.sync()
}
```

### Phase 4: Simplify DeployRunner

**File**: `src/commands/deploy/runner.rs` (simplify)

```rust
// Replace sync_outputs() method with:
fn sync_outputs<F>(&self, outputs: &[OutputFile], callback: Option<F>) -> Result<SyncResult>
where
    F: FnMut(SyncEvent),
{
    use calvin::sync::engine::{SyncEngine, SyncEngineOptions};
    
    let engine_options = SyncEngineOptions {
        force: self.options.force,
        interactive: self.options.interactive,
        dry_run: self.options.dry_run,
        verbose: false,
        quiet_transfer: !self.options.json,
    };
    
    let engine = match &self.target {
        DeployTarget::Project(root) => {
            SyncEngine::local(outputs, root.clone(), engine_options)
        }
        DeployTarget::Home => {
            SyncEngine::home(outputs, &self.source, engine_options)
        }
        DeployTarget::Remote(remote) => {
            let (host, path) = parse_remote(remote);
            SyncEngine::remote(outputs, host, path, &self.source, engine_options)
        }
    };
    
    if let Some(cb) = callback {
        engine.sync_with_callback(Some(cb))
    } else {
        engine.sync()
    }
}
```

## Test Plan (TDD)

### Tests to Add

```rust
// src/sync/engine.rs tests

#[test]
fn engine_local_skips_unchanged_files() {
    // Setup: Create temp dir with existing files matching outputs
    // Action: Run engine.sync()
    // Assert: result.written is empty, result.skipped has all files
}

#[test]
fn engine_local_writes_new_files() {
    // Setup: Empty temp dir
    // Action: Run engine.sync()
    // Assert: result.written has all files
}

#[test]
fn engine_local_detects_modified_files() {
    // Setup: Create files with different content
    // Action: Run engine.plan()
    // Assert: plan.conflicts is not empty
}

#[test]
fn engine_local_updates_lockfile() {
    // Setup: Empty temp dir
    // Action: Run engine.sync()
    // Assert: lockfile contains hashes for all written files
}

#[test]
fn engine_force_overwrites_conflicts() {
    // Setup: Modified files
    // Options: force = true
    // Action: Run engine.sync()
    // Assert: All files written
}

#[test]
fn engine_home_uses_source_lockfile() {
    // Assert: lockfile_path is source/.calvin.lock, not home/.calvin.lock
}

#[test]
fn engine_remote_uses_batch_planning() {
    // Mock RemoteFileSystem
    // Assert: batch_check_files called once, not per-file checks
}
```

### Existing Tests to Verify

```
✓ test_plan_new_files_go_to_write
✓ test_plan_untracked_existing_files_are_conflicts
✓ test_plan_modified_tracked_files_are_conflicts
✓ test_plan_unmodified_tracked_files_can_overwrite
✓ test_execute_empty_plan_returns_empty_result
✓ test_execute_writes_files_to_local
✓ test_execute_with_callback_emits_events
```

## File Changes Summary

| File | Change Type | Lines Changed (est) |
|------|-------------|---------------------|
| `src/sync/engine.rs` | NEW | +200 |
| `src/sync/mod.rs` | MODIFY | +5 (re-export) |
| `src/sync/plan.rs` | MODIFY | +20 (plan_sync_local) |
| `src/watcher.rs` | MODIFY | -20, +10 |
| `src/commands/deploy/runner.rs` | MODIFY | -100, +30 |

**Net change**: ~-85 lines (code reduction!)

## Migration Checklist

- [ ] Create `src/sync/engine.rs` with SyncEngine
- [ ] Add tests for SyncEngine (7 tests)
- [ ] Add `plan_sync_local` to plan.rs
- [ ] Update `sync/mod.rs` to export SyncEngine
- [ ] Update watcher.rs to use SyncEngine
- [ ] Run all watcher tests
- [ ] Update DeployRunner to use SyncEngine
- [ ] Run all deploy tests
- [ ] Manual test: `calvin deploy` to home (should show "X written, Y skipped")
- [ ] Manual test: `calvin watch` (should show clean output)
- [ ] Deprecate sync_outputs() with warning

