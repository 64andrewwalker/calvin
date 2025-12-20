# 接口定义 (Ports)

> **Updated**: 2025-12-20 (与实际代码同步)

Ports 是 Domain 层定义的接口，由 Infrastructure 层实现。

## 核心流程：Deploy

```
┌─────────────────────────────────────────────────────────────┐
│  User: `calvin deploy`                                      │
└─────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────┐
│  commands/deploy/cmd.rs                                     │
│  - 解析 CLI 参数                                             │
│  - 调用 presentation/factory 创建 DeployUseCase             │
└─────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────┐
│  application/deploy/use_case.rs                             │
│                                                             │
│  pub struct DeployUseCase<AR, LR, FS>                       │
│  where                                                      │
│      AR: AssetRepository,                                   │
│      LR: LockfileRepository,                                │
│      FS: FileSystem,                                        │
│                                                             │
│  fn execute(&self, options: &DeployOptions) -> DeployResult │
│  {                                                          │
│      // 1. 加载资产                                          │
│      let assets = self.asset_repo.load_all(&source)?;       │
│      // 2. 应用 Scope 策略                                   │
│      let assets = self.apply_scope_policy(assets, scope);   │
│      // 3. 编译 (通过 adapters)                              │
│      let outputs = self.compile_assets(&assets, &targets)?; │
│      // 4. 计划                                              │
│      let plan = self.plan_sync(&outputs, &lockfile, opts);  │
│      // 5. 解决冲突                                          │
│      let plan = self.resolve_conflicts(plan, &resolver)?;   │
│      // 6. 执行                                              │
│      self.execute_plan(&plan, &mut result);                 │
│      // 7. 更新锁文件                                        │
│      self.update_lockfile(&path, &plan, &result, scope);    │
│  }                                                          │
└─────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────┐
│  domain/services/                                           │
│  (纯领域逻辑，不依赖任何 I/O)                                │
│  - Planner::plan_file()                                     │
│  - OrphanDetector::detect()                                 │
│  - Differ::diff()                                           │
└─────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────┐
│  infrastructure/                                            │
│  - repositories/asset.rs: 从文件系统加载资产                 │
│  - repositories/lockfile.rs: 加载/保存 TOML 锁文件          │
│  - fs/local.rs: 本地文件操作                                │
│  - fs/remote.rs: SSH/rsync 远程操作                         │
└─────────────────────────────────────────────────────────────┘
```

## Port 接口定义

### AssetRepository

```rust
// domain/ports/asset_repository.rs
pub trait AssetRepository {
    /// Load all assets from a source directory
    fn load_all(&self, source: &Path) -> Result<Vec<Asset>>;

    /// Load a single asset by path
    fn load_by_path(&self, path: &Path) -> Result<Asset>;
}
```

**实现**: `infrastructure/repositories/asset.rs` → `FsAssetRepository`

### LockfileRepository

```rust
// domain/ports/lockfile_repository.rs
pub trait LockfileRepository {
    /// Load lockfile from path, or create a new empty one if not found
    fn load_or_new(&self, path: &Path) -> Lockfile;

    /// Load lockfile from path
    fn load(&self, path: &Path) -> Result<Lockfile, LockfileError>;

    /// Save lockfile to path
    fn save(&self, lockfile: &Lockfile, path: &Path) -> Result<(), LockfileError>;
}
```

**实现**: `infrastructure/repositories/lockfile.rs` → `TomlLockfileRepository`

### FileSystem

```rust
// domain/ports/file_system.rs
pub trait FileSystem {
    /// Read file content as string
    fn read(&self, path: &Path) -> FsResult<String>;

    /// Write content to file atomically
    fn write(&self, path: &Path, content: &str) -> FsResult<()>;

    /// Check if file exists
    fn exists(&self, path: &Path) -> bool;

    /// Remove a file
    fn remove(&self, path: &Path) -> FsResult<()>;

    /// Create directory and parents
    fn create_dir_all(&self, path: &Path) -> FsResult<()>;

    /// Compute content hash (SHA256)
    fn hash(&self, path: &Path) -> FsResult<String>;

    /// Expand ~ to home directory
    fn expand_home(&self, path: &Path) -> PathBuf;
}
```

