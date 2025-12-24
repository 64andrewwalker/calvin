# Phase 1: Core Layer System

> **Priority**: High (MVP)  
> **Estimated Effort**: 3-4 days  
> **Dependencies**: Phase 0 complete

## Objective

实现多层 promptpack 加载和合并的核心逻辑，使 `calvin deploy` 能够从多个来源合并 assets。

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

### Task 1.4: Update FsAssetRepository

**File**: `src/infrastructure/repositories/asset.rs`

```rust
impl FsAssetRepository {
    /// 从多个层加载 assets
    pub fn load_from_layers(&self, layers: &[Layer]) -> Result<Vec<LayerWithAssets>, Error> {
        let mut result = Vec::new();
        
        for layer in layers {
            let assets = self.load_assets(&layer.path)?;
            result.push(LayerWithAssets {
                layer: layer.clone(),
                assets,
            });
        }
        
        Ok(result)
    }
}
```

### Task 1.5: Update Deploy Command

**File**: `src/application/deploy/use_case.rs`

主要改动：
1. 使用 `LayerResolver` 解析层
2. 使用 `merge_layers` 合并 assets
3. verbose 模式显示层信息

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

