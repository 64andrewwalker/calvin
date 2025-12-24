# Multi-Layer PromptPack Migration Guide

> **Status**: Draft  
> **Author**: [Maintainer]  
> **Created**: 2024-12-24  
> **Related PRD**: [multi-layer-promptpack-prd.md](./multi-layer-promptpack-prd.md)

## Overview

本文档详细描述从当前单层 `.promptpack/` 实现迁移到多层架构时需要考虑的技术细节、潜在问题和解决方案。

---

## 1. Current Implementation Analysis

### 1.1 Lockfile 当前实现

**路径**: `.promptpack/.calvin.lock` (由 `source.join(".calvin.lock")` 生成)

**当前格式**:
```toml
version = 1

[files."project:.claude/commands/review.md"]
hash = "sha256:abc123..."

[files."home:~/.claude/commands/global.md"]
hash = "sha256:def456..."
```

**关键代码位置**:
- `src/domain/entities/lockfile.rs` - Lockfile 实体定义
- `src/infrastructure/repositories/lockfile.rs` - TOML 序列化/反序列化
- `src/application/deploy/use_case.rs:749-751` - `get_lockfile_path()` 方法

```rust
// 当前实现
fn get_lockfile_path(&self, source: &Path, _scope: Scope) -> PathBuf {
    source.join(".calvin.lock")
}
```

### 1.2 配置加载当前实现

**配置层级**:
1. Built-in defaults
2. User config: `~/.config/calvin/config.toml`
3. Project config: `.promptpack/config.toml`
4. Environment variables (`CALVIN_*`)
5. CLI arguments

**关键代码位置**:
- `src/config/loader.rs` - 配置加载逻辑
- `src/config/types.rs` - 配置类型定义

**注意**: 当前 `Config` 结构没有 `[sources]` section。

### 1.3 Asset 加载当前实现

**关键代码位置**:
- `src/infrastructure/repositories/asset.rs` - `FsAssetRepository`
- `src/domain/ports/asset_repository.rs` - `AssetRepository` trait

当前只从单一 `.promptpack/` 目录加载 assets。

---

## 2. Breaking Changes

### 2.1 Lockfile 位置迁移

| 项目 | 旧路径 | 新路径 |
|------|--------|--------|
| Lockfile | `.promptpack/.calvin.lock` | `./calvin.lock` |

**影响**:
- 所有现有项目的 lockfile 需要迁移
- CI/CD 脚本可能硬编码了旧路径
- `.gitignore` 规则可能需要更新

### 2.2 Lockfile 格式扩展

**新增字段** (可选，向后兼容):
```toml
[files."project:.claude/commands/review.md"]
hash = "sha256:abc123..."
source_layer = "user"           # 新增
source_asset = "review"         # 新增
source_path = "~/.calvin/.promptpack/actions/review.md"  # 新增
```

### 2.3 配置扩展

**新增 `[sources]` section**:
```toml
[sources]
use_user_layer = true
user_layer_path = "~/.calvin/.promptpack"
additional_layers = []
disable_project_layer = false
```

---

## 3. Migration Scenarios

### 3.1 场景：现有项目首次使用新版本

**用户操作**:
```bash
cd /my-project
calvin deploy  # 使用新版 Calvin
```

**预期行为**:
1. 检测到 `.promptpack/.calvin.lock` 存在
2. 自动迁移到 `./calvin.lock`
3. 删除旧文件
4. 输出迁移提示

**实现代码**:
```rust
fn load_lockfile(project_root: &Path) -> Lockfile {
    let new_path = project_root.join("calvin.lock");
    let old_path = project_root.join(".promptpack/.calvin.lock");
    
    if new_path.exists() {
        return load_from(&new_path);
    }
    
    if old_path.exists() {
        eprintln!("ℹ Migrating lockfile to new location: {}", new_path.display());
        let lockfile = load_from(&old_path);
        
        // 迁移格式（添加 source_layer 默认值）
        let migrated = migrate_lockfile_format(lockfile);
        
        save_to(&migrated, &new_path)?;
        std::fs::remove_file(&old_path)?;
        
        eprintln!("✓ Lockfile migrated successfully");
        return migrated;
    }
    
    Lockfile::new()
}
```

### 3.2 场景：旧版 Calvin 打开新格式 lockfile

**问题**: 用户降级 Calvin 版本，遇到新格式 lockfile

**预期行为**:
- 旧版本应该能读取新格式（忽略未知字段）
- TOML 解析默认忽略未知字段 ✓

