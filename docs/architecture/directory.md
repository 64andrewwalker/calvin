# 目录结构

## 代码目录

```
src/
├── main.rs                 # 入口
│
├── presentation/           # Layer 0: 表示层
│   ├── cli.rs              # CLI 定义 (clap)
│   ├── commands/           # 命令处理器
│   │   ├── deploy.rs       # 参数解析 + 调用 UseCase
│   │   ├── check.rs
│   │   ├── watch.rs
│   │   └── interactive.rs
│   ├── output/             # 输出格式化
│   │   ├── text.rs         # 终端输出
│   │   ├── json.rs         # JSON 输出
│   │   └── progress.rs     # 进度条
│   └── mod.rs
│
├── application/            # Layer 1: 应用层
│   ├── deploy_use_case.rs  # DeployUseCase
│   ├── check_use_case.rs   # CheckUseCase
│   ├── watch_use_case.rs   # WatchUseCase
│   ├── diff_use_case.rs    # DiffUseCase
│   └── mod.rs
│
├── domain/                 # Layer 2: 领域层 (核心!)
│   ├── entities/           # 实体
│   │   ├── asset.rs        # Asset (带生命周期的核心实体)
│   │   ├── output_file.rs  # OutputFile (编译产物)
│   │   └── lockfile.rs     # Lockfile (状态跟踪)
│   ├── value_objects/      # 值对象
│   │   ├── scope.rs        # Scope (user/project)
│   │   ├── target.rs       # Target (claude-code/cursor/...)
│   │   ├── hash.rs         # ContentHash
│   │   └── path.rs         # SafePath (验证过的路径)
│   ├── services/           # 领域服务
│   │   ├── compiler.rs     # 资产 → 输出 编译器
│   │   ├── planner.rs      # 计划阶段 (检测变化)
│   │   ├── differ.rs       # 差异计算
│   │   └── orphan.rs       # 孤儿检测
│   ├── policies/           # 策略
│   │   ├── scope_policy.rs # Scope 处理策略
│   │   └── security.rs     # 安全策略
│   ├── ports/              # 端口 (接口定义)
│   │   ├── asset_repo.rs   # trait AssetRepository
│   │   ├── lockfile_repo.rs# trait LockfileRepository
│   │   ├── file_system.rs  # trait FileSystem
│   │   └── target_adapter.rs # trait TargetAdapter
│   └── mod.rs
│
├── infrastructure/         # Layer 3: 基础设施层
│   ├── repositories/       # 仓储实现
│   │   ├── fs_asset_repo.rs    # 文件系统资产仓储
│   │   └── toml_lockfile_repo.rs # TOML 锁文件仓储
│   ├── adapters/           # 目标适配器
│   │   ├── claude_code.rs  # Claude Code 适配器
│   │   ├── cursor.rs       # Cursor 适配器
│   │   ├── vscode.rs       # VS Code 适配器
│   │   └── ...
│   ├── fs/                 # 文件系统
│   │   ├── local.rs        # 本地文件系统
│   │   └── remote.rs       # SSH 远程文件系统
│   ├── config/             # 配置
│   │   ├── loader.rs       # 配置加载
│   │   └── types.rs        # 配置类型
│   └── mod.rs
│
└── lib.rs                  # 库入口
```

