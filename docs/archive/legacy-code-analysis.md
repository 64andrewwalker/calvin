# 遗留代码分析与迁移方案

> **Created**: 2025-12-20  
> **Status**: 待审核

---

## 1. 旧引擎使用情况

### 1.1 触发条件

旧引擎（DeployRunner）仅在以下条件下被调用：

```rust
// src/commands/deploy/cmd.rs:241-261
} else if super::bridge::should_use_new_engine() {
    // 新引擎路径
} else {
    // 旧引擎路径 - 仅当 CALVIN_LEGACY_ENGINE=1
    runner.run()?
}
```

**判断逻辑** (`bridge.rs:46-53`):

```rust
pub fn should_use_new_engine() -> bool {
    std::env::var("CALVIN_LEGACY_ENGINE")
        .map(|v| v != "1")
        .unwrap_or(true)  // 默认使用新引擎
}
```

### 1.2 新引擎覆盖的路径

| 部署路径 | 引擎 | 状态 |
|----------|------|------|
| 本地项目 (`--project`) | 新引擎 | ✅ |
| 本地 Home (`--home`) | 新引擎 | ✅ |
| 远程 (`--remote`) | 新引擎 | ✅ |
| JSON 模式 (`--json`) | 新引擎 | ✅ |
| `CALVIN_LEGACY_ENGINE=1` | 旧引擎 | ⚠️ 待移除 |

---

## 2. 遗留代码清单

### 2.1 遗留模块结构

```
src/
├── adapters/                    # 🔴 遗留 - 被 sync/ 使用
│   ├── mod.rs                   # TargetAdapter trait (旧版)
│   ├── antigravity.rs
│   ├── claude_code.rs
│   ├── codex.rs
│   ├── cursor.rs
│   ├── escaping.rs              # ⚠️ 未使用
│   └── vscode.rs
│
├── commands/deploy/
│   ├── runner.rs                # 🔴 遗留 DeployRunner
│   └── bridge.rs                # 🟡 桥接层 (保留部分转换逻辑)
│
├── sync/                        # 🔴 遗留同步引擎
│   ├── engine.rs                # SyncEngine (被 DeployRunner 使用)
│   ├── compile.rs               # 编译逻辑 (已迁移到 domain/services)
│   ├── execute.rs               # 执行逻辑
│   ├── pipeline.rs              # AssetPipeline
│   ├── plan.rs                  # SyncPlan (部分共用)
│   ├── lockfile.rs              # 🟡 共用 - 被新旧引擎都使用
│   ├── orphan.rs                # 🟡 共用 - 孤儿检测
│   ├── remote.rs                # 🟡 共用 - rsync 执行
│   ├── conflict.rs              # 🟡 共用 - 冲突解决 UI
│   ├── scope.rs
│   ├── tests.rs
│   └── writer.rs                # 🟢 通用 - 原子写入
```

### 2.2 依赖关系图

```
DeployRunner (runner.rs)
    │
    ├─→ sync/engine.rs (SyncEngine)
    │       │
    │       ├─→ sync/compile.rs → src/adapters/*
    │       ├─→ sync/execute.rs
    │       ├─→ sync/plan.rs
    │       ├─→ sync/lockfile.rs ←─ (共用)
    │       └─→ sync/remote.rs (rsync)
    │
    └─→ sync/orphan.rs (孤儿检测)

新引擎 (DeployUseCase)
    │
    ├─→ infrastructure/adapters/* ✅
    ├─→ domain/services/planner.rs ✅
    ├─→ infrastructure/repositories/lockfile.rs ✅
    └─→ infrastructure/sync/remote.rs ✅ (新的 SyncDestination)
```

### 2.3 遗留代码统计

