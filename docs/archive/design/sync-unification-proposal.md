# Sync Logic Unification Proposal

> Date: 2024-12-18
> Status: Proposed
> Author: AI Assistant

## Problem Statement

Calvin 有多个同步路径，但它们的实现不一致：

| 功能 | Deploy (Remote) | Deploy (Local) | Watch |
|------|-----------------|----------------|-------|
| 增量检测 | ✅ `plan_sync_remote()` | ❌ 直接 rsync | ❌ 直接 rsync |
| Hash 对比 | ✅ lockfile + remote SHA | ❌ | ❌ |
| 冲突提示 | ✅ interactive | ❌ | ❌ |
| 跳过未变化 | ✅ | ❌ | ❌ |
| UI 反馈 | ✅ "X files written, Y skipped" | ⚠️ rsync 输出 | ⚠️ rsync 输出 |

**结果**：
- 本地/home 部署总是传输所有文件（即使未变化）
- Watch 模式初始同步传输所有文件
- rsync 的原始输出替代了我们精心设计的 UI

## Current Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Entry Points                              │
├───────────────┬───────────────┬──────────────────────────────┤
│ cmd_deploy    │ cmd_watch     │ cmd_sync (deprecated)        │
└───────┬───────┴───────┬───────┴──────────────────────────────┘
        │               │
        ▼               ▼
┌───────────────┐ ┌───────────────────────────────────────────┐
│ DeployRunner  │ │ watcher.rs::perform_sync_incremental()    │
│ (two-stage)   │ │ (single-stage, force=true)                │
└───────┬───────┘ └───────────────┬───────────────────────────┘
        │                         │
        ├─────────────────────────┤
        ▼                         ▼
┌───────────────────────────────────────────────────────────────┐
│                    Sync Backend                                │
├───────────────────┬───────────────────┬───────────────────────┤
│ sync_outputs()    │ sync_local_rsync()│ sync_remote_rsync()   │
│ (file-by-file)    │ (batch local)     │ (batch remote SSH)    │
└───────────────────┴───────────────────┴───────────────────────┘
```

### Key Issues

1. **DeployRunner** 只用于 `cmd_deploy`，Watch 有自己的同步逻辑
2. **plan_sync / plan_sync_remote** 只在 DeployRunner 中使用
3. **Watch 总是使用 force=true**，跳过所有冲突检测
4. **本地部署** 的 `select_strategy()` 检查 `plan.to_write.len() > 10`，但本地模式没有经过 planning！

## Proposed Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Entry Points                              │
├───────────────┬───────────────┬──────────────────────────────┤
│ cmd_deploy    │ cmd_watch     │ cmd_sync (deprecated)        │
└───────┬───────┴───────┬───────┴──────────────────────────────┘
        │               │
        ▼               ▼
┌─────────────────────────────────────────────────────────────┐
│                    SyncEngine (NEW)                          │
│  - Unified two-stage sync for all destinations               │
│  - plan() -> resolve() -> execute()                          │
│  - Consistent lockfile handling                              │
│  - Unified UI callbacks                                      │
└───────────────────────────────┬─────────────────────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        ▼                       ▼                       ▼
┌───────────────┐       ┌───────────────┐       ┌───────────────┐
│ LocalFS       │       │ RemoteFS      │       │ Future FS     │
│ (plan_sync)   │       │ (plan_sync_   │       │ (cloud, etc)  │
│               │       │  remote)      │       │               │
└───────────────┘       └───────────────┘       └───────────────┘
```

## Refactoring Steps

### Phase 1: Extract SyncEngine

1. **Move `DeployRunner` core logic to `sync/engine.rs`**
   - `SyncEngine::new(outputs, destination, options)`
   - `SyncEngine::plan()` → `SyncPlan`
   - `SyncEngine::resolve(plan)` → `SyncPlan` (with interactive prompt if needed)
   - `SyncEngine::execute(plan)` → `SyncResult`

2. **Generalize lockfile path handling**
   - For local: `<dest>/.promptpack/.calvin.lock` or `<source>/.calvin.lock`
   - For remote: `<source>/.calvin.lock` (local copy)
   - Make this configurable in SyncEngine

### Phase 2: Unify plan_sync

1. **Make `plan_sync` work for local destinations**
   - Currently only `plan_sync_remote` does proper hash checking
   - Add batch file hash checking for local FS (use parallel reads)
   
2. **Add `plan_sync_local` optimized variant**
   - Use `fs::metadata()` for quick mtime checks first
   - Fall back to hash comparison only if mtime changed

### Phase 3: Update Watch to Use SyncEngine

