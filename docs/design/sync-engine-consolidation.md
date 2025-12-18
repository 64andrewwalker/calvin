# SyncEngine Consolidation Design

## 目标

将分散的同步逻辑整合到统一的 `SyncEngine` 中，删除重复代码，同时保持所有功能和测试覆盖。

## 当前问题

### 代码重复

1. **旧 API (`sync/mod.rs` 第 146-385 行, ~240 行)**
   - `sync_outputs()` - 入口点
   - `sync_with_fs()` - 使用泛型 FS
   - `sync_with_fs_with_prompter()` - 支持自定义 prompter
   - 实现完整同步流程：冲突检测 + 交互式提示 + 写入 + lockfile

2. **新 API (`sync/engine.rs` + `plan.rs` + `execute.rs`)**
   - `SyncEngine` - 两阶段设计
   - `plan_sync()` / `plan_sync_remote()` - 冲突检测
   - `execute_sync()` - 文件写入
   - `resolve_conflicts_interactive()` - 交互式冲突解决

两套 API 功能重叠约 80%。

### 可测试性问题

| 特性 | 旧 API | 新 API |
|------|--------|--------|
| 泛型 FileSystem | ✓ `FS: FileSystem` | ✗ 硬编码 `LocalFileSystem` |
| 可 mock 的冲突解决 | ✓ `SyncPrompter` trait | ✗ 直接使用 stdin/stderr |
| MockFileSystem 测试 | ✓ 有约 200 行测试 | ✗ 无法直接测试 |

## 设计方案

### Phase A: SyncEngine 泛型化

#### 目标
使 `SyncEngine` 可以接受任意 `FileSystem` 实现，支持 mock 测试。

#### 变更

```rust
// 之前
pub struct SyncEngine<'a> {
    outputs: &'a [OutputFile],
    destination: SyncDestination,
    lockfile_path: PathBuf,
    options: SyncEngineOptions,
}

impl<'a> SyncEngine<'a> {
    pub fn plan(&self) -> CalvinResult<SyncPlan> {
        let fs = LocalFileSystem; // 硬编码
        // ...
    }
}

// 之后
pub struct SyncEngine<'a, FS: FileSystem = LocalFileSystem> {
    outputs: &'a [OutputFile],
    destination: SyncDestination,
    lockfile_path: PathBuf,
    options: SyncEngineOptions,
    fs: FS,
}

impl<'a, FS: FileSystem> SyncEngine<'a, FS> {
    pub fn new_with_fs(
        outputs: &'a [OutputFile],
        destination: SyncDestination,
        lockfile_path: PathBuf,
        options: SyncEngineOptions,
        fs: FS,
    ) -> Self { ... }
    
    pub fn plan(&self) -> CalvinResult<SyncPlan> {
        // 使用 self.fs
    }
}

// 便捷构造器保持不变（使用 LocalFileSystem）
impl<'a> SyncEngine<'a, LocalFileSystem> {
    pub fn local(outputs: &'a [OutputFile], root: PathBuf, options: SyncEngineOptions) -> Self { ... }
    pub fn home(outputs: &'a [OutputFile], source: &Path, options: SyncEngineOptions) -> Self { ... }
}
```

### Phase B: 可测试的冲突解决

#### 目标
将交互式冲突解决抽象为 trait，支持 mock 测试。

#### 变更

```rust
/// 冲突解决器 trait
pub trait ConflictResolver {
    /// 提示用户解决单个冲突
    fn resolve_conflict(
        &mut self,
        conflict: &Conflict,
        existing_content: &str,
    ) -> ConflictChoice;
    
    /// 显示 diff（如果用户请求）
    fn show_diff(&mut self, diff: &str);
}

/// 交互式解决器（生产环境使用）
pub struct InteractiveResolver;

impl ConflictResolver for InteractiveResolver {
    fn resolve_conflict(&mut self, conflict: &Conflict, existing_content: &str) -> ConflictChoice {
        // 使用 stdin/stderr
    }
    fn show_diff(&mut self, diff: &str) {
        eprintln!("{}", diff);
    }
}

/// 用于测试的 mock 解决器
#[cfg(test)]
pub struct MockResolver {
    pub choices: Vec<ConflictChoice>,
    pub call_count: usize,
}
```

