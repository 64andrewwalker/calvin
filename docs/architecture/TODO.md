# 架构重构进度追踪

> **Created**: 2025-12-19  
> **Status**: 规划中

---

## 📋 整体进度

| 阶段 | 状态 | 预计时间 |
|------|------|----------|
| 阶段 0: 规划 | ✅ 完成 | 1 天 |
| 阶段 1: 建立骨架 | 🔲 待开始 | 1-2 天 |
| 阶段 2: 提取 Domain | 🔲 待开始 | 2-3 天 |
| 阶段 3: 提取 Infrastructure | 🔲 待开始 | 2-3 天 |
| 阶段 4: 重写 Application | 🔲 待开始 | 1-2 天 |
| 阶段 5: 清理 Presentation | 🔲 待开始 | 1 天 |

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

## 阶段 1: 建立骨架

**状态**: 🔲 待开始

**目标**: 创建新目录结构，定义核心 traits

**任务**:
- [ ] 创建 `src/presentation/` 目录
- [ ] 创建 `src/application/` 目录
- [ ] 创建 `src/domain/` 目录
- [ ] 创建 `src/infrastructure/` 目录
- [ ] 定义 `AssetRepository` trait
- [ ] 定义 `LockfileRepository` trait
- [ ] 定义 `FileSystem` trait (已有，需迁移)
- [ ] 定义 `TargetAdapter` trait

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

**状态**: 🔲 待开始

**目标**: 将纯业务逻辑提取到 `domain/`

**任务**:
- [ ] 提取 `Asset` 实体到 `domain/entities/`
- [ ] 提取 `OutputFile` 到 `domain/entities/`
- [ ] 提取 `Lockfile` 逻辑到 `domain/entities/`
- [ ] 提取 `Scope` 值对象到 `domain/value_objects/`
- [ ] 提取 `Target` 值对象到 `domain/value_objects/`
- [ ] 提取编译逻辑到 `domain/services/compiler.rs`
- [ ] 提取计划逻辑到 `domain/services/planner.rs`
- [ ] 提取 Orphan 检测到 `domain/services/orphan.rs`
- [ ] 提取安全策略到 `domain/policies/security.rs`
- [ ] 提取 Scope 策略到 `domain/policies/scope_policy.rs`

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

**状态**: 🔲 待开始

**目标**: 将 I/O 操作移到 `infrastructure/`

**任务**:
- [ ] 实现 `FsAssetRepository` (从文件系统加载资产)
- [ ] 实现 `TomlLockfileRepository` (TOML 锁文件)
- [ ] 迁移 `LocalFileSystem` 到 `infrastructure/fs/`
- [ ] 迁移 `RemoteFileSystem` 到 `infrastructure/fs/`
- [ ] 迁移所有适配器到 `infrastructure/adapters/`
- [ ] 迁移配置加载到 `infrastructure/config/`

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

**状态**: 🔲 待开始

**目标**: 用 Use Cases 替代 Runner

**任务**:
- [ ] 实现 `DeployUseCase`
- [ ] 实现 `CheckUseCase`
- [ ] 实现 `WatchUseCase`
- [ ] 实现 `DiffUseCase`
- [ ] 删除 `DeployRunner` (或重命名)
- [ ] 依赖注入：从 main.rs 注入依赖

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

**状态**: 🔲 待开始

**目标**: 统一 UI 输出，移除命令中的业务逻辑

**任务**:
- [ ] 迁移 CLI 定义到 `presentation/cli.rs`
- [ ] 迁移命令处理器到 `presentation/commands/`
- [ ] 统一输出接口 (`text.rs`, `json.rs`)
- [ ] 移除命令中的 `eprintln!` 直接调用
- [ ] 实现 `Renderer` trait

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

## 更新日志

| 日期 | 更新内容 |
|------|----------|
| 2025-12-19 | 创建 TODO 文档，完成阶段 0 规划 |

