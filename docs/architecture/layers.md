# 分层架构

## 四层结构

```
┌─────────────────────────────────────────────────────────────┐
│  Layer 0: Presentation (表示层)                             │
│  - CLI 参数解析                                              │
│  - 交互式菜单                                                │
│  - JSON 输出                                                 │
│  - 进度条/UI 渲染                                            │
├─────────────────────────────────────────────────────────────┤
│  Layer 1: Application (应用层)                              │
│  - Use Cases: DeployUseCase, CheckUseCase, WatchUseCase     │
│  - 编排业务流程                                              │
│  - 不包含业务规则                                            │
├─────────────────────────────────────────────────────────────┤
│  Layer 2: Domain (领域层) ← 核心!                           │
│  - Entities: Asset, OutputFile, Lockfile                    │
│  - Value Objects: Scope, Target, Hash                       │
│  - Domain Services: Compiler, Planner, Differ               │
│  - Policies: ScopePolicy, SecurityPolicy                    │
├─────────────────────────────────────────────────────────────┤
│  Layer 3: Infrastructure (基础设施层)                        │
│  - Repositories: AssetRepository, LockfileRepository        │
│  - Adapters: ClaudeAdapter, CursorAdapter...                │
│  - External: FileSystem, SSH, Config                        │
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

| 组件 | 职责 |
|------|------|
| `cli.rs` | CLI 参数定义 (clap) |
| `commands/` | 参数解析 + 调用 UseCase |
| `output/` | 终端/JSON 输出格式化 |

### Layer 1: Application

| 组件 | 职责 |
|------|------|
| `DeployUseCase` | 编排部署流程 |
| `CheckUseCase` | 编排安全检查流程 |
| `WatchUseCase` | 编排文件监控流程 |

### Layer 2: Domain

| 组件 | 职责 |
|------|------|
| `entities/` | 核心业务实体 |
| `value_objects/` | 不可变值对象 |
| `services/` | 无状态业务逻辑 |
| `policies/` | 业务规则/策略 |
| `ports/` | 接口定义 |

### Layer 3: Infrastructure

| 组件 | 职责 |
|------|------|
| `repositories/` | 实现 Domain 的仓储接口 |
| `adapters/` | 目标平台适配器 |
| `fs/` | 文件系统抽象 |
| `config/` | 配置加载/保存 |