**验证代码**:
```rust
#[test]
fn old_version_reads_new_format() {
    // 新格式 lockfile 包含 source_layer 等新字段
    let new_format = r#"
        version = 1
        
        [files."project:test.md"]
        hash = "sha256:abc"
        source_layer = "user"
        source_asset = "review"
        source_path = "~/.calvin/.promptpack/actions/review.md"
    "#;
    
    // 旧版本的 TomlFileEntry 只有 hash 字段
    #[derive(Deserialize)]
    struct OldTomlFileEntry {
        hash: String,
    }
    
    // 解析应该成功，忽略未知字段
    let parsed: toml::Value = toml::from_str(new_format).unwrap();
    // ...
}
```

### 3.3 场景：新版 Calvin 打开旧格式 lockfile (已迁移位置)

**问题**: 用户手动将 lockfile 移到新位置，但格式是旧的

**预期行为**:
- 解析成功，缺少的字段使用默认值
- `source_layer = "unknown"`
- `source_asset = "unknown"`
- `source_path = "unknown"`

**实现代码**:
```rust
fn load_lockfile_entry(entry: &TomlEntry) -> LockfileEntry {
    LockfileEntry {
        hash: entry.hash.clone(),
        source_layer: entry.source_layer.clone().unwrap_or_else(|| "unknown".to_string()),
        source_asset: entry.source_asset.clone().unwrap_or_else(|| "unknown".to_string()),
        source_path: entry.source_path.clone().unwrap_or_else(|| "unknown".to_string()),
    }
}
```

### 3.4 场景：项目层不存在，只有用户层

**用户操作**:
```bash
cd /new-project  # 没有 .promptpack/
# 但用户层存在（默认 ~/.calvin/.promptpack，或用户配置的其他路径）

calvin deploy
```

**当前行为**: 错误 "No .promptpack directory found"

**新行为**: 
- 检测用户层（路径由 `~/.config/calvin/config.toml` 中的 `sources.user_layer_path` 指定）
- 使用用户层的 assets
- 成功部署
- 输出提示："No project .promptpack found. Using user layer: ~/.calvin/.promptpack"

**关键点**：
- 用户层路径**不一定在 `~/.calvin/`**
- 用户可以通过 `sources.user_layer_path` 指定任意路径
- 常见用法：指向 dotfiles repo 中的 `.promptpack`

```toml
# ~/.config/calvin/config.toml
# 自定义示例：指向 dotfiles repo
[sources]
user_layer_path = "~/dotfiles/.promptpack"  # 可选：跟随 dotfiles repo 同步
```

**实现改动**:
```rust
// 当前
fn load_assets(&self, source: &Path) -> Result<Vec<Asset>, Error> {
    if !source.exists() {
        return Err(Error::SourceNotFound(source.to_path_buf()));
    }
    // ...
}

// 新实现
fn load_layers(&self, config: &Config, project_root: &Path) -> Result<Vec<Layer>, Error> {
    let mut layers = Vec::new();
    
    // 1. User layer
    if config.sources.use_user_layer {
        if let Ok(layer) = self.load_layer(&config.sources.user_layer_path) {
            layers.push(layer);
        }
    }
    
    // 2. Additional layers
    for path in &config.sources.additional_layers {
        if let Ok(layer) = self.load_layer(path) {
            layers.push(layer);
        } else {
            // 警告但继续
            eprintln!("⚠ Warning: Additional layer not found: {}", path.display());
        }
    }
    
    // 3. Project layer
    let project_promptpack = project_root.join(".promptpack");
    if project_promptpack.exists() {
        if let Ok(layer) = self.load_layer(&project_promptpack) {
            layers.push(layer);
        }
    }
    
    // 至少需要一个 layer
    if layers.is_empty() {
        return Err(Error::NoLayersFound);
    }
    
    Ok(layers)
}
```

---

## 4. Potential Issues and Solutions

### 4.1 Issue: CI/CD 脚本硬编码 lockfile 路径

**问题**:
```yaml
# .github/workflows/calvin.yml
- name: Check lockfile
  run: cat .promptpack/.calvin.lock
```

**解决方案**:
1. 在 CHANGELOG 和发布说明中明确标注 breaking change
2. 提供迁移脚本或 `calvin migrate` 命令
3. 在一个版本中同时支持两个位置（读取），但只写入新位置

### 4.2 Issue: .gitignore 规则

**问题**: 用户可能有 `.gitignore` 规则忽略旧路径

**现状**:
```gitignore
# 用户可能没有显式规则，因为 lockfile 在 .promptpack/ 里
# 而 .promptpack/ 通常被提交
```