| 文件 | 行数 | 状态 | 说明 |
|------|------|------|------|
| `src/adapters/mod.rs` | 168 | 🔴 遗留 | 旧 trait 定义 |
| `src/adapters/claude_code.rs` | 450 | 🔴 遗留 | 旧适配器 |
| `src/adapters/cursor.rs` | 280 | 🔴 遗留 | 旧适配器 |
| `src/adapters/vscode.rs` | 380 | 🔴 遗留 | 旧适配器 |
| `src/adapters/antigravity.rs` | 220 | 🔴 遗留 | 旧适配器 |
| `src/adapters/codex.rs` | 260 | 🔴 遗留 | 旧适配器 |
| `src/adapters/escaping.rs` | 224 | ⚠️ 未使用 | 可直接删除 |
| `commands/deploy/runner.rs` | 593 | 🔴 遗留 | DeployRunner |
| `sync/engine.rs` | 1601 | 🔴 遗留 | 老 SyncEngine |
| `sync/compile.rs` | 120 | 🔴 遗留 | 编译逻辑 |
| `sync/execute.rs` | 280 | 🔴 遗留 | 执行逻辑 |
| `sync/pipeline.rs` | 180 | 🔴 遗留 | 资产管道 |
| `sync/plan.rs` | 650 | 🟡 共用 | 规划逻辑 |
| `sync/lockfile.rs` | 370 | 🟡 共用 | 锁文件 |
| `sync/orphan.rs` | 480 | 🟡 共用 | 孤儿检测 |
| `sync/remote.rs` | 320 | 🟡 共用 | rsync |
| `sync/conflict.rs` | 240 | 🟢 保留 | 冲突 UI |
| `sync/writer.rs` | 140 | 🟢 保留 | 原子写入 |

**总计**: 约 6,900 行遗留代码

---

## 3. 使用遗留代码的位置

### 3.1 使用 `src/adapters/` 的文件

```bash
# 共 17 处引用
src/infrastructure/adapters/codex.rs      # LegacyTargetAdapter 别名
src/infrastructure/adapters/antigravity.rs
src/infrastructure/adapters/vscode.rs
src/sync/remote.rs                         # OutputFile
src/sync/orphan.rs
src/sync/engine.rs
src/sync/plan.rs
src/sync/pipeline.rs
src/sync/execute.rs
src/sync/compile.rs
src/adapters/* (内部引用)
```

### 3.2 使用 `calvin::sync::` 的文件

| 文件 | 使用的类型 | 可迁移到 |
|------|-----------|----------|
| `commands/deploy/cmd.rs` | Lockfile, delete_orphans, ScopePolicy | domain/entities, domain/services |
| `commands/deploy/runner.rs` | SyncEngine, plan_sync, execute_sync | 删除整个文件 |
| `commands/deploy/bridge.rs` | SyncResult | domain/value_objects |
| `commands/deploy/targets.rs` | SyncDestination | domain/ports |
| `commands/interactive.rs` | SyncResult | 已迁移 |
| `commands/debug.rs` | Lockfile, detect_orphans | domain |
| `ui/views/orphan.rs` | OrphanFile | domain/entities |
| `watcher.rs` | AssetPipeline, SyncEngine | application |
| `lib.rs` | 公开导出 | 更新导出路径 |

### 3.3 lib.rs 公开导出

```rust
// 当前导出 (遗留)
pub use adapters::{all_adapters, get_adapter, OutputFile, TargetAdapter};
pub use sync::{compile_assets, SyncEngine, SyncEngineOptions, SyncOptions, SyncResult};

// 应该改为
pub use infrastructure::{all_adapters, get_adapter};
pub use domain::entities::OutputFile;
pub use domain::ports::TargetAdapter;
pub use application::{DeployUseCase, DeployOptions, DeployResult};
```

---

## 4. 迁移方案

### 4.1 阶段 1: 移除 CALVIN_LEGACY_ENGINE 支持 (1-2 小时)

**目标**: 完全移除旧引擎入口

**步骤**:

1. 删除 `commands/deploy/cmd.rs` 中的旧引擎分支
2. 删除 `bridge.rs` 中的 `should_use_new_engine()` 函数
3. 标记 `runner.rs` 为 `#[deprecated]`
4. 运行测试确保没有回归