1. **Replace `perform_sync_incremental()`**
   ```rust
   // Before
   let sync_options = SyncOptions { force: true, ... };
   if has_rsync() && outputs.len() > 10 {
       sync_local_rsync(&dist_root, &outputs, &sync_options, ...)
   } else {
       sync_outputs(&dist_root, &outputs, &sync_options)
   }
   
   // After
   let engine = SyncEngine::new(outputs, dest, options);
   let plan = engine.plan();
   let result = engine.execute(plan.overwrite_all());  // force mode
   ```

2. **Benefits**
   - Watch gets "X files written, Y skipped" for free
   - Lockfile is automatically updated
   - No more rsync raw output in watch mode

### Phase 4: Improve Local Deploy

1. **Deploy to home/project should use planning**
   - Current: Skip planning entirely for local, use rsync
   - Proposed: Always plan, then use rsync/file-by-file for execution

2. **Show proper summary**
   - "4 files written, 143 skipped" instead of rsync file list

## API Design

```rust
// sync/engine.rs

pub struct SyncEngine {
    outputs: Vec<OutputFile>,
    destination: SyncDestination,
    lockfile_path: PathBuf,
    options: SyncEngineOptions,
}

pub struct SyncEngineOptions {
    /// Skip planning, overwrite everything
    pub force: bool,
    /// Prompt user for conflicts
    pub interactive: bool,
    /// Don't actually write files
    pub dry_run: bool,
    /// Show raw rsync output (for JSON=false, verbose mode)
    pub show_transfer_details: bool,
}

impl SyncEngine {
    pub fn new(
        outputs: Vec<OutputFile>,
        destination: SyncDestination,
        lockfile_path: PathBuf,
    ) -> Self;
    
    /// Stage 1: Detect changes and conflicts
    pub fn plan(&self) -> CalvinResult<SyncPlan>;
    
    /// Stage 2: Execute the plan
    pub fn execute(&self, plan: SyncPlan) -> CalvinResult<SyncResult>;
    
    /// Stage 2 with progress callbacks
    pub fn execute_with_callback<F>(&self, plan: SyncPlan, callback: F) -> CalvinResult<SyncResult>
    where F: FnMut(SyncEvent);
    
    /// Convenience: plan -> auto-resolve -> execute
    pub fn sync(&self) -> CalvinResult<SyncResult> {
        let plan = self.plan()?;
        let resolved = if self.options.force {
            plan.overwrite_all()
        } else if self.options.interactive {
            // ... interactive resolution
        } else {
            plan.skip_all()
        };
        self.execute(resolved)
    }
}
```

## Migration Path

1. **Phase 1** (1-2 hours)
   - Create `sync/engine.rs` with SyncEngine
   - Migrate DeployRunner to use SyncEngine internally
   - All tests should still pass

2. **Phase 2** (1 hour)
   - Add `plan_sync` for local FS (like plan_sync_remote but for local)
   - Update SyncEngine to use it

3. **Phase 3** (1 hour)
   - Update watcher.rs to use SyncEngine
   - Remove direct rsync calls from watcher

4. **Phase 4** (30 min)
   - Update UI to hide rsync output by default
   - Show clean "X written, Y skipped" summary

## Benefits

| Aspect | Before | After |
|--------|--------|-------|
| Code paths | 3 (deploy, watch, sync_outputs) | 1 (SyncEngine) |
| Incremental detection | Remote only | All destinations |
| Lockfile handling | Inconsistent | Unified |
| UI consistency | Variable | Always clean summary |
| Testing | Hard to test all paths | One engine to test |

## Risks

1. **Breaking changes**: Deploy behavior might change (show fewer files)
   - Mitigation: Keep `--verbose` flag to show all transfer details

2. **Performance regression**: Planning adds overhead
   - Mitigation: Use mtime-first strategy for local, batch checks for remote

3. **Watch mode latency**: Extra planning step
   - Mitigation: Cache lockfile in memory, only reload on changes

## Decision Points

1. **Should watch mode still use force=true?**
   - Option A: Yes, always overwrite (current behavior)
   - Option B: No, respect external changes and prompt
   - Recommendation: A (watch is for dev, external changes rare)

2. **Where should lockfile live for home deploy?**
   - Option A: `~/.promptpack/.calvin.lock` (in dest)
   - Option B: `.promptpack/.calvin.lock` (in source)
   - Recommendation: B (source is the "project", home is just a target)

3. **Should we show rsync output by default?**
   - Option A: Yes, for transparency
   - Option B: No, use clean summary
   - Recommendation: B with `-v` flag for verbose output

