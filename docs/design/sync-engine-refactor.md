# Sync Engine Refactor Design

> **目标**: 统一本地/远程同步策略，交互选择后也能使用 rsync 批量传输

## 当前问题

### 架构问题

```
┌─────────────────────────────────────────────────────────────┐
│                      deploy.rs                               │
│                                                              │
│  if target == Project                                        │
│    └─> sync_outputs() [逐个文件]                             │
│                                                              │
│  if target == Home                                           │
│    └─> sync_outputs() [逐个文件]                             │
│                                                              │
│  if target == Remote                                         │
│    ├─ if force && has_rsync                                  │
│    │    └─> sync_remote_rsync() [批量 ✓]                     │
│    ├─ else if json || animation                              │
│    │    └─> sync_with_fs() via SSH [逐个 ✗]                  │
│    ├─ else if interactive                                    │
│    │    └─> sync_with_fs() via SSH [逐个 ✗]                  │
│    └─ else                                                   │
│         └─> sync_remote_rsync() [批量 ✓]                     │
└─────────────────────────────────────────────────────────────┘
```

### 问题点

1. **交互后无法切换**: 用户选择 "overwrite all" 后，仍逐个上传
2. **本地不用 rsync**: 本地同步永远逐个写入，即使系统有 rsync
3. **冲突检测与传输耦合**: 冲突检测和文件传输混在一起
4. **代码重复**: 多个分支有类似的进度显示逻辑

---

## 重构方案

### 核心思想

**将同步分为两个阶段:**

```
Stage 1: Conflict Resolution (冲突检测)
  └─> 遍历所有文件，检测冲突，收集用户决策
  └─> 输出: 需要写入的文件列表 + 需要跳过的文件列表

Stage 2: Batch Transfer (批量传输)
  └─> 使用 rsync 批量传输（如果可用）
  └─> 或回退到逐个文件写入
```

### 新架构

```
┌─────────────────────────────────────────────────────────────┐
│                     SyncPlan                                 │
│                                                              │
│  pub struct SyncPlan {                                       │
│      to_write: Vec<OutputFile>,   // 需要写入的文件          │
│      to_skip: Vec<String>,        // 跳过的文件              │
│      conflicts: Vec<Conflict>,    // 未决冲突                │
│  }                                                           │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                   Stage 1: plan_sync()                       │
│                                                              │
│  fn plan_sync(                                               │
│      outputs: &[OutputFile],                                 │
│      dest: &SyncDestination,     // Local | Remote           │
│      lockfile: &Lockfile,                                    │
│  ) -> SyncPlan                                               │
│                                                              │
│  - 检查每个文件是否存在冲突                                  │
│  - 不执行任何写入                                            │
│  - 返回冲突列表供用户决策                                    │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                  Stage 2: execute_sync()                     │
│                                                              │
│  fn execute_sync(                                            │
│      plan: &SyncPlan,                                        │
│      dest: &SyncDestination,                                 │
│      strategy: SyncStrategy,     // Rsync | FileByFile       │
│  ) -> SyncResult                                             │
│                                                              │
│  - 无冲突检测，直接批量传输                                  │
│  - 自动选择最快的传输方式                                    │
└─────────────────────────────────────────────────────────────┘
```

### 数据结构

```rust
/// 同步目标
pub enum SyncDestination {
    Local(PathBuf),
    Remote { host: String, path: PathBuf },
}

/// 同步策略
pub enum SyncStrategy {
    /// 自动选择（rsync 优先）
    Auto,
    /// 强制使用 rsync
    Rsync,
    /// 逐个文件（用于需要细粒度进度的场景）
    FileByFile,
}

/// 冲突信息
pub struct Conflict {
    pub path: PathBuf,
    pub reason: ConflictReason,
    pub existing_content: Option<String>,
    pub new_content: String,
}

/// 同步计划
pub struct SyncPlan {
    /// 需要写入的文件
    pub to_write: Vec<OutputFile>,
    /// 跳过的文件（已存在且内容相同）
    pub to_skip: Vec<String>,
    /// 需要用户决策的冲突
    pub conflicts: Vec<Conflict>,
}
```

### 新的 deploy 流程

```rust
// Stage 1: 生成同步计划
let plan = sync::plan_sync(&outputs, &dest, &lockfile)?;

// Stage 2: 处理冲突（如果有）
let final_plan = if plan.conflicts.is_empty() || force {
    // 无冲突或强制覆盖
    SyncPlan {
        to_write: plan.to_write.into_iter()
            .chain(plan.conflicts.into_iter().map(|c| c.into_output()))
            .collect(),
        to_skip: plan.to_skip,
        conflicts: vec![],
    }
} else if interactive {
    // 交互式解决冲突
    resolve_conflicts_interactive(&plan)?
} else {
    // 非交互模式，跳过冲突
    plan
};

// Stage 3: 批量传输
let strategy = if final_plan.to_write.len() > 10 && has_rsync() {
    SyncStrategy::Rsync
} else {
    SyncStrategy::FileByFile
};

let result = sync::execute_sync(&final_plan, &dest, strategy)?;
```

---

## 实施步骤

### Phase 1: 引入 SyncPlan

1. 创建 `src/sync/plan.rs`
2. 实现 `plan_sync()` 函数（只检测冲突，不写入）
3. 实现 `SyncPlan` 和 `Conflict` 结构

### Phase 2: 重构 execute

1. 创建 `src/sync/execute.rs`
2. 实现统一的 `execute_sync()` 函数
3. 内部自动选择 rsync 或逐文件
4. 支持进度回调

### Phase 3: 重构 deploy.rs

1. 拆分当前的 `run_deploy()` 函数
2. 使用新的两阶段 API
3. 移除重复的分支逻辑

### Phase 4: 统一本地/远程

1. `SyncDestination` 抽象统一处理
2. 本地同步也可以使用 rsync
3. 移除 `sync_with_fs` 中的冲突检测

---

## 兼容性

### 保持兼容的接口

```rust
// 旧 API（保留，内部调用新实现）
pub fn sync_outputs(...) -> CalvinResult<SyncResult>

// 新 API
pub fn plan_sync(...) -> CalvinResult<SyncPlan>
pub fn execute_sync(...) -> CalvinResult<SyncResult>
```

### 迁移策略

1. 新代码使用 `plan_sync + execute_sync`
2. 旧 `sync_outputs` 内部调用新 API
3. 逐步迁移所有调用点

---

## 预期收益

| 维度 | Before | After |
|------|--------|-------|
| 交互后选 all | 逐个 SSH | rsync 批量 |
| 本地 100+ 文件 | 逐个写 ~2s | rsync ~0.2s |
| 代码复杂度 | 5 个分支 | 统一流程 |
| 可测试性 | 难 | plan + execute 分离易测 |

---

## 文件变更估计

```
新增:
  src/sync/plan.rs      ~100 lines
  src/sync/execute.rs   ~150 lines

修改:
  src/sync/mod.rs       重导出
  src/commands/deploy.rs  简化 ~200 lines
  src/watcher.rs        使用新 API

删除/简化:
  src/sync/conflict.rs  合并到 plan.rs
```

---

## 后续优化

1. **增量 rsync**: 只传输变化的文件（rsync 自动支持）
2. **并行 SSH**: 如果需要逐个文件，可以并行多个连接
3. **压缩传输**: rsync -z 已支持
4. **断点续传**: rsync --partial 支持