**影响**: 用户无法再通过环境变量回退到旧引擎

### 4.2 阶段 2: 清理 runner.rs (2-3 小时)

**目标**: 删除 DeployRunner 及其依赖

**步骤**:

1. 确认所有功能已迁移到 DeployUseCase
2. 删除 `commands/deploy/runner.rs`
3. 更新 `commands/deploy/mod.rs`
4. 简化 `bridge.rs`，只保留类型转换函数

**依赖检查**:
- [ ] DeployRunner.run() → DeployUseCase.execute()
- [ ] detect_orphans() → DeployUseCase 内部处理
- [ ] update_lockfile() → LockfileRepository

### 4.3 阶段 3: 清理 sync/ 模块 (4-6 小时)

**目标**: 删除不再需要的 sync 子模块

**可直接删除**:
- `sync/compile.rs` - 已迁移到 domain/services/compiler
- `sync/execute.rs` - 已迁移到 DeployUseCase
- `sync/pipeline.rs` - 已迁移到 application

**需要保留**:
- `sync/conflict.rs` - 冲突解决 UI，迁移到 infrastructure/conflict
- `sync/writer.rs` - 原子写入，迁移到 infrastructure/fs
- `sync/lockfile.rs` - 部分功能共用

**需要迁移**:
- `sync/plan.rs` 中的 Conflict, SyncPlan 类型
- `sync/orphan.rs` 中的 OrphanFile 类型
- `sync/remote.rs` 中的 rsync 执行逻辑

**步骤**:

1. 将 `sync/conflict.rs` 的 unified_diff 迁移到 infrastructure
2. 将 `sync/writer.rs` 迁移到 infrastructure/fs
3. 删除 `sync/engine.rs`
4. 删除 `sync/compile.rs`, `execute.rs`, `pipeline.rs`
5. 更新 `sync/mod.rs` 只导出保留的模块

### 4.4 阶段 4: 删除 src/adapters/ (2-3 小时)

**前置条件**: sync/ 清理完成

**步骤**:

1. 确认 `src/infrastructure/adapters/` 不再依赖 `src/adapters/`
2. 更新 infrastructure adapters 中的 LegacyTargetAdapter 导入
3. 删除整个 `src/adapters/` 目录
4. 更新 `lib.rs` 导出

### 4.5 阶段 5: 更新公开 API (1-2 小时)

**步骤**:

1. 更新 `lib.rs` 导出新的类型
2. 添加向后兼容的类型别名（如果需要）
3. 更新文档

---

## 5. 风险评估

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 功能回归 | 高 | 全面的集成测试覆盖 |
| 外部依赖 | 低 | lib.rs 导出的类型变更 |
| watch 命令 | 中 | watch 仍使用 SyncEngine |
| rsync 功能 | 中 | 需要验证远程部署 |

---

## 6. 建议执行顺序

```
Week 1:
├── Day 1-2: 阶段 1 (移除 CALVIN_LEGACY_ENGINE)
└── Day 3-4: 阶段 2 (删除 runner.rs)

Week 2:
├── Day 1-3: 阶段 3 (清理 sync/)
└── Day 4-5: 阶段 4 (删除 adapters/)

Week 3:
└── Day 1-2: 阶段 5 (更新公开 API)
```

**预计总工时**: 12-18 小时

---

## 7. 检查清单

### 删除前确认

- [ ] 所有测试通过
- [ ] 新引擎覆盖所有用例
- [ ] watch 命令正常工作
- [ ] 远程部署正常工作
- [ ] JSON 模式正常工作
- [ ] 孤儿检测正常工作
- [ ] 锁文件更新正常工作

### 删除后验证

- [ ] cargo build 成功
- [ ] cargo test 全部通过
- [ ] cargo clippy 无警告
- [ ] 手动测试核心功能