**新情况**:
```gitignore
# 用户可能需要确保 calvin.lock 被提交
# !calvin.lock  # 如果有通配符规则可能需要这个
```

**解决方案**:
- `calvin migrate` 检查并建议 .gitignore 更新
- 在文档中明确说明 `calvin.lock` 应该被提交

### 4.3 Issue: 符号链接的 .promptpack 目录

**问题**:
```bash
ln -s /shared/team-prompts ~/.calvin/.promptpack
```

**潜在问题**:
- 循环链接
- 权限问题
- 路径解析不一致

**解决方案**:
```rust
fn resolve_layer_path(path: &Path) -> Result<PathBuf, Error> {
    // 检测循环链接
    let mut seen = HashSet::new();
    let mut current = path.to_path_buf();
    
    while current.is_symlink() {
        if !seen.insert(current.clone()) {
            return Err(Error::CircularSymlink(path.to_path_buf()));
        }
        current = current.read_link()?;
    }
    
    // 验证最终路径存在且可读
    if !current.exists() {
        return Err(Error::LayerNotFound(path.to_path_buf()));
    }
    
    current.canonicalize()
        .map_err(|e| Error::PathResolution(path.to_path_buf(), e))
}
```

### 4.4 Issue: Windows 路径兼容性

**问题**: `~` 展开和路径分隔符

**解决方案**:
```rust
fn expand_path(path: &str) -> PathBuf {
    if path.starts_with("~") {
        let home = if cfg!(windows) {
            std::env::var("USERPROFILE")
        } else {
            std::env::var("HOME")
        };
        
        match home {
            Ok(home) => {
                let rest = path.strip_prefix("~").unwrap_or(path);
                let rest = rest.strip_prefix('/').or_else(|| rest.strip_prefix('\\')).unwrap_or(rest);
                PathBuf::from(home).join(rest)
            }
            Err(_) => PathBuf::from(path),
        }
    } else {
        PathBuf::from(path)
    }
}
```

### 4.5 Issue: Remote Deploy 行为变化

**问题**: 用户期望 remote deploy 也使用全局层

**当前 PRD 决定**: Remote deploy 只使用项目层

**实现**:
```rust
fn execute_remote(&self, options: &DeployOptions) -> Result<DeployResult> {
    // 强制禁用所有非项目层
    let remote_options = DeployOptions {
        use_user_layer: false,
        use_additional_layers: false,
        ..options.clone()
    };
    
    // ...
}
```

**文档说明**:
> Remote deploy 只部署项目层的 assets。这是因为远程机器上可能没有相同的用户层配置。如果需要，请先在本地项目中创建覆盖。

### 4.6 Issue: Watch 模式的全局层变化

**问题**: 全局层变化时，是否应该触发所有项目的重新编译？

**决定**: 默认不监听全局层

**实现**:
```rust
fn watch(&self, options: &WatchOptions) -> Result<()> {
    let paths_to_watch = if options.watch_all_layers {
        // 获取所有层路径
        self.get_all_layer_paths(options)
    } else {
        // 只监听项目层
        vec![options.source.clone()]
    };
    
    // ...
}
```

---

## 5. Code Changes Required

### 5.1 LockfileEntry 扩展

```rust
// src/domain/entities/lockfile.rs

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockfileEntry {
    hash: String,
    source_layer: Option<String>,       // 新增：层名称 (user/project/custom)
    source_layer_path: Option<PathBuf>, // 新增：层实际路径
    source_asset: Option<String>,       // 新增：asset id
    source_file: Option<PathBuf>,       // 新增：来源文件路径
    overrides: Option<String>,          // 新增：覆盖了哪个层
}

impl LockfileEntry {
    pub fn new(hash: impl Into<String>) -> Self {
        Self {
            hash: hash.into(),
            source_layer: None,
            source_layer_path: None,
            source_asset: None,
            source_file: None,
            overrides: None,
        }
    }
    
    pub fn with_provenance(
        hash: impl Into<String>,
        provenance: OutputProvenance,
    ) -> Self {
        Self {
            hash: hash.into(),
            source_layer: Some(provenance.source_layer),
            source_layer_path: Some(provenance.source_layer_path),
            source_asset: Some(provenance.source_asset),
            source_file: Some(provenance.source_file),
            overrides: provenance.overrides,
        }
    }
    
    // ... getters
}

/// 输出文件的来源信息
#[derive(Debug, Clone)]
pub struct OutputProvenance {
    pub source_layer: String,
    pub source_layer_path: PathBuf,
    pub source_asset: String,
    pub source_file: PathBuf,
    pub overrides: Option<String>,
}
```

