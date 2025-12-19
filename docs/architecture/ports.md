# 接口定义 (Ports)

Ports 是 Domain 层定义的接口，由 Infrastructure 层实现。

## 核心流程：Deploy

```
┌─────────────────────────────────────────────────────────────┐
│  User: `calvin deploy`                                      │
└─────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────┐
│  presentation/commands/deploy.rs                            │
│  - 解析 CLI 参数                                             │
│  - 创建 DeployUseCase 并调用 execute()                       │
└─────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────┐
│  application/deploy_use_case.rs                             │
│                                                             │
│  pub struct DeployUseCase<AR, LR, FS, TA>                   │
│  where                                                      │
│      AR: AssetRepository,                                   │
│      LR: LockfileRepository,                                │
│      FS: FileSystem,                                        │
│      TA: TargetAdapter,                                     │
│                                                             │
│  fn execute(&self, options: DeployOptions) -> Result<...>   │
│  {                                                          │
│      // 1. 加载资产                                          │
│      let assets = self.asset_repo.load_all()?;              │
│      // 2. 编译                                              │
│      let outputs = Compiler::compile(&assets, &adapters)?;  │
│      // 3. 计划                                              │
│      let plan = Planner::plan(&outputs, &lockfile)?;        │
│      // 4. 执行                                              │
│      let result = self.file_system.execute(&plan)?;         │
│      // 5. 更新锁文件                                        │
│      self.lockfile_repo.save(&updated_lockfile)?;           │
│  }                                                          │
└─────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────┐
│  domain/services/                                           │
│  (纯领域逻辑，不依赖任何 I/O)                                │
└─────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────┐
│  infrastructure/                                            │
│  - fs_asset_repo: 从文件系统加载资产                         │
│  - toml_lockfile_repo: 加载/保存 TOML 锁文件                 │
│  - local_fs / remote_fs: 写入文件                           │
└─────────────────────────────────────────────────────────────┘
```

## AssetRepository

```rust
// domain/ports/asset_repo.rs
pub trait AssetRepository {
    fn load_all(&self, source: &Path) -> Result<Vec<Asset>>;
    fn load_by_path(&self, path: &Path) -> Result<Asset>;
}
```

## LockfileRepository

```rust
// domain/ports/lockfile_repo.rs
pub trait LockfileRepository {
    fn load(&self, path: &Path) -> Result<Lockfile>;
    fn save(&self, lockfile: &Lockfile, path: &Path) -> Result<()>;
}
```

## FileSystem

```rust
// domain/ports/file_system.rs
pub trait FileSystem {
    fn read(&self, path: &Path) -> Result<String>;
    fn write(&self, path: &Path, content: &str) -> Result<()>;
    fn exists(&self, path: &Path) -> bool;
    fn remove(&self, path: &Path) -> Result<()>;
}
```

## TargetAdapter

```rust
// domain/ports/target_adapter.rs
pub trait TargetAdapter {
    fn target(&self) -> Target;
    fn compile(&self, assets: &[Asset]) -> Result<Vec<OutputFile>>;
    fn output_path(&self, asset: &Asset) -> PathBuf;
}
```

