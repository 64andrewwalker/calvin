# 重构计划

> **Created**: 2025-12-20  
> **Status**: 分析完成

---

## 📊 当前状态

架构重构进度: **~95% 完成**

已完成:
- ✅ Domain 层 (entities, value objects, services, policies)
- ✅ Infrastructure 层 (adapters, repositories, file system)
- ✅ Application 层 (DeployUseCase, CheckUseCase, WatchUseCase, DiffUseCase)
- ✅ 命令集成 (deploy, check, diff 已使用新引擎)

---

## 🔴 高优先级：待清理的遗留代码

### 1. 重复的适配器模块

**问题**: 存在两套适配器实现
- `src/adapters/` - 遗留模块，使用 `PromptAsset` 和旧 trait
- `src/infrastructure/adapters/` - 新模块，使用 `domain::Asset` 和新 trait

**影响**: 代码重复 (~50%)，维护成本翻倍

**引用计数**:
- `src/adapters/` 被 legacy sync engine 使用 (16 个文件)
- `src/infrastructure/adapters/` 被新 UseCases 使用

**解决方案**:
```bash
# 阶段 1: 让 sync/engine.rs 使用新适配器
# 阶段 2: 迁移 pipeline.rs, compile.rs 等
# 阶段 3: 删除 src/adapters/
```

**预估时间**: 4-6 小时

---

### 2. 遗留 Sync 模块

**问题**: `src/sync/` 目录包含混合架构代码

| 文件 | 状态 | 说明 |
|------|------|------|
| `engine.rs` | 🔴 遗留 | 旧的 SyncEngine，仍被 DeployRunner 使用 |
| `pipeline.rs` | 🔴 遗留 | AssetPipeline，被 watch/diff 使用 |
| `compile.rs` | 🔴 遗留 | 编译逻辑，已迁移到 domain/services |
| `plan.rs` | 🟡 共用 | 规划逻辑，部分迁移到 domain/services/planner |
| `lockfile.rs` | 🟡 共用 | 锁文件逻辑，已有 domain/entities/lockfile |
| `orphan.rs` | 🟡 共用 | 孤儿检测，已有 domain/services/orphan_detector |
| `remote.rs` | 🔴 遗留 | rsync 逻辑，暂时保留 |
| `writer.rs` | 🟢 保留 | 原子写入，Infrastructure 层复用 |
| `conflict.rs` | 🟢 保留 | 冲突解决 UI，Presentation 层 |

**解决方案**:
1. 让 DeployRunner 使用 DeployUseCase
2. 删除不再使用的 sync 模块
3. 保留 writer.rs 和 conflict.rs

**预估时间**: 8-12 小时

---

### 3. 遗留命令处理器

**问题**: `src/commands/deploy/runner.rs` 仍然存在

| 文件 | 状态 |
|------|------|
| `runner.rs` | 🔴 遗留 DeployRunner |
| `bridge.rs` | 🟡 用于切换新旧引擎 |
| `cmd.rs` | 🟡 混合使用新旧引擎 |

**当前切换逻辑**:
- `CALVIN_LEGACY_ENGINE=1` → 使用旧 DeployRunner
- 默认 → 使用新 DeployUseCase

**解决方案**:
1. 确认新引擎稳定后，删除 runner.rs
2. 简化 cmd.rs，移除 bridge 逻辑

**预估时间**: 2-3 小时

---

## 🟡 中优先级：代码质量改进

### 4. #[allow(dead_code)] 标记

**问题**: 20 处 `#[allow(dead_code)]`，表示未使用的代码

**分布**:
| 位置 | 数量 | 原因 |
|------|------|------|
| `ui/` 模块 | 12 | 预留的 UI 组件 |
| `application/` | 2 | 未来功能预留 |
| `domain/` | 1 | 内部使用的方法 |
| `commands/` | 2 | 遗留函数标记 |

**解决方案**:
1. 删除确实无用的代码
2. 为预留功能添加 issue 跟踪
3. 保留合理的 dead_code 标记

**预估时间**: 1-2 小时

---

### 5. DiffUseCase 缺少 post_compile

**问题**: 新 DiffUseCase 不生成 AGENTS.md 等 post-compile 输出

**影响**: 
- 新引擎: 144 文件
- 旧引擎: 147 文件 (包含 AGENTS.md)

**解决方案**:
1. 在 DiffUseCase 添加 post_compile 支持
2. 或者在 TargetAdapter trait 中处理

**预估时间**: 2-3 小时

---

## 🟢 低优先级：未来改进

### 6. 跨平台兼容性

| 任务 | 状态 |
|------|------|
| Windows CI 测试 | 🔲 待开始 |
| WSL 兼容性测试 | 🔲 待开始 |
| rsync 替代方案 (Windows) | 🔲 待开始 |

### 7. 性能优化

| 任务 | 状态 |
|------|------|
| 大规模 PromptPack 测试 | 🔲 待开始 |
| 增量编译优化 | 🔲 待开始 |
| 并行适配器处理 | 🔲 待开始 |

---

## 📋 建议执行顺序

| 步骤 | 任务 | 优先级 | 时间 |
|------|------|--------|------|
| 1 | 添加 post_compile 到 DiffUseCase | 高 | 2h |
| 2 | 删除旧 DeployRunner 和 bridge | 高 | 2h |
| 3 | 合并适配器模块 | 高 | 6h |
| 4 | 清理 sync/ 遗留模块 | 中 | 8h |
| 5 | 清理 dead_code | 低 | 2h |

**总预估时间**: 20 小时

---

## 依赖关系图

```
DeployRunner (legacy)
    └─→ sync/engine.rs
         └─→ src/adapters/ (legacy)
         └─→ sync/pipeline.rs
              └─→ sync/compile.rs

DeployUseCase (new) ✅
    └─→ infrastructure/adapters/ ✅
    └─→ domain/services/ ✅
```

删除顺序:
1. DeployRunner → sync/engine.rs 不再需要
2. sync/engine.rs → src/adapters/ 不再需要
3. src/adapters/ → 删除

---

## 文件删除清单 (待确认)

删除后节省约 3000+ 行代码:

```
src/adapters/                  # 整个目录 (~1500 行)
src/commands/deploy/runner.rs  # (~600 行)
src/sync/engine.rs             # 部分功能 (~400 行)
src/sync/compile.rs            # (~200 行)
src/sync/pipeline.rs           # (~200 行)
```

**注意**: 删除前需确认所有功能已迁移并测试通过

---

## 更新日志

| 日期 | 更新 |
|------|------|
| 2025-12-20 | 初始分析完成 |

