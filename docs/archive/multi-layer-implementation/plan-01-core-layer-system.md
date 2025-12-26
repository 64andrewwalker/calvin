# Phase 1: Core Layer System

> **Priority**: High (MVP)  
> **Estimated Effort**: 3-4 days  
> **Dependencies**: Phase 0 complete
> **Architecture Reference**: `docs/architecture/layers.md`

## Objective

实现多层 promptpack 加载和合并的核心逻辑，使 `calvin deploy` 能够从多个来源合并 assets。

## Architecture Compliance

按四层架构分布新组件，禁止上帝对象：

```
Layer 2 (Domain):
├── domain/entities/layer.rs       # Layer, LayerType (~50 lines)
├── domain/services/layer_resolver.rs  # LayerResolver (~150 lines)
├── domain/services/layer_merger.rs    # merge_layers (~120 lines)
└── domain/ports/layer_loader.rs      # LayerLoader trait (~20 lines)

Layer 3 (Infrastructure):
└── infrastructure/layer/fs_loader.rs  # FsLayerLoader (~80 lines)

Layer 1 (Application):
└── application/deploy/use_case.rs   # 修改现有，使用新组件
```

**关键约束**:
- `Layer` entity 不依赖任何外部模块
- `LayerResolver` 只依赖 entities 和 ports
- `FsLayerLoader` 实现 `LayerLoader` port
- 每个文件 < 400 行

## Key Concepts

### Layer

```rust
pub struct Layer {
    /// 层名称 (user, project, custom)
    pub name: String,
    /// 层路径
    pub path: PathBuf,
    /// 层类型
    pub layer_type: LayerType,
    /// 该层的 assets
    pub assets: Vec<Asset>,
}

pub enum LayerType {
    User,     // ~/.calvin/.promptpack
    Custom,   // 配置的额外路径
    Project,  // ./.promptpack
}
```

### LayerStack

```rust
pub struct LayerStack {
    /// 从低优先级到高优先级排序
    layers: Vec<Layer>,
}

impl LayerStack {
    /// 合并所有层的 assets
    pub fn merge(&self) -> MergedAssets { ... }
}
```

## Detailed Tasks

### Task 1.1: Define Layer Types

**File**: `src/domain/entities/layer.rs`

```rust
use std::path::PathBuf;
use crate::models::PromptAsset;

#[derive(Debug, Clone, PartialEq)]
pub enum LayerType {
    User,
    Custom,
    Project,
}

#[derive(Debug, Clone)]
pub struct Layer {
    pub name: String,
    pub path: PathBuf,
    pub layer_type: LayerType,
    pub assets: Vec<PromptAsset>,
}

impl Layer {
    pub fn new(name: impl Into<String>, path: PathBuf, layer_type: LayerType) -> Self {
        Self {
            name: name.into(),
            path,
            layer_type,
            assets: Vec::new(),
        }
    }
    
    pub fn with_assets(mut self, assets: Vec<PromptAsset>) -> Self {
        self.assets = assets;
        self
    }
}
```

**Tests**:
```rust
#[test]
fn layer_creation() {
    let layer = Layer::new("user", PathBuf::from("~/.calvin/.promptpack"), LayerType::User);
    assert_eq!(layer.name, "user");
    assert_eq!(layer.layer_type, LayerType::User);
}
```

### Task 1.2: Implement Layer Resolution

**File**: `src/domain/services/layer_resolver.rs`

```rust
pub struct LayerResolver {
    user_layer_path: Option<PathBuf>,
    additional_layers: Vec<PathBuf>,
    project_root: PathBuf,
}

impl LayerResolver {
    pub fn resolve(&self) -> Result<Vec<Layer>, LayerError> {
        let mut layers = Vec::new();
        
        // 1. User layer (lowest priority)
        if let Some(user_path) = &self.user_layer_path {
            let expanded = expand_path(user_path);
            if expanded.exists() {
                layers.push(Layer::new("user", expanded, LayerType::User));
            }
        }
        
        // 2. Additional layers (in order)
        for (i, path) in self.additional_layers.iter().enumerate() {
            let expanded = expand_path(path);
            if expanded.exists() {
                layers.push(Layer::new(format!("custom-{}", i), expanded, LayerType::Custom));
            } else {
                // 警告但继续
                warn!("Additional layer not found: {}", path.display());
            }
        }
        
        // 3. Project layer (highest priority)
        let project_layer = self.project_root.join(".promptpack");
        if project_layer.exists() {
            layers.push(Layer::new("project", project_layer, LayerType::Project));
        }
        
        // 至少需要一个 layer
        if layers.is_empty() {
            return Err(LayerError::NoLayersFound);
        }
        
        Ok(layers)
    }
}
```

