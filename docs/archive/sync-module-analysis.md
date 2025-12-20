# Sync 模块分析

## 评估标准
- **可直接复用**: 逻辑清晰，接口合理，可直接保留或简单重新导出
- **需要整理**: 逻辑合理但接口或依赖需要调整
- **需要重写**: 逻辑过于复杂或与新架构不兼容

---

## 模块分析

### 1. compile.rs (155行) ✅ 可直接复用

**现状**: 
- 已使用新的 infrastructure adapters
- 提供 `compile_assets()` 函数，编译资产到输出文件

**质量**: 
- 逻辑清晰，单一职责
- 已完成迁移到新适配器

**建议**: 保留作为公共 API，或在适当时候移至 application 层

---

### 2. conflict.rs (242行) ✅ 可直接复用

**现状**:
- 定义 `ConflictResolver` trait 和 `InteractiveResolver`
- 提供 `unified_diff()` 函数

**质量**:
- 抽象良好，trait 设计清晰
- 已被 `domain::ports::ConflictResolver` 使用

**建议**: 
- `ConflictReason` 和 `ConflictChoice` 可移至 domain::value_objects
- `InteractiveResolver` 可移至 infrastructure

---

### 3. engine.rs (1597行) ⚠️ 需要整理

**现状**:
- 大型统一同步引擎，处理本地和远程同步
- 包含大量测试 (~1000行测试代码)

**问题**:
- 文件过大，违反单一职责原则
- 与新的 `DeployUseCase` 功能重叠

**建议**:
- 标记为 deprecated
- 保留给 `watcher` 命令使用
- 新代码使用 `DeployUseCase`
- 长期目标：迁移 watcher 到新架构后删除

---

### 4. execute.rs (292行) ✅ 可直接复用

**现状**:
- 实现 `execute_sync()` 函数
- 支持 rsync 批量传输

**质量**:
- 逻辑清晰，职责单一
- rsync 策略实现良好

**建议**: 
- 可移至 infrastructure/sync/
- 或保留作为内部实现细节

---

### 5. lockfile.rs (357行) ✅ 已迁移

**现状**:
- `LockfileNamespace` 和 `lockfile_key` 已从 domain 重新导出
- `Lockfile` struct 保留用于 I/O 操作

**问题**:
- 两个 Lockfile 实现（sync::Lockfile 和 domain::entities::Lockfile）
- sync::Lockfile 有 I/O 方法，domain::Lockfile 是纯数据

**建议**:
- 长期目标：让所有使用方改用 `LockfileRepository`
- 短期：保留 sync::Lockfile 用于 watcher 等旧代码

---

### 6. orphan.rs (464行) ⚠️ 需要整理

**现状**:
- 孤儿文件检测逻辑
- 定义 `OrphanFile` 和 `OrphanDetectionResult`

**问题**:
- 与 `domain::services::orphan_detector` 功能重叠
- `ui/views/orphan.rs` 依赖此模块的类型

**建议**:
- 比较两个实现的功能差异
- 统一到 domain 层
- 更新 UI 使用新类型

---

### 7. pipeline.rs (174行) ⚠️ 需要整理

**现状**:
- `AssetPipeline` 统一解析 + 编译流程
- 被 `watcher.rs` 和 `debug.rs` 使用

**问题**:
- 使用旧的 `PromptAsset` 类型
- 与新架构的 `AssetRepository` + Adapters 功能重叠

**建议**:
- 保留给 watcher 使用
- 新代码直接使用 `AssetRepository` + `DeployUseCase`
- 长期目标：迁移 watcher 后删除

---

### 8. plan.rs (583行) ✅ 可直接复用

**现状**:
- `plan_sync()` 函数检测冲突
- 定义 `SyncDestination`, `Conflict`, `SyncPlan`

**质量**:
- 职责清晰（计划阶段，不做实际写入）
- 支持本地和远程目标

**建议**:
- `SyncDestination` 可考虑移至 domain
- 实现细节可保留在 sync/ 或移至 infrastructure/

---

### 9. remote.rs (351行) ✅ 可直接复用

**现状**:
- `sync_remote_rsync()` 函数
- 优化的远程同步（rsync 批量传输）

**质量**:
- 实现高效，使用 rsync 减少 SSH 调用
- 单一职责

**建议**:
- 可移至 infrastructure/sync/remote.rs
- 或保留作为 plan.rs 的内部实现

---

### 10. scope.rs (128行) ✅ 已迁移

**现状**:
- `ScopePolicy` 和 `DeploymentTarget` 从 domain 重新导出
- 添加 `ScopePolicyExt` trait 用于旧版 apply()

**质量**: 良好，迁移完成

---

### 11. writer.rs (158行) ⚠️ 需要整理

**现状**:
- `atomic_write()` 函数
- `hash_content()` 和 `hash_file()` 函数

**问题**:
- `atomic_write` 已在 `LocalFs` 中实现
- `hash_content` 已委托给 `ContentHash`

**建议**:
- 删除重复代码
- 保留 `hash_file()` 作为便捷函数（或移至 LocalFs）

---

## 总结

### 可直接复用 (保留或简单调整)
- `compile.rs` - 编译逻辑
- `conflict.rs` - 冲突解决 trait
- `execute.rs` - 执行策略
- `plan.rs` - 计划阶段
- `remote.rs` - rsync 远程同步

### 已完成迁移 (重新导出 domain 类型)
- `lockfile.rs` - LockfileNamespace, lockfile_key
- `scope.rs` - ScopePolicy, DeploymentTarget

### 需要整理
- `engine.rs` - 过大，与 DeployUseCase 重叠
- `orphan.rs` - 与 domain::services::orphan_detector 重叠
- `pipeline.rs` - 被 watcher 依赖，暂时保留
- `writer.rs` - 部分功能已在 LocalFs 中

### 建议执行顺序

1. **writer.rs** - 删除已迁移的重复代码
2. **orphan.rs** - 统一到 domain 层
3. **engine.rs** - 标记 deprecated，保留给 watcher
4. **pipeline.rs** - 保留给 watcher，不做改动
5. **最终目标** - 迁移 watcher 到新架构后，大量删除 sync/ 代码

