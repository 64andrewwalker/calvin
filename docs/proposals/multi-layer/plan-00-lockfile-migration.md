# Phase 0: Lockfile Migration

> **Priority**: Must complete first  
> **Estimated Effort**: 2-3 days  
> **Breaking Change**: Yes (with auto-migration)
> **Architecture Reference**: `docs/architecture/layers.md`

## Objective

将 lockfile 从 `.promptpack/.calvin.lock` 迁移到 `./calvin.lock`，并扩展格式支持来源追踪。

## Architecture Compliance

按四层架构分布改动：

```
Layer 2 (Domain):
└── domain/entities/lockfile.rs    # 扩展 LockfileEntry (~20 lines 新增)
    - 新增 OutputProvenance 结构
    - 扩展 with_provenance() 方法

Layer 3 (Infrastructure):
└── infrastructure/repositories/lockfile.rs  # 更新序列化 (~30 lines 修改)
    - 支持新字段的 serde 序列化
    - 向后兼容旧格式

Layer 1 (Application):
└── application/deploy/use_case.rs  # 迁移逻辑 (~30 lines 新增)
    - get_lockfile_path() 方法更新
    - migrate_lockfile() 新增
```

**关键约束**:
- `LockfileEntry` 是纯数据实体，迁移逻辑不在 entity 中
- 迁移逻辑在 Application 层，因为涉及业务决策
- Infrastructure 层只负责文件读写

## Background

### Current Implementation

```
路径: .promptpack/.calvin.lock
格式: 
  version = 1
  [files."project:path"]
  hash = "sha256:..."
```

### Target Implementation

```
路径: ./calvin.lock
格式:
  version = 1
  [files."project:path"]
  hash = "sha256:..."
  source_layer = "user"
  source_layer_path = "~/.calvin/.promptpack"
  source_asset = "review"
  source_file = "~/.calvin/.promptpack/actions/review.md"
  overrides = null
```

## Detailed Tasks

### Task 0.1: Extend LockfileEntry

**File**: `src/domain/entities/lockfile.rs`

```rust
// 扩展结构
pub struct LockfileEntry {
    hash: String,
    source_layer: Option<String>,
    source_layer_path: Option<PathBuf>,
    source_asset: Option<String>,
    source_file: Option<PathBuf>,
    overrides: Option<String>,
}
```

**Tests**:
```rust
#[test]
fn lockfile_entry_with_provenance() {
    let entry = LockfileEntry::with_provenance(
        "sha256:abc",
        OutputProvenance {
            source_layer: "user".to_string(),
            source_layer_path: PathBuf::from("~/.calvin/.promptpack"),
            source_asset: "review".to_string(),
            source_file: PathBuf::from("~/.calvin/.promptpack/actions/review.md"),
            overrides: None,
        },
    );
    assert_eq!(entry.source_layer(), Some("user"));
}
```

### Task 0.2: Update Serialization

**File**: `src/infrastructure/repositories/lockfile.rs`

```rust
#[derive(Serialize, Deserialize)]
struct TomlFileEntry {
    hash: String,
    #[serde(default)]
    source_layer: Option<String>,
    #[serde(default)]
    source_layer_path: Option<PathBuf>,
    #[serde(default)]
    source_asset: Option<String>,
    #[serde(default)]
    source_file: Option<PathBuf>,
    #[serde(default)]
    overrides: Option<String>,
}
```

**Tests**:
```rust
#[test]
fn deserialize_old_format() {
    let content = r#"
        version = 1
        [files."project:test.md"]
        hash = "sha256:abc"
    "#;
    let lockfile = parse_lockfile(content);
    let entry = lockfile.get("project:test.md").unwrap();
    assert_eq!(entry.hash(), "sha256:abc");
    assert_eq!(entry.source_layer(), None); // 旧格式没有这个字段
}

#[test]
fn deserialize_new_format() {
    let content = r#"
        version = 1
        [files."project:test.md"]
        hash = "sha256:abc"
        source_layer = "user"
        source_asset = "review"
    "#;
    let lockfile = parse_lockfile(content);
    let entry = lockfile.get("project:test.md").unwrap();
    assert_eq!(entry.source_layer(), Some("user"));
}
```

### Task 0.3: Update Path Logic

**File**: `src/application/deploy/use_case.rs`

```rust
// 旧代码
fn get_lockfile_path(&self, source: &Path, _scope: Scope) -> PathBuf {
    source.join(".calvin.lock")
}

// 新代码
fn get_lockfile_path(&self, project_root: &Path) -> PathBuf {
    let new_path = project_root.join("calvin.lock");
    let old_path = project_root.join(".promptpack/.calvin.lock");
    
    if new_path.exists() {
        return new_path;
    }
    
    if old_path.exists() {
        // 自动迁移
        self.migrate_lockfile(&old_path, &new_path);
        return new_path;
    }
    
    new_path
}

fn migrate_lockfile(&self, old_path: &Path, new_path: &Path) {
    let lockfile = self.lockfile_repo.load_or_new(old_path);
    self.lockfile_repo.save(&lockfile, new_path).ok();
    std::fs::remove_file(old_path).ok();
    eprintln!("ℹ Migrated lockfile to {}", new_path.display());
}
```

**Tests**:
```rust
#[test]
fn migrate_lockfile_from_old_location() {
    let dir = tempdir().unwrap();
    let old_path = dir.path().join(".promptpack/.calvin.lock");
    let new_path = dir.path().join("calvin.lock");
    
    // 创建旧位置 lockfile
    fs::create_dir_all(old_path.parent().unwrap()).unwrap();
    fs::write(&old_path, "version = 1\n").unwrap();
    
    // 触发迁移
    let use_case = create_deploy_use_case();
    let lockfile_path = use_case.get_lockfile_path(dir.path());
    
    assert_eq!(lockfile_path, new_path);
    assert!(new_path.exists());
    assert!(!old_path.exists());
}
```

### Task 0.4: Update All Commands

需要更新的命令：
- `src/commands/deploy.rs`
- `src/commands/clean.rs`
- `src/application/diff.rs`
- `src/application/watch/use_case.rs`

主要改动：使用新的 `get_lockfile_path()` 方法。

### Task 0.5: Integration Tests

**File**: `tests/cli_lockfile_migration.rs`

```rust
#[test]
fn deploy_migrates_old_lockfile() {
    let project = setup_project();
    
    // 创建旧位置 lockfile
    let old_lockfile = project.path().join(".promptpack/.calvin.lock");
    fs::write(&old_lockfile, "version = 1\n").unwrap();
    
    // 运行 deploy
    let output = Command::new(calvin_binary())
        .args(["deploy"])
        .current_dir(project.path())
        .output()
        .unwrap();
    
    assert!(output.status.success());
    
    // 验证迁移
    let new_lockfile = project.path().join("calvin.lock");
    assert!(new_lockfile.exists());
    assert!(!old_lockfile.exists());
}
```

## Verification

1. 运行 `cargo test lockfile`
2. 运行 `cargo test --test cli_deploy`
3. 手动测试：
   - 在有旧 lockfile 的项目运行 `calvin deploy`
   - 验证 lockfile 迁移成功
   - 验证输出包含迁移提示

## Rollback Plan

如果需要回滚：
1. 用户可以手动移动 lockfile：`mv calvin.lock .promptpack/.calvin.lock`
2. 降级到旧版本 Calvin

## Dependencies

无（这是第一个阶段）

## Outputs

- 扩展的 `LockfileEntry` 结构
- 更新的 `TomlLockfileRepository`
- 自动迁移逻辑
- 新的集成测试