**实现**: 
- `infrastructure/fs/local.rs` → `LocalFs`
- `infrastructure/fs/remote.rs` → `RemoteFs`

### TargetAdapter

```rust
// domain/ports/target_adapter.rs
pub trait TargetAdapter: Send + Sync {
    /// Target identifier for this adapter
    fn target(&self) -> Target;

    /// Adapter version (for cache invalidation)
    fn version(&self) -> u8 { 1 }

    /// Compile a single asset into platform-specific output files
    fn compile(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError>;

    /// Validate generated output against platform best practices
    fn validate(&self, output: &OutputFile) -> Vec<AdapterDiagnostic>;

    /// Post-compilation tasks (e.g., generate AGENTS.md)
    fn post_compile(&self, assets: &[Asset]) -> Result<Vec<OutputFile>, AdapterError> {
        Ok(Vec::new())
    }

    /// Generate security baseline configuration
    fn security_baseline(&self, config: &Config) -> Result<Vec<OutputFile>, AdapterError> {
        Ok(Vec::new())
    }

    /// Generate header/footer markers
    fn header(&self, source_path: &str) -> String;
    fn footer(&self, source_path: &str) -> String;
}
```

**实现**: `infrastructure/adapters/` 下的各个适配器

### ConflictResolver

```rust
// domain/ports/conflict_resolver.rs
pub trait ConflictResolver {
    /// Resolve a conflict
    fn resolve(&self, context: &ConflictContext) -> ConflictChoice;

    /// Show diff to user
    fn show_diff(&self, diff: &str);
}

pub enum ConflictChoice {
    Overwrite,
    Skip,
    Diff,
    Abort,
    OverwriteAll,
    SkipAll,
}
```

**实现**:
- `infrastructure/conflict/interactive.rs` → `InteractiveResolver`
- `domain/ports/conflict_resolver.rs` → `ForceResolver`, `SafeResolver`

### DeployEventSink

```rust
// domain/ports/deploy_events.rs
pub trait DeployEventSink: Send + Sync {
    fn on_event(&self, event: DeployEvent);
}

pub enum DeployEvent {
    Started { source: PathBuf, destination: String, asset_count: usize },
    Compiled { output_count: usize },
    FileWritten { index: usize, path: PathBuf },
    FileSkipped { index: usize, path: PathBuf, reason: String },
    FileError { index: usize, path: PathBuf, error: String },
    OrphansDetected { total: usize, safe_to_delete: usize },
    OrphanDeleted { path: PathBuf },
    Completed { written_count: usize, skipped_count: usize, error_count: usize, deleted_count: usize },
}
```

**实现**: `infrastructure/events/json.rs` → `JsonEventSink`

### SyncDestination

```rust
// domain/ports/sync_destination.rs
pub trait SyncDestination {
    fn sync(&self, outputs: &[OutputFile], options: &SyncOptions) -> Result<SyncResult, SyncDestinationError>;
}
```

**实现**: `infrastructure/sync/` 下的各个目标实现

### ConfigRepository

```rust
// domain/ports/config_repository.rs
pub trait ConfigRepository {
    fn load(&self, path: &Path) -> Result<Config, ConfigError>;
    fn save(&self, config: &Config, path: &Path) -> Result<(), ConfigError>;
}
```

**实现**: `infrastructure/config/toml_config.rs` → `TomlConfigRepository`

## 接口依赖关系

```
DeployUseCase
    ├── AssetRepository (加载资产)
    ├── LockfileRepository (锁文件管理)
    ├── FileSystem (文件操作)
    ├── TargetAdapter[] (编译)
    ├── ConflictResolver (冲突解决)
    └── DeployEventSink (事件报告)

CheckUseCase
    └── ConfigRepository (配置加载)

WatchUseCase
    ├── FileSystem (文件监控)
    └── DeployUseCase (增量部署)
```