### 5.2 Config 扩展

```rust
// src/config/types.rs

/// Sources configuration for multi-layer support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcesConfig {
    #[serde(default = "default_true")]
    pub use_user_layer: bool,
    
    #[serde(default = "default_user_layer_path")]
    pub user_layer_path: PathBuf,
    
    #[serde(default)]
    pub additional_layers: Vec<PathBuf>,
    
    #[serde(default)]
    pub disable_project_layer: bool,
}

fn default_user_layer_path() -> PathBuf {
    // ~/.calvin/.promptpack
    dirs::home_dir()
        .map(|h| h.join(".calvin/.promptpack"))
        .unwrap_or_else(|| PathBuf::from("~/.calvin/.promptpack"))
}

impl Default for SourcesConfig {
    fn default() -> Self {
        Self {
            use_user_layer: true,
            user_layer_path: default_user_layer_path(),
            additional_layers: Vec::new(),
            disable_project_layer: false,
        }
    }
}

// 扩展 Config
pub struct Config {
    // ... existing fields
    
    #[serde(default)]
    pub sources: SourcesConfig,  // 新增
}
```

### 5.3 Registry 实现

```rust
// src/infrastructure/repositories/registry.rs (新文件)

use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEntry {
    pub path: PathBuf,
    pub lockfile: PathBuf,
    pub last_deployed: DateTime<Utc>,
    pub asset_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Registry {
    pub version: u32,
    pub projects: Vec<ProjectEntry>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            version: 1,
            projects: Vec::new(),
        }
    }
    
    pub fn upsert(&mut self, entry: ProjectEntry) {
        if let Some(existing) = self.projects.iter_mut().find(|p| p.path == entry.path) {
            *existing = entry;
        } else {
            self.projects.push(entry);
        }
    }
    
    pub fn remove(&mut self, path: &Path) -> bool {
        let len_before = self.projects.len();
        self.projects.retain(|p| p.path != path);
        self.projects.len() != len_before
    }
    
    pub fn prune(&mut self) -> Vec<PathBuf> {
        let (valid, invalid): (Vec<_>, Vec<_>) = self.projects
            .drain(..)
            .partition(|p| p.lockfile.exists());
        
        let removed: Vec<_> = invalid.into_iter().map(|p| p.path).collect();
        self.projects = valid;
        removed
    }
}
```

---

## 6. Testing Requirements

### 6.1 迁移测试

```rust
#[test]
fn migrate_lockfile_from_old_location() {
    let dir = tempdir().unwrap();
    let old_path = dir.path().join(".promptpack/.calvin.lock");
    let new_path = dir.path().join("calvin.lock");
    
    // 创建旧格式 lockfile
    fs::create_dir_all(old_path.parent().unwrap()).unwrap();
    fs::write(&old_path, "version = 1\n[files.\"project:test.md\"]\nhash = \"abc\"").unwrap();
    
    // 加载应该触发迁移
    let lockfile = load_lockfile(dir.path());
    
    // 验证
    assert!(new_path.exists(), "New lockfile should exist");
    assert!(!old_path.exists(), "Old lockfile should be removed");
    assert_eq!(lockfile.get_hash("project:test.md"), Some("abc"));
}

#[test]
fn backward_compatible_with_old_format() {
    // 新版本 Calvin 应该能读取没有 source_* 字段的 lockfile
    let content = r#"
        version = 1
        [files."project:test.md"]
        hash = "abc"
    "#;
    
    let lockfile = parse_lockfile(content);
    let entry = lockfile.get("project:test.md").unwrap();
    
    assert_eq!(entry.hash(), "abc");
    assert_eq!(entry.source_layer(), Some("unknown"));
}
```

### 6.2 多层合并测试

```rust
#[test]
fn layer_merge_higher_priority_wins() {
    let user_layer = create_layer("user", vec![
        asset("style", "user content"),
        asset("security", "user security"),
    ]);
    
    let project_layer = create_layer("project", vec![
        asset("style", "project content"),  // 覆盖 user
    ]);
    
    let merged = merge_layers(vec![user_layer, project_layer]);
    
    assert_eq!(merged.get("style").content(), "project content");
    assert_eq!(merged.get("security").content(), "user security");
}
```

---

## 7. Migration Command

建议实现 `calvin migrate` 命令：

