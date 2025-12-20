# Sync 模块重构设计文档

> 日期: 2025-12-20
> 状态: 设计中

## 目录

1. [当前状态分析](#当前状态分析)
2. [问题诊断](#问题诊断)
3. [目标架构](#目标架构)
4. [迁移计划](#迁移计划)
5. [TODO 列表](#todo-列表)

---

## 当前状态分析

### 现有 sync/ 模块结构

```
src/sync/
├── compile.rs      # 资产编译（已使用新适配器）
├── conflict.rs     # 冲突类型和解析器
├── engine.rs       # SyncEngine 主引擎（1602行）
├── execute.rs      # 执行同步操作
├── lockfile.rs     # 锁文件管理
├── mod.rs          # 模块入口和公共 API
├── orphan.rs       # 孤儿文件检测
├── pipeline.rs     # AssetPipeline 编译管道
├── plan.rs         # 同步计划和冲突检测
├── remote.rs       # 远程 rsync 同步
├── scope.rs        # ScopePolicy 范围策略
├── tests.rs        # 测试
└── writer.rs       # 原子写入和哈希
```

### 外部依赖关系

| sync 模块功能 | 使用位置 | 依赖类型 |
|--------------|---------|---------|
| `expand_home_dir` | debug.rs, fs.rs | 工具函数 |
| `AssetPipeline` | debug.rs, watcher.rs | 编译管道 |
| `ScopePolicy` | debug.rs, cmd.rs, watcher.rs | 策略对象 |
| `SyncResult` | bridge.rs, interactive.rs, watcher.rs | 结果类型 |
| `SyncEngine`, `SyncEngineOptions` | watcher.rs | 核心引擎 |
| `SyncDestination` | targets.rs | 值对象 |
| `Lockfile`, `LockfileNamespace` | debug.rs | 数据管理 |
| `detect_orphans` | debug.rs | 服务函数 |
| `OrphanFile`, `extract_path_from_key` | ui/orphan.rs | UI 数据 |
| `writer::atomic_write`, `hash_file` | fs.rs, local.rs | 底层工具 |
| `ConflictResolver`, `InteractiveResolver` | 公共 API | trait |

### 文件系统重复实现

目前存在两套文件系统：

1. **src/fs.rs** (`LocalFileSystem`, `RemoteFileSystem`)
   - 被 sync 模块使用
   - 依赖 sync::writer

2. **src/infrastructure/fs/local.rs** (`LocalFs`)
   - 被新的 DeployUseCase 使用
   - 已内联 atomic_write 和 hash

---

## 问题诊断

### P1: 模块职责不清

- `sync/engine.rs` 1602 行，混合了太多职责
- `SyncEngine` 既做规划又做执行
- 编译、规划、执行、孤儿检测耦合在一起

### P2: 两套文件系统实现

- `LocalFileSystem` (fs.rs) vs `LocalFs` (infrastructure/)
- 代码重复，维护成本高
- 新代码应统一使用 `LocalFs`

### P3: 遗留代码污染

- `expand_home_dir` 应该在 FileSystem 接口中
- `writer.rs` 的功能已被内联到 LocalFs
- 多个 deprecated 标记未清理

### P4: 架构层次不一致

- sync 模块不符合分层架构（domain/application/infrastructure）
- 业务逻辑和基础设施代码混合
- 新旧代码风格不一致

---

## 目标架构

### 层次划分

```
domain/
├── entities/
│   ├── asset.rs           # 已有
│   ├── output_file.rs     # 已有
│   └── lockfile.rs        # 新增：Lockfile 实体
├── value_objects/
│   ├── scope.rs           # 已有
│   ├── target.rs          # 已有
│   └── sync_destination.rs # 新增：从 sync/plan.rs 迁移
├── services/
│   ├── compiler.rs        # 已有
│   ├── planner.rs         # 已有
│   ├── orphan_detector.rs # 已有
│   └── conflict_resolver.rs # 新增：从 sync/conflict.rs 迁移
└── ports/
    ├── file_system.rs     # 已有
    ├── lockfile_repository.rs # 已有
    └── sync_executor.rs   # 新增：同步执行端口

application/
├── deploy.rs              # 已有：DeployUseCase
├── check.rs               # 已有
├── diff.rs                # 已有
└── watch.rs               # 新增：WatchUseCase（从 watcher.rs 迁移）

infrastructure/
├── fs/
│   ├── local.rs           # 已有：LocalFs
│   ├── remote.rs          # 新增：RemoteFs（从 fs.rs 迁移）
│   └── mod.rs
├── sync/
│   ├── rsync_executor.rs  # 新增：rsync 实现
│   └── file_executor.rs   # 新增：逐文件同步
└── repositories/
    └── lockfile.rs        # 已有
```

### 统一文件系统

目标：删除 `src/fs.rs`，统一使用 `infrastructure/fs/`

```rust
// infrastructure/fs/remote.rs (新)
pub struct RemoteFs {
    destination: String,
    cached_home: Mutex<Option<String>>,
}

impl FileSystem for RemoteFs {
    // 从 src/fs.rs RemoteFileSystem 迁移
}
```

### ScopePolicy 迁移

```rust
// domain/policies/scope_policy.rs (已有，需更新)
pub enum ScopePolicy {
    User,
    Project,
    Both,
}

impl ScopePolicy {
    pub fn lockfile_namespace(&self) -> LockfileNamespace { ... }
    pub fn target_for_scope(&self, project_root: &Path) -> Vec<PathBuf> { ... }
}
```

---

## 迁移计划

### Phase 1: 统一文件系统 ⏳

**目标**: 删除 `src/fs.rs`，统一使用 `infrastructure/fs/`

1.1. 创建 `infrastructure/fs/remote.rs`，从 `fs.rs` 迁移 `RemoteFileSystem`
1.2. 更新所有使用 `LocalFileSystem` 的地方，改用 `LocalFs`
1.3. 更新所有使用 `RemoteFileSystem` 的地方，改用 `RemoteFs`
1.4. 删除 `src/fs.rs`
1.5. 更新 `lib.rs` 导出

### Phase 2: 迁移 Lockfile 实体 ⏳

**目标**: 将 Lockfile 从 sync 迁移到 domain

2.1. 创建 `domain/entities/lockfile.rs`，定义 Lockfile 实体
2.2. 更新 `LockfileRepository` 使用新实体
2.3. 更新所有 `sync::Lockfile` 引用
2.4. 删除 `sync/lockfile.rs` 中的重复代码

### Phase 3: 迁移 SyncDestination ⏳

**目标**: 将 SyncDestination 移到 domain 值对象

3.1. 创建 `domain/value_objects/sync_destination.rs`
3.2. 从 `sync/plan.rs` 迁移类型定义
3.3. 更新所有引用

### Phase 4: 迁移冲突解析 ⏳

**目标**: 将冲突解析移到 domain 服务

4.1. 创建 `domain/services/conflict_resolver.rs`
4.2. 从 `sync/conflict.rs` 迁移 trait 和实现
4.3. 更新所有引用

### Phase 5: 简化 SyncEngine ⏳

**目标**: 拆分 SyncEngine，降低复杂度

5.1. 提取规划逻辑到 `domain/services/sync_planner.rs`
5.2. 提取执行逻辑到 `infrastructure/sync/executor.rs`
5.3. SyncEngine 变成薄封装
5.4. 减少 engine.rs 行数到 <500

### Phase 6: 清理遗留代码 ⏳

**目标**: 删除不再需要的模块

6.1. 删除 `sync/writer.rs`（功能已内联到 LocalFs）
6.2. 删除 `sync/mod.rs` 中的 `expand_home_dir`（使用 FileSystem::expand_home）
6.3. 清理 deprecated 标记
6.4. 更新文档

---

## TODO 列表

### 高优先级

- [ ] **P1.1** 创建 `infrastructure/fs/remote.rs`
- [ ] **P1.2** 迁移 sync/engine.rs 使用 LocalFs 替代 LocalFileSystem
- [ ] **P1.3** 迁移所有命令使用新文件系统
- [ ] **P1.4** 删除 `src/fs.rs`

### 中优先级

- [ ] **P2.1** 创建 `domain/entities/lockfile.rs`
- [ ] **P2.2** 更新 LockfileRepository
- [ ] **P3.1** 迁移 SyncDestination 到 domain
- [ ] **P4.1** 迁移 ConflictResolver 到 domain

### 低优先级

- [ ] **P5.1** 拆分 SyncEngine
- [ ] **P6.1** 删除 sync/writer.rs
- [ ] **P6.2** 清理 expand_home_dir
- [ ] **P6.3** 更新文档

---

## TDD 实施指南

每个 Phase 的实施步骤：

1. **编写测试** - 先为目标行为编写测试
2. **运行测试** - 确认测试失败（红）
3. **实现代码** - 最小化实现使测试通过
4. **重构** - 改进代码质量
5. **验证** - 运行全部测试确保无回归

### 测试策略

```rust
// 示例：Phase 1.1 的测试
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn remote_fs_expands_home() {
        // 测试 RemoteFs::expand_home
    }
    
    #[test]
    fn remote_fs_batch_check_files() {
        // 测试批量文件检查
    }
}
```

---

## 风险和缓解

| 风险 | 影响 | 缓解措施 |
|-----|-----|---------|
| sync 模块被广泛使用 | 高 | 保持公共 API 兼容，逐步迁移 |
| watcher 命令依赖复杂 | 中 | 最后处理，先完成基础迁移 |
| 测试覆盖不足 | 中 | 每步添加测试，不跳过 |

---

## 验收标准

重构完成后：

- [ ] `src/fs.rs` 已删除
- [ ] `sync/writer.rs` 已删除
- [ ] `sync/engine.rs` < 500 行
- [ ] 所有测试通过
- [ ] 无 deprecated 警告
- [ ] 文档已更新

