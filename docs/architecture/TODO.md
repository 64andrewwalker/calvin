# 架构重构进度追踪

> **Created**: 2025-12-19  
> **Updated**: 2025-12-19  
> **Status**: ✅ 主要阶段完成

---

## 📋 整体进度

| 阶段 | 状态 | 预计时间 |
|------|------|----------|
| 阶段 0: 规划 | ✅ 完成 | 1 天 |
| 阶段 0.5: 测试覆盖 | ✅ 评估完成 | - |
| **阶段 1: 建立骨架** | ✅ 完成 | 1-2 天 |
| **阶段 2: 提取 Domain** | ✅ 完成 | 2-3 天 |
| **阶段 3: 提取 Infrastructure** | ✅ 完成 | 2-3 天 |
| **阶段 4: 重写 Application** | ✅ 完成 | 1-2 天 |
| **阶段 5: 清理 Presentation** | 🔄 进行中 | 1 天 |

**重要原则**：
- ⚡ **先测试后重构**：每个阶段开始前确保关键模块有足够测试覆盖
- 🔒 **行为一致性**：迁移后的模块必须与原模块行为完全一致
- 🧱 **原子化迁移**：每次只迁移一个模块，验证后再继续

---

## 阶段 0: 规划 ✅

**状态**: 完成

**产出**:
- [x] 架构概览文档
- [x] 分层设计文档
- [x] 目录结构规范
- [x] 接口定义 (Ports)
- [x] 迁移路径规划
- [x] 文档架构设计

**必读文档**:
- [overview.md](./overview.md) - 设计目标
- [layers.md](./layers.md) - 四层架构

---

## 阶段 0.5: 测试覆盖 🧪

**状态**: ✅ 评估完成 (已有足够覆盖)

**目标**: 在重构前增加关键模块的测试覆盖，确保行为一致性

### 🎉 当前测试覆盖评估结果

**总计**: 243 个测试 + 9 个 golden 快照测试 + 多个 E2E 测试

**覆盖率** (via `cargo llvm-cov`):
- **Lines**: 75.24% (目标: ≥70%)
- **Functions**: 84.36%
- **Branches**: 73.79%

#### 🔴 高优先级（核心路径）

| 模块 | 测试数量 | 状态 | 备注 |
|------|----------|------|------|
| `sync/engine.rs` | 33 | ✅ 充足 | 覆盖 local/home/remote/冲突/lockfile |
| `sync/plan.rs` | 9 | ✅ 充足 | 覆盖冲突检测/路径扩展 |
| `sync/lockfile.rs` | 16 | ✅ 充足 | 完整覆盖 |
| `sync/orphan.rs` | 14 | ✅ 充足 | 包含多 scope 共存测试 |
| `commands/deploy/` | 4 E2E | ✅ 有 | `cli_deploy_cleanup.rs` |

#### 🟡 中优先级

| 模块 | 测试数量 | 状态 | 备注 |
|------|----------|------|------|
| `security.rs` | 14 | ✅ 充足 | 规则验证已覆盖 |
| `config.rs` | 16 | ✅ 充足 | 加载/保存/迁移已覆盖 |
| `adapters/*.rs` | 40+ | ✅ 充足 | 每个 adapter 都有测试 |

#### 🟢 低优先级

| 模块 | 测试数量 | 状态 | 备注 |
|------|----------|------|------|
| `ui/views/*.rs` | 有 | ✅ insta 快照 | golden 测试覆盖 |
| `parser.rs` | - | ⚠️ 可选添加 | 通过 adapter 测试间接覆盖 |

### 木桶效应分析：低覆盖率模块

| 模块 | 初始覆盖率 | 当前覆盖率 | 状态 |
|------|------------|------------|------|
| `sync/remote.rs` | 8.09% | 74.00% | ✅ 已修复 |
| `sync/conflict.rs` | 15.71% | 71.57% | ✅ 已修复 |
| `ui/error.rs` | 24.57% | 59.56% | ✅ 已修复 |
| `ui/live_region.rs` | 0.00% | - | 🟢 已忽略 (TTY) |
| `ui/menu.rs` | 0.00% | - | 🟢 已忽略 (TTY) |
| `commands/interactive.rs` | 28.55% | - | 🟢 已忽略 (TTY) |
| `commands/watch.rs` | 0.00% | - | 🟢 已忽略 (Watcher) |