**Tests**:
```rust
#[test]
fn resolve_user_layer_only() {
    let dir = tempdir().unwrap();
    let user_layer = dir.path().join(".calvin/.promptpack");
    fs::create_dir_all(&user_layer).unwrap();
    
    let resolver = LayerResolver {
        user_layer_path: Some(user_layer.clone()),
        additional_layers: vec![],
        project_root: dir.path().to_path_buf(),
    };
    
    let layers = resolver.resolve().unwrap();
    assert_eq!(layers.len(), 1);
    assert_eq!(layers[0].layer_type, LayerType::User);
}

#[test]
fn resolve_all_layers() {
    // 创建三层
    let dir = tempdir().unwrap();
    let user_layer = dir.path().join("user/.promptpack");
    let custom_layer = dir.path().join("custom/.promptpack");
    let project_layer = dir.path().join("project/.promptpack");
    
    for layer in [&user_layer, &custom_layer, &project_layer] {
        fs::create_dir_all(layer).unwrap();
    }
    
    let resolver = LayerResolver {
        user_layer_path: Some(user_layer),
        additional_layers: vec![custom_layer],
        project_root: dir.path().join("project"),
    };
    
    let layers = resolver.resolve().unwrap();
    assert_eq!(layers.len(), 3);
    assert_eq!(layers[0].layer_type, LayerType::User);
    assert_eq!(layers[1].layer_type, LayerType::Custom);
    assert_eq!(layers[2].layer_type, LayerType::Project);
}
```

### Task 1.3: Implement Asset Merger

**File**: `src/domain/services/layer_merger.rs`

```rust
use std::collections::HashMap;

pub struct MergedAsset {
    pub asset: PromptAsset,
    pub source_layer: String,
    pub source_path: PathBuf,
    pub overrides: Option<String>,
}

pub struct MergeResult {
    pub assets: HashMap<String, MergedAsset>,
    pub overrides: Vec<OverrideInfo>,
}

pub struct OverrideInfo {
    pub asset_id: String,
    pub from_layer: String,
    pub by_layer: String,
}

pub fn merge_layers(layers: &[Layer]) -> MergeResult {
    let mut merged: HashMap<String, MergedAsset> = HashMap::new();
    let mut overrides = Vec::new();
    
    // 从低优先级到高优先级处理
    for layer in layers {
        for asset in &layer.assets {
            let id = asset.id().to_string();
            
            if let Some(existing) = merged.get(&id) {
                // 记录覆盖
                overrides.push(OverrideInfo {
                    asset_id: id.clone(),
                    from_layer: existing.source_layer.clone(),
                    by_layer: layer.name.clone(),
                });
            }
            
            merged.insert(id.clone(), MergedAsset {
                asset: asset.clone(),
                source_layer: layer.name.clone(),
                source_path: layer.path.clone(),
                overrides: merged.get(&id).map(|e| e.source_layer.clone()),
            });
        }
    }
    
    MergeResult { assets: merged, overrides }
}
```

**Tests**:
```rust
#[test]
fn merge_same_id_higher_wins() {
    let user_layer = Layer::new("user", PathBuf::from("user"), LayerType::User)
        .with_assets(vec![create_asset("style", "user content")]);
    
    let project_layer = Layer::new("project", PathBuf::from("project"), LayerType::Project)
        .with_assets(vec![create_asset("style", "project content")]);
    
    let result = merge_layers(&[user_layer, project_layer]);
    
    assert_eq!(result.assets.len(), 1);
    let style = result.assets.get("style").unwrap();
    assert_eq!(style.source_layer, "project");
    assert_eq!(style.overrides, Some("user".to_string()));
}

#[test]
fn merge_different_ids_all_kept() {
    let user_layer = Layer::new("user", PathBuf::from("user"), LayerType::User)
        .with_assets(vec![create_asset("security", "security rules")]);
    
    let project_layer = Layer::new("project", PathBuf::from("project"), LayerType::Project)
        .with_assets(vec![create_asset("style", "style rules")]);
    
    let result = merge_layers(&[user_layer, project_layer]);
    
    assert_eq!(result.assets.len(), 2);
    assert!(result.assets.contains_key("security"));
    assert!(result.assets.contains_key("style"));
}
```

