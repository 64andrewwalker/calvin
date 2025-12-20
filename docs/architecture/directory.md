# 目录结构

> **Updated**: 2025-12-20 (与实际代码同步)

## 代码目录

```
src/
├── main.rs                 # 入口
├── lib.rs                  # 库入口 (re-exports)
│
├── presentation/           # Layer 0: 表示层 (部分)
│   ├── cli.rs              # CLI 定义 (clap)
│   ├── factory.rs          # UseCase 工厂 (依赖注入)
│   ├── output.rs           # 输出渲染器
│   └── mod.rs
│
├── commands/               # Layer 0: 命令处理器 (独立于 presentation/)
│   ├── check/              # check 命令模块
│   │   ├── audit.rs
│   │   ├── doctor.rs
│   │   ├── engine.rs
│   │   └── mod.rs
│   ├── deploy/             # deploy 命令模块
│   │   ├── bridge.rs       # CLI → UseCase 桥接
│   │   ├── cmd.rs
│   │   ├── options.rs
│   │   ├── targets.rs
│   │   └── mod.rs
│   ├── interactive/        # 交互式模式模块
│   │   ├── menu.rs
│   │   ├── wizard.rs
│   │   ├── tests.rs
│   │   └── mod.rs
│   ├── debug.rs
│   ├── explain.rs
│   ├── watch.rs
│   └── mod.rs
│
├── ui/                     # Layer 0: UI 渲染 (独立模块)
│   ├── blocks/             # UI 块组件
│   ├── components/         # UI 组件
│   ├── primitives/         # UI 原语
│   ├── views/              # 视图渲染
│   ├── widgets/            # Widget
│   ├── context.rs          # UI 上下文
│   ├── terminal.rs         # 终端检测
│   ├── theme.rs            # 主题
│   └── mod.rs
│
├── application/            # Layer 1: 应用层
│   ├── deploy/             # DeployUseCase 模块
│   │   ├── options.rs      # 部署选项
│   │   ├── result.rs       # 部署结果
│   │   ├── use_case.rs     # 核心 UseCase
│   │   ├── tests.rs
│   │   └── mod.rs
│   ├── check.rs            # CheckUseCase
│   ├── compiler.rs         # compile_assets 服务
│   ├── diff.rs             # DiffUseCase
│   ├── pipeline.rs         # AssetPipeline
│   ├── watch.rs            # WatchUseCase
│   └── mod.rs
│
├── domain/                 # Layer 2: 领域层 (核心!)
│   ├── entities/           # 实体
│   │   ├── asset.rs        # Asset (核心实体)
│   │   ├── output_file.rs  # OutputFile (编译产物)
│   │   ├── lockfile.rs     # Lockfile (状态跟踪)
│   │   └── mod.rs
│   ├── value_objects/      # 值对象
│   │   ├── scope.rs        # Scope (user/project)
│   │   ├── target.rs       # Target (claude-code/cursor/...)
│   │   ├── hash.rs         # ContentHash
│   │   ├── path.rs         # SafePath
│   │   ├── lockfile_namespace.rs  # LockfileNamespace
│   │   └── mod.rs
│   ├── services/           # 领域服务
│   │   ├── compiler.rs     # 路径生成/Footer 生成
│   │   ├── planner.rs      # 计划阶段 (检测变化)
│   │   ├── differ.rs       # 差异计算
│   │   ├── orphan_detector.rs  # 孤儿检测
│   │   └── mod.rs
│   ├── policies/           # 策略
│   │   ├── scope_policy.rs # Scope 处理策略
│   │   ├── security.rs     # 安全策略
│   │   └── mod.rs
│   ├── ports/              # 端口 (接口定义)
│   │   ├── asset_repository.rs     # trait AssetRepository
│   │   ├── lockfile_repository.rs  # trait LockfileRepository
│   │   ├── file_system.rs          # trait FileSystem
│   │   ├── target_adapter.rs       # trait TargetAdapter
│   │   ├── conflict_resolver.rs    # trait ConflictResolver
│   │   ├── deploy_events.rs        # trait DeployEventSink
│   │   ├── sync_destination.rs     # trait SyncDestination
│   │   ├── config_repository.rs    # trait ConfigRepository
│   │   └── mod.rs
│   └── mod.rs
│
├── infrastructure/         # Layer 3: 基础设施层
│   ├── adapters/           # 目标适配器
│   │   ├── claude_code.rs
│   │   ├── cursor.rs
│   │   ├── vscode.rs
│   │   ├── antigravity.rs
│   │   ├── codex.rs
│   │   └── mod.rs
│   ├── repositories/       # 仓储实现
│   │   ├── asset.rs        # FsAssetRepository
│   │   ├── lockfile.rs     # TomlLockfileRepository
│   │   └── mod.rs
│   ├── fs/                 # 文件系统
│   │   ├── local.rs        # LocalFs
│   │   ├── remote.rs       # RemoteFs (SSH/rsync)
│   │   ├── destination.rs  # DestinationFs
│   │   └── mod.rs
│   ├── config/             # 配置实现
│   │   ├── toml_config.rs  # TomlConfigRepository
│   │   └── mod.rs
│   ├── conflict/           # 冲突解决
│   │   ├── interactive.rs  # InteractiveResolver
│   │   └── mod.rs
│   ├── events/             # 事件接收器
│   │   ├── json.rs         # JsonEventSink
│   │   └── mod.rs
│   ├── sync/               # 同步目标
│   │   ├── local.rs        # LocalProjectDestination, LocalHomeDestination
│   │   ├── remote.rs       # RemoteDestination
│   │   └── mod.rs
│   └── mod.rs
│
├── config/                 # 配置模块 (独立)
│   ├── types.rs            # 配置类型定义
│   ├── loader.rs           # 配置加载逻辑
│   ├── tests.rs
│   └── mod.rs
│
├── security/               # 安全模块 (独立)
│   ├── types.rs
│   ├── checks.rs
│   ├── report.rs
│   ├── tests.rs
│   └── mod.rs
│
├── watcher/                # 文件监控模块 (独立)
│   ├── cache.rs            # IncrementalCache
│   ├── event.rs            # WatchEvent
│   ├── sync.rs             # watch 函数
│   ├── tests.rs
│   └── mod.rs
│
├── sync/                   # 兼容层 (待移除)
│   ├── compile.rs          # 重导出 application::compile_assets
│   └── orphan.rs           # 兼容函数 detect_orphans/delete_orphans
│
├── error.rs                # 错误类型定义
├── fs.rs                   # FileSystem trait (legacy, 与 infrastructure/fs 重复)
├── models.rs               # PromptAsset 等模型 (legacy)
├── parser.rs               # frontmatter 解析 (legacy)
├── state.rs                # 状态管理 (legacy)
└── security_baseline.rs    # 安全基线 (legacy)
```

## 目录归属说明

| 目录/文件 | 层级 | 说明 |
|-----------|------|------|
| `presentation/` | Layer 0 | CLI 定义和工厂 |
| `commands/` | Layer 0 | 命令处理器 (理论上应在 presentation 下) |
| `ui/` | Layer 0 | UI 渲染组件 |
| `application/` | Layer 1 | UseCase 和应用服务 |
| `domain/` | Layer 2 | 领域逻辑 (核心) |
| `infrastructure/` | Layer 3 | 适配器和外部实现 |
| `config/`, `security/`, `watcher/` | 独立 | 功能模块 |
| `sync/` | 兼容层 | 待移除，保留向后兼容 |
| `*.rs` (根目录) | Legacy | 待迁移到合适位置 |

## 迁移说明

以下模块保留在根目录是历史原因，未来版本可能迁移：

- `models.rs` → `domain/entities/` (部分已迁移)
- `parser.rs` → `infrastructure/` 或 `domain/services/`
- `fs.rs` → 已由 `infrastructure/fs/` 取代，可考虑删除
- `state.rs` → 评估是否仍需要
