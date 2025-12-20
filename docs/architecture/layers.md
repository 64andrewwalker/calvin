# 分层架构

> **Updated**: 2025-12-21 (watcher 迁移到 application 层)

## 四层结构

```
┌─────────────────────────────────────────────────────────────┐
│  Layer 0: Presentation (表示层)                             │
│  - CLI 参数解析 (presentation/cli.rs)                       │
│  - 命令处理器 (commands/)                                   │
│  - UI 渲染 (ui/)                                            │
│  - 输出格式化 (presentation/output.rs)                      │
├─────────────────────────────────────────────────────────────┤
│  Layer 1: Application (应用层)                              │
│  - Use Cases: DeployUseCase, CheckUseCase, WatchUseCase     │
│  - Use Cases: DiffUseCase                                   │
│  - 服务: compile_assets, AssetPipeline                      │
│  - 编排业务流程，不包含业务规则                              │
├─────────────────────────────────────────────────────────────┤
│  Layer 2: Domain (领域层) ← 核心!                           │
│  - Entities: Asset, OutputFile, Lockfile                    │
│  - Value Objects: Scope, Target, Hash, SafePath             │
│  - Domain Services: Planner, OrphanDetector, Differ         │
│  - Policies: ScopePolicy, SecurityPolicy                    │
│  - Ports: 接口定义 (8 个 trait)                             │
├─────────────────────────────────────────────────────────────┤
│  Layer 3: Infrastructure (基础设施层)                        │
│  - Repositories: FsAssetRepository, TomlLockfileRepository  │
│  - Adapters: ClaudeCode, Cursor, VSCode, Antigravity, Codex │
│  - FileSystem: LocalFs, RemoteFs                            │
│  - Conflict: InteractiveResolver                            │
│  - Events: JsonEventSink                                    │
│  - Sync: LocalProjectDestination, RemoteDestination         │
└─────────────────────────────────────────────────────────────┘
```

## 依赖规则

```
Layer 0 → Layer 1 → Layer 2 ← Layer 3
              ↓           ↑
              └───────────┘
```

**关键点**：
- Layer 0 依赖 Layer 1
- Layer 1 依赖 Layer 2
- Layer 3 **实现** Layer 2 定义的接口
- Layer 2 **不依赖** Layer 3（通过接口反转）

## 各层职责

### Layer 0: Presentation

| 组件 | 位置 | 职责 |
|------|------|------|
| `cli.rs` | `presentation/` | CLI 参数定义 (clap) |
| `factory.rs` | `presentation/` | UseCase 工厂 (依赖注入) |
| `output.rs` | `presentation/` | 输出渲染 |
| `commands/` | `src/commands/` ¹ | 命令处理器 (参数解析 + 调用 UseCase) |
| `ui/` | `src/ui/` ¹ | UI 渲染组件 |

> ¹ **架构说明**: `commands/` 和 `ui/` 物理上位于 main crate（`src/main.rs` 声明），
> 逻辑上属于 Presentation 层。这是 Rust 项目的常见模式：lib crate 暴露核心功能，
> bin crate 处理 CLI 交互。考虑到迁移成本与收益比，保持现状是合理的设计决策。

### Layer 1: Application

| 组件 | 位置 | 职责 |
|------|------|------|
| `DeployUseCase` | `application/deploy/` | 编排部署流程 |
| `CheckUseCase` | `application/check.rs` | 编排安全检查流程 |
| `WatchUseCase` | `application/watch/` | 编排文件监控流程 (含增量编译缓存) |
| `DiffUseCase` | `application/diff.rs` | 编排差异预览流程 |
| `compile_assets` | `application/compiler.rs` | 资产编译服务 |
| `AssetPipeline` | `application/pipeline.rs` | 资产处理管道 |

### Layer 2: Domain

| 组件 | 位置 | 职责 |
|------|------|------|
| `entities/` | `domain/entities/` | 核心业务实体 (Asset, OutputFile, Lockfile) |
| `value_objects/` | `domain/value_objects/` | 不可变值对象 (Scope, Target, Hash) |
| `services/` | `domain/services/` | 无状态业务逻辑 (Planner, OrphanDetector) |
| `policies/` | `domain/policies/` | 业务规则/策略 (ScopePolicy) |
| `ports/` | `domain/ports/` | 接口定义 (8 个 trait) |

**Domain Ports (8 个接口):**
1. `AssetRepository` - 资产加载
2. `LockfileRepository` - 锁文件管理
3. `FileSystem` - 文件操作
4. `TargetAdapter` - 平台适配器
5. `ConflictResolver` - 冲突解决
6. `DeployEventSink` - 事件报告
7. `SyncDestination` - 同步目标
8. `ConfigRepository` - 配置管理

### Layer 3: Infrastructure

| 组件 | 位置 | 职责 |
|------|------|------|
| `repositories/` | `infrastructure/repositories/` | 实现 Domain 的仓储接口 |
| `adapters/` | `infrastructure/adapters/` | 目标平台适配器 (5 个) |
| `fs/` | `infrastructure/fs/` | 文件系统抽象 (Local, Remote) |
| `config/` | `infrastructure/config/` | 配置加载/保存 |
| `conflict/` | `infrastructure/conflict/` | 冲突解决器 |
| `events/` | `infrastructure/events/` | 事件接收器 |
| `sync/` | `infrastructure/sync/` | 同步目标实现 |

## 独立模块

以下模块不严格属于四层架构，但作为功能模块独立存在：

| 模块 | 位置 | 说明 |
|------|------|------|
| `config/` | `src/config/` | 配置类型和加载 |
| `security/` | `src/security/` | 安全检查逻辑 |

## Legacy 模块

以下模块是历史遗留，部分功能已迁移：

| 模块 | 状态 | 说明 |
|------|------|------|
| `models.rs` | 保留 | PromptAsset 等类型定义 |
| `parser.rs` | 保留 | frontmatter 解析 |
| `fs.rs` | 待评估 | FileSystem trait (与 infrastructure/fs 重复) |