### Task 1.4: Define LayerLoader Port and Implementation

**Port Definition (Layer 2 Domain)**:

**File**: `src/domain/ports/layer_loader.rs`

```rust
use crate::domain::entities::Layer;
use std::path::Path;

/// Port: 从路径加载层的 assets
pub trait LayerLoader: Send + Sync {
    /// 加载指定层的所有 assets
    fn load_layer_assets(&self, layer: &mut Layer) -> Result<(), LayerError>;
}

#[derive(Debug, thiserror::Error)]
pub enum LayerError {
    #[error("Layer path not found: {path}")]
    PathNotFound { path: std::path::PathBuf },
    
    #[error("Failed to load assets from layer: {message}")]
    LoadFailed { message: String },
}
```

**Infrastructure Implementation (Layer 3)**:

**File**: `src/infrastructure/layer/fs_loader.rs`

```rust
use crate::domain::entities::Layer;
use crate::domain::ports::layer_loader::{LayerLoader, LayerError};
use crate::infrastructure::repositories::FsAssetRepository;

/// 基于文件系统的层加载器
pub struct FsLayerLoader {
    asset_repo: FsAssetRepository,
}

impl FsLayerLoader {
    pub fn new(asset_repo: FsAssetRepository) -> Self {
        Self { asset_repo }
    }
}

impl LayerLoader for FsLayerLoader {
    fn load_layer_assets(&self, layer: &mut Layer) -> Result<(), LayerError> {
        if !layer.path.exists() {
            return Err(LayerError::PathNotFound { 
                path: layer.path.clone() 
            });
        }
        
        let assets = self.asset_repo
            .load_assets(&layer.path)
            .map_err(|e| LayerError::LoadFailed { 
                message: e.to_string() 
            })?;
        
        layer.assets = assets;
        Ok(())
    }
}
```

**Tests**:
```rust
#[test]
fn fs_loader_loads_assets() {
    let dir = tempdir().unwrap();
    let layer_path = dir.path().join(".promptpack");
    fs::create_dir_all(&layer_path).unwrap();
    // 创建测试 asset 文件...
    
    let loader = FsLayerLoader::new(FsAssetRepository::new());
    let mut layer = Layer::new("test", layer_path, LayerType::Project);
    
    loader.load_layer_assets(&mut layer).unwrap();
    
    assert!(!layer.assets.is_empty());
}

#[test]
fn fs_loader_error_on_missing_path() {
    let loader = FsLayerLoader::new(FsAssetRepository::new());
    let mut layer = Layer::new("test", PathBuf::from("/nonexistent"), LayerType::Project);
    
    let result = loader.load_layer_assets(&mut layer);
    
    assert!(matches!(result, Err(LayerError::PathNotFound { .. })));
}
```

### Task 1.5: Update Deploy Command

**File**: `src/application/deploy/use_case.rs`

主要改动：
1. 使用 `LayerResolver` 解析层
2. 使用 `merge_layers` 合并 assets
3. verbose 模式显示层信息

### Task 1.6: Handle Asset Layer Migration (PRD §5.5)

**场景**：Asset 在不同层之间迁移时的处理

```
1. 第一次 deploy：asset "review" 来自 project 层
2. 用户删除 project 层的 review.md，但 user 层有同名 asset
3. 第二次 deploy：asset "review" 现在来自 user 层
```

**行为**：
- 输出路径相同 → 更新 lockfile 的 source_layer，不产生 orphan
- 输出路径不同 → 旧路径成为 orphan，新路径是新文件

**File**: `src/domain/services/layer_merger.rs`