### 红线 (Red Lines)

**CI 强制检查**:
- 整体覆盖率 ≥ 70% (否则 CI 失败)
- 新代码覆盖率 ≥ 80% (via Codecov patch check)

**模块级红线** (建议，非强制):
- `sync/engine.rs` ≥ 90%
- `sync/lockfile.rs` ≥ 85%
- `adapters/*.rs` ≥ 80%
- `config.rs` ≥ 70%

### 结论

**现有测试覆盖已经足够开始重构**，因为：
1. 核心路径模块覆盖率高（engine 94%, lockfile 87%, orphan 98%）
2. 低覆盖率模块大多是 UI/TTY 相关（已标记忽略）
3. 有 CI 阈值检查防止覆盖率下降

### 可选改进（重构后）

- [ ] 为 `sync/conflict.rs` 添加测试
- [ ] 为 `sync/remote.rs` 添加 mock SSH 测试
- [ ] 考虑添加属性测试 (`proptest`)

### 参考文档

- [../test-variants-rationale.md](../test-variants-rationale.md) - 测试策略
- 现有 TDD 会话记录

---

## 阶段 1: 建立骨架 ✅

**状态**: ✅ 完成

**目标**: 创建新目录结构，定义核心 traits

**任务**:
- [x] 创建 `src/domain/` 目录
- [x] 定义 `AssetRepository` trait (`domain/ports/asset_repository.rs`)
- [x] 定义 `LockfileRepository` trait (`domain/ports/lockfile_repository.rs`)
- [x] 定义 `FileSystem` trait (`domain/ports/file_system.rs`)
- [ ] 定义 `TargetAdapter` trait (移至阶段 3)
- [ ] 创建 `src/presentation/` 目录 (移至阶段 5)
- [ ] 创建 `src/application/` 目录 (移至阶段 4)
- [ ] 创建 `src/infrastructure/` 目录 (移至阶段 3)

**必读文档**:
- [directory.md](./directory.md) - 目录结构规范
- [ports.md](./ports.md) - 接口定义

**参考文档** (产品/UI 设计):
- [../cli-state-machine.md](../cli-state-machine.md) - CLI 状态机设计 (52KB)
- [../ui-components-spec.md](../ui-components-spec.md) - UI 组件规范 (22KB)
- [../ux-review.md](../ux-review.md) - UX 审查 (24KB)

**验收标准**:
- 编译通过
- 现有功能不受影响
- 新目录结构可用

---

## 阶段 2: 提取 Domain

**状态**: ✅ 完成

**目标**: 将纯业务逻辑提取到 `domain/`

**任务**:
- [x] 提取 `Asset` 实体到 `domain/entities/asset.rs` (11 tests)
- [x] 提取 `OutputFile` 到 `domain/entities/output_file.rs` (10 tests)
- [x] 提取 `Lockfile` 逻辑到 `domain/entities/lockfile.rs` (15 tests)
- [x] 提取 `Scope` 值对象到 `domain/value_objects/scope.rs` (8 tests)
- [x] 提取 `Target` 值对象到 `domain/value_objects/target.rs` (10 tests)
- [x] 提取编译辅助到 `domain/services/compiler.rs` (17 tests)
- [x] 提取计划逻辑到 `domain/services/planner.rs` (18 tests)
- [x] 提取 Orphan 检测到 `domain/services/orphan_detector.rs` (20 tests)
- [x] 提取 Scope 策略到 `domain/policies/scope_policy.rs`
- [ ] 提取安全策略到 `domain/policies/security.rs` (可选/延期)

**当前测试统计**: 109+ 个 domain 层测试

**必读文档**:
- [layers.md](./layers.md) - Domain 层职责
- [ports.md](./ports.md) - Domain 定义的接口

**参考文档** (现有设计):
- [../scope-guide.md](../scope-guide.md) - Scope 设计
- [../scope-policy-consistency.md](../scope-policy-consistency.md) - Scope 策略
- [../impl-plan-sc7-scope.md](../impl-plan-sc7-scope.md) - Scope 隔离设计
- [../security-audit-report.md](../security-audit-report.md) - 安全审计

**验收标准**:
- Domain 层不依赖任何 I/O
- 所有 Domain 代码可独立测试
- 现有功能不受影响