```bash
# 迁移 lockfile 位置
calvin migrate --lockfile

# 迁移并创建用户层目录
calvin migrate --init-user-layer

# 预览迁移（不执行）
calvin migrate --dry-run

# 完整迁移
calvin migrate
```

**实现**:
```rust
pub fn execute_migrate(options: &MigrateOptions) -> Result<()> {
    let mut changes = Vec::new();
    
    // 1. 检查 lockfile 迁移
    let old_lockfile = options.project_root.join(".promptpack/.calvin.lock");
    let new_lockfile = options.project_root.join("calvin.lock");
    
    if old_lockfile.exists() && !new_lockfile.exists() {
        changes.push(MigrationChange::MoveLockfile {
            from: old_lockfile.clone(),
            to: new_lockfile.clone(),
        });
    }
    
    // 2. 检查用户层初始化
    let user_layer = expand_path("~/.calvin/.promptpack");
    if options.init_user_layer && !user_layer.exists() {
        changes.push(MigrationChange::CreateUserLayer {
            path: user_layer,
        });
    }
    
    // 3. 执行或预览
    if options.dry_run {
        println!("Would perform the following changes:");
        for change in &changes {
            println!("  - {}", change);
        }
    } else {
        for change in changes {
            change.execute()?;
        }
        println!("Migration complete");
    }
    
    Ok(())
}
```

---

## 8. Rollback Plan

如果迁移失败，用户应该能够回滚：

1. **Lockfile 回滚**:
   ```bash
   mv calvin.lock .promptpack/.calvin.lock
   ```

2. **降级 Calvin 版本**:
   ```bash
   brew install calvin@0.3  # 假设的旧版本
   ```

3. **禁用多层功能**:
   ```toml
   # .promptpack/config.toml
   [sources]
   use_user_layer = false
   additional_layers = []
   ```

---

## 9. Documentation Updates Required

| 文档 | 更新内容 |
|------|----------|
| `docs/configuration.md` | 添加 `[sources]` section 说明 |
| `docs/command-reference.md` | 添加 `calvin migrate`, `calvin projects`, `calvin layers` |
| `docs-content/docs/api/frontmatter.mdx` | 更新 scope 说明 |
| `docs-content/docs/api/versioning.mdx` | 添加 lockfile 格式版本说明 |
| `CHANGELOG.md` | 添加 breaking changes 说明 |
| `README.md` | 更新快速开始示例 |

---

## 10. Release Checklist

- [ ] Lockfile 迁移逻辑实现并测试
- [ ] Config `[sources]` section 实现
- [ ] Registry 实现
- [ ] 多层合并逻辑实现
- [ ] `calvin migrate` 命令实现
- [ ] `calvin projects` 命令实现
- [ ] `calvin layers` 命令实现
- [ ] 所有现有测试仍然通过
- [ ] 新功能测试覆盖
- [ ] 文档更新
- [ ] CHANGELOG 更新，明确标注 breaking changes
- [ ] 发布说明准备

---

## Appendix: File Changes Summary

### 新文件

- `src/infrastructure/repositories/registry.rs`
- `src/commands/migrate.rs`
- `src/commands/projects.rs`
- `src/commands/layers.rs`
- `src/commands/provenance.rs` - 产物来源报告
- `src/domain/entities/layer.rs`
- `src/domain/services/layer_merger.rs`
- `src/domain/value_objects/provenance.rs` - OutputProvenance 定义

### 修改文件

- `src/domain/entities/lockfile.rs` - 扩展 LockfileEntry
- `src/infrastructure/repositories/lockfile.rs` - 新格式序列化
- `src/config/types.rs` - 添加 SourcesConfig
- `src/config/loader.rs` - 加载 sources 配置
- `src/infrastructure/repositories/asset.rs` - 多层加载
- `src/application/deploy/use_case.rs` - 多层支持
- `src/presentation/cli.rs` - 新命令和新参数

### CLI 参数扩展

```rust
// src/presentation/cli.rs

#[derive(Parser)]
pub struct DeployArgs {
    // 现有参数...
    
    /// Explicit path to .promptpack directory (overrides project layer detection)
    #[arg(long)]
    pub source: Option<PathBuf>,
    
    /// Additional promptpack layers (can be specified multiple times)
    #[arg(long = "layer", value_name = "PATH")]
    pub layers: Vec<PathBuf>,
    
    /// Disable user layer
    #[arg(long)]
    pub no_user_layer: bool,
    
    /// Disable additional configured layers
    #[arg(long)]
    pub no_additional_layers: bool,
}
```