```rust
fn detect_layer_migrations(
    old_lockfile: &Lockfile,
    new_outputs: &[OutputFile],
) -> Vec<LayerMigration> {
    let mut migrations = Vec::new();
    
    for output in new_outputs {
        if let Some(old_entry) = old_lockfile.get(&output.key) {
            if old_entry.source_layer != Some(output.source_layer.clone()) {
                migrations.push(LayerMigration {
                    output_key: output.key.clone(),
                    from_layer: old_entry.source_layer.clone(),
                    to_layer: output.source_layer.clone(),
                });
            }
        }
    }
    
    migrations
}
```

**Tests**:
```rust
#[test]
fn detect_migration_project_to_user() {
    let old_lockfile = create_lockfile_with_entry(
        "project:.claude/commands/review.md",
        "sha256:abc",
        Some("project"),
    );
    
    let new_outputs = vec![OutputFile {
        key: "project:.claude/commands/review.md".to_string(),
        source_layer: "user".to_string(),
        // ...
    }];
    
    let migrations = detect_layer_migrations(&old_lockfile, &new_outputs);
    
    assert_eq!(migrations.len(), 1);
    assert_eq!(migrations[0].from_layer, Some("project".to_string()));
    assert_eq!(migrations[0].to_layer, "user".to_string());
}
```

### Task 1.7: Symlink Handling (PRD §5.6)

**策略**：跟随符号链接，但记录原始路径用于显示

**File**: `src/domain/services/layer_resolver.rs`

```rust
fn resolve_layer_path(path: &Path) -> Result<ResolvedPath, LayerError> {
    // 检测循环符号链接
    let mut seen = HashSet::new();
    let mut current = path.to_path_buf();
    
    while current.is_symlink() {
        if !seen.insert(current.clone()) {
            return Err(LayerError::CircularSymlink { 
                path: path.to_path_buf(),
                chain: format_symlink_chain(&seen),
            });
        }
        current = current.read_link().map_err(|_| LayerError::SymlinkReadError {
            path: current.clone(),
        })?;
    }
    
    let canonical = path.canonicalize().map_err(|_| LayerError::PathNotFound {
        path: path.to_path_buf(),
    })?;
    
    Ok(ResolvedPath {
        original: path.to_path_buf(),
        resolved: canonical,
    })
}
```

**Tests**:
```rust
#[test]
fn resolve_symlink_layer() {
    let dir = tempdir().unwrap();
    let real_layer = dir.path().join("real/.promptpack");
    let symlink_layer = dir.path().join("link/.promptpack");
    
    fs::create_dir_all(&real_layer).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&real_layer, &symlink_layer).unwrap();
    
    let resolved = resolve_layer_path(&symlink_layer).unwrap();
    assert_eq!(resolved.original, symlink_layer);
    assert_eq!(resolved.resolved, real_layer.canonicalize().unwrap());
}

#[test]
fn detect_circular_symlink() {
    // Create A -> B -> A
    let dir = tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&b, &a).unwrap();
        std::os::unix::fs::symlink(&a, &b).unwrap();
    }
    
    let result = resolve_layer_path(&a);
    assert!(matches!(result, Err(LayerError::CircularSymlink { .. })));
}
```

### Note: Remote Deploy Behavior (PRD §14.3)

当使用 `--remote` 时，多层系统简化为：
- **只使用项目层** (`.promptpack/`)
- **忽略用户层和额外层**

**原因**：
- Remote deploy 的目的是"把这个项目部署到远程"
- 远程机器的用户层可能不存在或不同
- 保持行为简单可预测

此行为在 `LayerResolver` 中通过参数控制：
```rust
let resolver = LayerResolver::new()
    .with_remote_mode(is_remote);  // 只使用项目层
```

### Note: Watch Mode Behavior (PRD §14.4)

**默认行为**：只监听项目层

**原因**：
- 用户层变化是罕见的（全局配置）
- 监听多个目录有性能开销
- 用户可以手动重新运行 `calvin deploy`

**未来选项**：`--watch-all-layers` 标志（Phase 5）

## Verification

1. 运行 `cargo test layer`
2. 运行 `cargo test merge`
3. 手动测试：
   - 创建 `~/.calvin/.promptpack/actions/test.md`
   - 在项目中运行 `calvin deploy -v`
   - 验证显示层栈

## Outputs

- `Layer` 和 `LayerType` 类型
- `LayerResolver` 服务
- `merge_layers` 函数
- 更新的 `FsAssetRepository`
- 更新的 `deploy` 命令