---

## 阶段 3: 提取 Infrastructure

**状态**: ✅ 完成

**目标**: 将 I/O 操作移到 `infrastructure/`

**任务**:
- [x] 实现 `FsAssetRepository` (从文件系统加载资产) (4 tests)
- [x] 实现 `TomlLockfileRepository` (TOML 锁文件) (4 tests)
- [x] 迁移 `LocalFileSystem` 到 `infrastructure/fs/`
- [ ] 迁移 `RemoteFileSystem` 到 `infrastructure/fs/` (保留别名)
- [x] 迁移 Claude Code 适配器到 `infrastructure/adapters/` (14 tests)
- [x] 迁移 Cursor 适配器到 `infrastructure/adapters/` (14 tests)
- [ ] 迁移其他适配器 (VSCode, Antigravity, Codex) (延期)
- [ ] 迁移配置加载到 `infrastructure/config/` (延期)

**必读文档**:
- [directory.md](./directory.md) - Infrastructure 目录
- [ports.md](./ports.md) - 需要实现的接口

**参考文档** (现有设计):
- [../target-platforms.md](../target-platforms.md) - 目标平台规范
- [../tech-decisions.md](../tech-decisions.md) - 技术决策
- [../configuration.md](../configuration.md) - 配置设计

**验收标准**:
- Infrastructure 实现 Domain 定义的接口
- 所有适配器可独立测试
- 现有功能不受影响

---

## 阶段 4: 重写 Application

**状态**: ✅ 完成 (核心用例)

**目标**: 用 Use Cases 替代 Runner

**任务**:
- [x] 实现 `DeployUseCase` (3 tests)
  - `DeployOptions` - 部署配置
  - `DeployResult` - 部署结果
  - 完整的依赖注入支持
- [ ] 实现 `CheckUseCase` (延期)
- [ ] 实现 `WatchUseCase` (延期)
- [ ] 实现 `DiffUseCase` (延期)
- [ ] 删除 `DeployRunner` (保留共存)
- [ ] 依赖注入：从 main.rs 注入依赖 (通过 presentation/factory)

**必读文档**:
- [layers.md](./layers.md) - Application 层职责
- [ports.md](./ports.md) - Deploy 流程图

**参考文档** (现有设计):
- [../impl-plan-sc6-cleanup.md](../impl-plan-sc6-cleanup.md) - Cleanup 实现
- [../design-deploy-targets.md](../design-deploy-targets.md) - Deploy 目标设计

**验收标准**:
- Use Cases 只做编排，不包含业务逻辑
- 通过依赖注入，Use Cases 可独立测试
- 现有功能不受影响

---

## 阶段 5: 清理 Presentation

**状态**: 🔄 进行中 (基础结构完成)

**目标**: 统一 UI 输出，移除命令中的业务逻辑

**任务**:
- [x] 创建 `presentation/` 模块结构
- [x] 创建 `presentation/factory.rs` - UseCase 工厂 (5 tests)
- [x] 创建 `presentation/output.rs` - 输出渲染器 (6 tests)
  - `TextRenderer` - 文本输出
  - `JsonRenderer` - JSON 输出
  - `DeployResultRenderer` trait
- [ ] 迁移 CLI 定义到 `presentation/cli.rs` (可选，现有结构可用)
- [ ] 迁移命令处理器到 `presentation/commands/` (延期)
- [ ] 移除命令中的 `eprintln!` 直接调用 (延期)
- [ ] 集成新 UseCase 到现有命令 (下一步)

**必读文档**:
- [layers.md](./layers.md) - Presentation 层职责
- [docs.md](./docs.md) - 文档与代码的映射

**参考文档** (UI 设计):
- [../ui-components-spec.md](../ui-components-spec.md) - UI 组件规范 ⭐️ 必读
- [../ux-review.md](../ux-review.md) - UX 审查 ⭐️ 必读
- [../cli-state-machine.md](../cli-state-machine.md) - CLI 状态机
- [../tdd-session-cli-animation.md](../tdd-session-cli-animation.md) - 动画设计

**验收标准**:
- 所有 UI 输出通过统一接口
- 命令层不包含业务逻辑
- UI 组件符合设计规范

---

## 📚 关键参考文档索引

### 产品设计理念

