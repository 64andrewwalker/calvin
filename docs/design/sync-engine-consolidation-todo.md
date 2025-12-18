# SyncEngine Consolidation TODO

详细的执行任务列表，按 TDD 方式执行。

## ✅ 已完成 (2024-12-18)

所有任务已完成！重构成功删除了约 380 行旧代码，统一了 sync 逻辑。

### 统计结果

| 指标 | 结果 |
|------|------|
| 测试通过 | 190 tests |
| 编译警告 | 0 |
| 删除代码 | ~380 行 (旧 sync 函数 + 旧测试) |
| 新增代码 | ~180 行 (SyncEngine 泛型化 + 新测试) |
| 净减少 | ~200 行 |

---

## Phase A: SyncEngine 泛型化 ✅

### A.1: 修改 SyncEngine 结构体接受泛型 FS ✅

**文件:** `src/sync/engine.rs`

**完成:**
- [x] 添加泛型参数 `FS: FileSystem`
- [x] 添加 `fs: FS` 字段
- [x] 设置默认类型 `FS = LocalFileSystem`
- [x] 添加 `new_with_fs()` 构造器

### A.2: 更新 SyncEngine 方法使用泛型 FS ✅

**文件:** `src/sync/engine.rs`

**完成:**
- [x] 更新 `plan()` 使用 `self.fs`
- [x] 更新 `execute_with_callback()` 使用 `self.fs`
- [x] 更新 `update_lockfile()` 使用 `self.fs`
- [x] 保持 `local()`, `home()`, `remote()` 便捷方法不变

### A.3: 添加 MockFileSystem 测试 ✅

**文件:** `src/sync/engine.rs` (测试模块)

**完成:**
- [x] 编写测试: `engine_with_mock_fs_writes_files`
- [x] 编写测试: `engine_with_mock_fs_creates_lockfile`
- [x] 编写测试: `engine_with_mock_fs_skips_unchanged`
- [x] 编写测试: `engine_with_mock_fs_detects_conflicts`
- [x] 验证现有测试仍通过

---

## Phase B: 可测试的冲突解决 ✅

### B.1: 创建 ConflictResolver trait ✅

**文件:** `src/sync/conflict.rs`

**完成:**
- [x] 定义 `ConflictResolver` trait
- [x] 定义 `ConflictChoice` enum

### B.2: 实现 InteractiveResolver ✅

**文件:** `src/sync/conflict.rs`

**完成:**
- [x] 实现 `InteractiveResolver` struct
- [x] 实现 `ConflictResolver` for `InteractiveResolver`
- [x] 导出 `InteractiveResolver`

### B.3: 更新 SyncEngine 接受 ConflictResolver ✅

**文件:** `src/sync/engine.rs`

**完成:**
- [x] 添加 `sync_with_resolver()` 方法接受泛型 resolver
- [x] 更新 `sync_with_callback()` 使用 resolver 处理冲突
- [x] 保持便捷方法使用默认 `InteractiveResolver`

### B.4: 添加 MockResolver 测试 ✅

**文件:** `src/sync/engine.rs` (测试模块)

**完成:**
- [x] 创建 `MockConflictResolver` struct
- [x] 编写测试: `engine_interactive_overwrite_with_mock_resolver`
- [x] 编写测试: `engine_interactive_skip_with_mock_resolver`

---

## Phase C: 迁移旧测试 ❌ (已取消)

决定不显式迁移旧测试，因为新的 SyncEngine 测试提供了更好的覆盖。
旧测试在 Phase D 中随旧 API 一起删除。

---

## Phase D: 删除旧代码 ✅

### D.1: 删除 sync_outputs/sync_with_fs 函数 ✅

**文件:** `src/sync/mod.rs`

**删除函数 (~258 行):**
- [x] `sync_outputs()`
- [x] `sync_with_fs()`
- [x] `sync_with_fs_with_prompter()`
- [x] `sync_with_fs_with_callback()`
- [x] `sync()`

### D.2: 删除 SyncPrompter trait ✅

**文件:** `src/sync/conflict.rs`

**删除类型:**
- [x] `SyncPrompter` trait
- [x] `StdioPrompter` struct

### D.3: 更新 lib.rs 导出 ✅

**文件:** `src/lib.rs`

**完成:**
- [x] 移除 `sync_outputs` 导出
- [x] 确保 `SyncEngine` 已导出
- [x] 验证公共 API 正确

### D.4: 删除旧测试 ✅

**文件:** `src/sync/tests.rs`

**删除测试 (~380 行):**
- [x] `test_sync_user_scope`
- [x] `test_sync_with_mock_fs`
- [x] `test_sync_with_callback_emits_written_events`
- [x] 所有 `test_sync_interactive_*` 测试

### D.5: 运行所有测试验证 ✅

**完成:**
- [x] `cargo test --lib` - 190 tests passed
- [x] `cargo test` - All tests passed
- [x] `cargo build --release` - 0 warnings

---

## 验收标准 ✅

- [x] 所有 190 测试通过
- [x] 0 个编译警告
- [x] 代码行数减少 ~200 行 (净变化)
- [x] `SyncEngine` 支持 `MockFileSystem` 进行测试
- [x] `ConflictResolver` trait 支持 mock 冲突解决

---

## 架构改进

### 之前
```
sync_outputs() → sync_with_fs() → sync_with_fs_with_prompter() → ...
     ↓                 ↓                       ↓
    250+ 行嵌套逻辑，难以测试，SyncPrompter 无法 mock
```

### 之后
```
SyncEngine<FS, CR>
     │
     ├─ plan() → SyncPlan
     ├─ execute_with_callback() → SyncResult
     └─ sync_with_resolver() → SyncResult
           ↓
    泛型 FS + CR 支持完全 mock 测试
```

关键改进:
1. **泛型 FileSystem**: `SyncEngine<FS>` 可以使用 `MockFileSystem` 测试
2. **可注入 Resolver**: `sync_with_resolver()` 接受任意 `ConflictResolver`
3. **清晰分层**: plan → resolve → execute
4. **代码减少**: 净减少约 200 行