#### SyncEngine 集成

```rust
pub struct SyncEngine<'a, FS: FileSystem = LocalFileSystem, CR: ConflictResolver = InteractiveResolver> {
    // ... existing fields
    resolver: CR,
}

impl<'a, FS: FileSystem, CR: ConflictResolver> SyncEngine<'a, FS, CR> {
    pub fn sync_with_resolver(&self) -> CalvinResult<SyncResult> {
        let plan = self.plan()?;
        
        // 使用 resolver 处理冲突
        let resolved = if self.options.force {
            plan.overwrite_all()
        } else if plan.conflicts.is_empty() {
            plan
        } else if self.options.interactive {
            self.resolve_conflicts(plan)?
        } else {
            plan.skip_all()
        };
        
        self.execute(resolved)
    }
    
    fn resolve_conflicts(&self, mut plan: SyncPlan) -> CalvinResult<SyncPlan> {
        // 使用 self.resolver 而不是直接 stdin
    }
}
```

### Phase C: 迁移测试

迁移现有的 ~200 行测试代码到新 API：

| 旧测试 | 新测试 |
|--------|--------|
| `test_sync_user_scope` | `test_engine_user_scope` |
| `test_sync_with_mock_fs` | `test_engine_with_mock_fs` |
| `test_sync_with_callback_emits_written_events` | `test_engine_callback_emits_events` |
| `test_sync_interactive_overwrite_modified_file` | `test_engine_interactive_overwrite` |
| `test_sync_interactive_skip_modified_file` | `test_engine_interactive_skip` |
| `test_sync_interactive_diff_then_overwrite` | `test_engine_interactive_diff` |
| `test_sync_interactive_overwrite_all_applies_to_multiple_conflicts` | `test_engine_interactive_overwrite_all` |
| `test_sync_interactive_abort_returns_error` | `test_engine_interactive_abort` |

### Phase D: 删除旧代码

删除以下函数和类型：
- `sync_outputs()` (~10 行)
- `sync_with_fs()` (~10 行)
- `sync_with_fs_with_prompter()` (~200 行)
- `sync_with_fs_with_callback()` (~10 行)
- `sync_outputs_with_callback()` (~5 行)
- `sync()` (~15 行)
- `SyncPrompter` trait
- `StdioPrompter` struct
- `ConflictChoice` enum (移至 `conflict.rs`)
- `ConflictReason` enum (已在 `plan.rs` 中有定义)

**预计删除约 280 行代码**

## 文件变更

| 文件 | 变更类型 | 预计行数变化 |
|------|---------|--------------|
| `sync/engine.rs` | 修改 | +50 (泛型支持) |
| `sync/plan.rs` | 修改 | -30 (移除 `resolve_conflicts_interactive`) |
| `sync/mod.rs` | 删除代码 | -280 |
| `sync/conflict.rs` | 修改 | +30 (添加 `ConflictResolver` trait) |
| `sync/tests.rs` | 重写 | 净变化约 0 |
| `lib.rs` | 修改 | -2 (更新导出) |

**净代码减少: ~230 行**

## 风险和缓解

1. **API 破坏性变更**
   - 风险: 外部调用者使用 `sync_outputs`
   - 缓解: 检查未发现外部调用者；`lib.rs` 导出更新

2. **泛型复杂性**
   - 风险: 增加 API 复杂性
   - 缓解: 使用默认类型参数，便捷构造器保持简单

3. **测试迁移遗漏**
   - 风险: 功能回归
   - 缓解: 逐个迁移测试，保持测试数量不变

## 成功指标

- [ ] 所有现有测试通过
- [ ] 代码行数减少 ≥200 行
- [ ] 0 个编译警告
- [ ] 功能回归测试通过