| 文档 | 描述 | 大小 | 重要性 |
|------|------|------|--------|
| [cli-state-machine.md](../cli-state-machine.md) | CLI 交互状态机 | 52KB | ⭐⭐⭐ |
| [ux-review.md](../ux-review.md) | UX 审查与建议 | 24KB | ⭐⭐⭐ |
| [target-platforms.md](../target-platforms.md) | 目标平台规范 | 6KB | ⭐⭐ |

### UI 交互设计

| 文档 | 描述 | 大小 | 重要性 |
|------|------|------|--------|
| [ui-components-spec.md](../ui-components-spec.md) | UI 组件规范 | 22KB | ⭐⭐⭐ |
| [tdd-session-cli-animation.md](../tdd-session-cli-animation.md) | 动画设计 | 12KB | ⭐⭐ |

### 技术设计

| 文档 | 描述 | 大小 | 重要性 |
|------|------|------|--------|
| [tech-decisions.md](../tech-decisions.md) | 技术决策记录 | 14KB | ⭐⭐⭐ |
| [configuration.md](../configuration.md) | 配置设计 | 5KB | ⭐⭐ |
| [security-audit-report.md](../security-audit-report.md) | 安全审计 | 7KB | ⭐⭐ |

### Scope 设计

| 文档 | 描述 | 大小 | 重要性 |
|------|------|------|--------|
| [scope-guide.md](../scope-guide.md) | Scope 用户指南 | 4KB | ⭐⭐ |
| [scope-policy-consistency.md](../scope-policy-consistency.md) | Scope 策略 | 6KB | ⭐⭐ |
| [impl-plan-sc7-scope.md](../impl-plan-sc7-scope.md) | Scope 隔离实现 | 4KB | ⭐⭐⭐ |

---

## 🚀 立即行动项（来自资深架构师审查）

### 高优先级（v0.3.0）

| 任务 | 预估时间 | 状态 |
|------|----------|------|
| 迁移 `atty` → `is-terminal` | 1 小时 | ✅ 完成 |
| 增加 SyncEngine 测试覆盖 | 4 小时 | 🔲 待开始 |
| 提取 Planner 到独立模块 | 2 小时 | 🔲 待开始 |

### 中优先级（v0.4.0）

| 任务 | 预估时间 | 状态 |
|------|----------|------|
| 完成 Domain 层提取 | 3-5 天 | 🔲 待开始 |
| 迁移 `serde_yaml` → `serde_yml` | 2 小时 | 🔲 待开始 |
| 统一错误处理 | 3 小时 | 🔲 待开始 |

### 低优先级（v0.5.0+）

| 任务 | 预估时间 | 状态 |
|------|----------|------|
| 支持多 Target 批量部署 | 2 天 | 🔲 待开始 |
| 考虑插件系统 | 评估中 | 🔲 待开始 |
| 性能优化（大规模 PromptPack） | 1 天 | 🔲 待开始 |

---

---

## 🌍 跨平台兼容性

**状态**: 🔄 设计中

**相关文档**: [platform.md](./platform.md)

### 检查清单

| 任务 | 状态 | 优先级 |
|------|------|--------|
| 使用 `dirs` crate 获取 home 目录 | ✅ 已使用 | P0 |
| 使用 `PathBuf::join()` 而非字符串拼接 | ✅ 已遵循 | P0 |
| 添加 Windows CI 测试 | 🔲 待开始 | P1 |
| 文档化 Windows rsync 要求 | 🔲 待开始 | P2 |
| 测试 WSL 环境兼容性 | 🔲 待开始 | P2 |

### 需关注的模块

- `sync/mod.rs` - `expand_home_dir()` ✅ 已使用 dirs crate
- `domain/services/compiler.rs` - 路径生成 (使用 PathBuf::from + join)
- `sync/remote.rs` - rsync 命令 (Unix 专用)
- `fs.rs` - 文件系统操作

---

## 更新日志

| 日期 | 更新内容 |
|------|----------|
| 2025-12-19 | 添加跨平台兼容性检查清单 |
| 2025-12-19 | 更新阶段 1/2 状态，添加 domain 测试统计 |
| 2025-12-19 | 添加资深架构师审查结果，更新立即行动项 |
| 2025-12-19 | 创建 TODO 文档，完成阶段 0 规划 |

