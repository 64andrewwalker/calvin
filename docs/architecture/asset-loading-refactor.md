# Asset Loading Architecture Refactor

> **Status**: ✅ Completed
> **Created**: 2025-12-28
> **Completed**: 2025-12-28
> **Author**: AI Assistant
> **Issue**: Deploy 和显示命令使用不同的加载路径，导致 `.calvinignore` 只在部分场景生效

---

## 0. 自我批判

### 0.1 方案 v3 遗漏的问题

1. **缺少端到端测试**
   - 搜索 `tests/` 目录发现：**没有任何 `.calvinignore` 相关的 CLI 测试**
   - 只有单元测试 (`fs_loader.rs`, `asset.rs`)
   - 这违反了 `TESTING_PHILOSOPHY.md` 的核心原则：*"We do not test implementations. We test contracts."*

2. **Mock 测试的问题**
   - 当前 `DeployUseCase` 测试使用 `MockAssetRepository`
   - 该 mock 的 `load_all()` 直接返回预设 assets，**完全跳过 .calvinignore 逻辑**
   - 这意味着如果我扩展 `AssetRepository` trait，所有 mock 都需要更新

3. **`LayerQueryUseCase` 直接依赖 `FsLayerLoader` 具体类型**
   ```rust
   // application/layers/query.rs
   pub struct LayerQueryUseCase {
       loader: FsLayerLoader,  // 具体类型，非 trait
   }
   ```
   - 这破坏了依赖倒置原则
   - 但如果泛型化，会增加复杂度

4. **`load_all_with_ignore()` 不在 trait 上**
   - `FsLayerLoader` 直接调用 `FsAssetRepository::load_all_with_ignore()`
   - 这是实现细节耦合，mock 无法使用

5. **两条路径的真正差异**

   | 特性 | AssetRepository.load_all() | LayerLoader.load_layer_assets() |
   |------|---------------------------|--------------------------------|
   | 输入 | `&Path` | `&mut Layer` |
   | 输出 | `Vec<Asset>` | 修改 `layer.assets` |
   | ignored_count | 丢弃 | 保存到 `layer.ignored_count` |
   | 验证 (path exists) | 无 | 有 |
   | 唯一 ID 检查 | 无 | 有 (`ensure_unique_asset_ids`) |

   **结论**：它们不是简单的重复，而是有不同的职责边界。

### 0.2 架构设计的深层问题

**问题**：`AssetRepository` 和 `LayerLoader` 是两个不同抽象层次的概念

```
抽象层次：

HIGH   LayerLoader         ← Layer 实体操作（验证 + 加载 + 填充）
                            ↓ uses
LOW    AssetRepository     ← 文件解析（纯数据转换）
```

**当前混淆**：
- `DeployUseCase` 直接使用低层 `AssetRepository`，但期望高层行为（带 ignore）
- 修复方法是在 `AssetRepository.load_all()` 中加入 ignore 逻辑
- 这实际上是**提升了 AssetRepository 的抽象层次**

**更好的设计**：
- `AssetRepository` 应该是纯粹的文件解析器，不关心 ignore
- `LayerLoader` 负责加载流程（包括 ignore），对外暴露
- `DeployUseCase` 应该使用 `LayerLoader` 而非 `AssetRepository`

---

## 1. 修正后的设计目标

1. **统一加载路径**：所有场景使用 `LayerLoader`
2. **职责清晰**：
   - `AssetRepository` = 文件解析（低层）
   - `LayerLoader` = Layer 加载流程（高层）
3. **TDD 驱动**：先写端到端测试，再实现
4. **不创建上帝对象**：保持组件精简

---

## 2. TDD 实施计划

按照 `TESTING_PHILOSOPHY.md`，我们应该：

### 2.1 第一步：写合约测试（Contract Tests）

**文件**: `tests/contracts/calvinignore.rs`

```rust
/// CONTRACT: .calvinignore patterns are respected during deploy
/// 
/// This prevents assets matching ignore patterns from being deployed.
/// The contract must hold for:
/// - Interactive mode
/// - CLI mode (non-interactive)
/// - All target platforms
#[test]
fn contract_calvinignore_excludes_matching_assets_from_deploy() {
    let env = TestEnv::builder()
        .with_project_asset("policies/draft.md", SIMPLE_POLICY)
        .with_project_asset("policies/final.md", SIMPLE_POLICY)
        .with_calvinignore("policies/draft.md\n")
        .build();
    
    let result = env.run(&["deploy", "--targets", "claude-code"]);
    
    assert!(result.success());
    assert!(!env.deployed_file_exists(".claude/policies/draft.mdc"));
    assert!(env.deployed_file_exists(".claude/policies/final.mdc"));
}

/// CONTRACT: .calvinignore patterns work for skill directories
#[test]
fn contract_calvinignore_excludes_skill_directories() {
    let env = TestEnv::builder()
        .with_skill("_template", SKILL_CONTENT)
        .with_skill("production", SKILL_CONTENT)
        .with_calvinignore("skills/_template/\n")
        .build();
    
    let result = env.run(&["deploy", "--targets", "claude-code"]);
    
    assert!(result.success());
    assert!(!env.deployed_dir_exists(".claude/skills/_template"));
    assert!(env.deployed_dir_exists(".claude/skills/production"));
}
```

### 2.2 第二步：写回归测试

**文件**: `tests/regression/calvinignore_deploy.rs`

```rust
/// REGRESSION: .calvinignore not applied during deploy
/// 
/// Bug: Running `calvin deploy` deployed assets that should have been 
/// ignored by .calvinignore patterns.
/// 
/// Root cause: DeployUseCase used AssetRepository.load_all() which 
/// did not apply .calvinignore filtering.
/// 
/// Fixed in: [this PR]
/// Related contract: contract_calvinignore_excludes_matching_assets_from_deploy
#[test]
fn regression_calvinignore_skills_template_deployed() {
    // Exact reproduction from user report:
    // - .promptpack/skills/_template/ exists
    // - .promptpack/.calvinignore contains "skills/_template/"
    // - After deploy, .claude/skills/_template should NOT exist
}
```

### 2.3 第三步：实现重构

**只有在测试写完之后**，才进行代码重构。

---

## 3. 重构方案（修正版）

### 3.1 方案概述

**核心变更**：让 `DeployUseCase` 和 `DiffUseCase` 使用 `LayerLoader` 而非直接调用 `AssetRepository`。

**但不改变泛型签名**，而是通过组合：

```rust
// DeployUseCase 保持现有签名
pub struct DeployUseCase<AR, LR, FS> { ... }

// 内部使用 LayerLoader trait object
impl<AR, LR, FS> DeployUseCase<AR, LR, FS> {
    fn load_assets_from_layers(&self, ...) -> Result<LayeredAssets, String> {
        // 不再: self.asset_repo.load_all(layer.path)
        // 而是: 使用公共函数 load_layer_assets()
    }
}
```

### 3.2 阶段一：扩展 AssetRepository trait（低风险）

将 `load_all_with_ignore` 提升到 trait 层面：

```rust
// domain/ports/asset_repository.rs
pub trait AssetRepository {
    fn load_all(&self, source: &Path) -> Result<Vec<Asset>>;
    
    fn load_all_with_ignore(
        &self,
        source: &Path,
        ignore: &IgnorePatterns,
    ) -> Result<(Vec<Asset>, usize)>;
    
    fn load_by_path(&self, path: &Path) -> Result<Asset>;
}
```

**影响**：
- 所有 mock 需要实现 `load_all_with_ignore`
- 现有代码不需要改动

### 3.3 阶段二：创建 layer_ops 模块（新增代码）

**文件**: `src/application/layer_ops.rs`

```rust
//! Layer Operations
//!
//! Shared utilities for loading assets from resolved layers.

use crate::domain::entities::Layer;
use crate::domain::ports::AssetRepository;
use crate::domain::ports::layer_loader::ensure_unique_asset_ids;
use crate::domain::value_objects::IgnorePatterns;

/// Load assets for a list of resolved layers.
///
/// This function:
/// 1. Reads .calvinignore from each layer
/// 2. Loads assets using AssetRepository with ignore filtering
/// 3. Ensures unique asset IDs within each layer
/// 4. Populates layer.assets and layer.ignored_count
pub fn load_resolved_layers<AR: AssetRepository>(
    asset_repo: &AR,
    layers: &mut [Layer],
) -> Result<(), String> {
    for layer in layers {
        let layer_root = layer.path.resolved();
        
        let ignore = IgnorePatterns::load(layer_root)
            .map_err(|e| format!(
                "Failed to load .calvinignore for '{}': {}", 
                layer.name, e
            ))?;
        
        let (assets, ignored_count) = asset_repo
            .load_all_with_ignore(layer_root, &ignore)
            .map_err(|e| format!(
                "Failed to load layer '{}': {}", 
                layer.name, e
            ))?;
        
        ensure_unique_asset_ids(&layer.name, &assets)
            .map_err(|e| e.to_string())?;
        
        layer.assets = assets;
        layer.ignored_count = ignored_count;
    }
    Ok(())
}
```

### 3.4 阶段三：更新 DeployUseCase

```rust
// application/deploy/use_case.rs

fn load_assets_from_layers(&self, options: &DeployOptions) -> Result<LayeredAssets, String> {
    // ... resolve layers (unchanged) ...
    
    // BEFORE:
    // for layer in &mut layers {
    //     layer.assets = self.asset_repo.load_all(layer.path.resolved())?;
    // }
    
    // AFTER:
    crate::application::layer_ops::load_resolved_layers(
        &self.asset_repo, 
        &mut resolution.layers
    )?;
    
    // ... merge (unchanged) ...
}
```

### 3.5 阶段四：更新 DiffUseCase

同样使用 `load_resolved_layers`。

### 3.6 阶段五：简化 FsLayerLoader

```rust
impl LayerLoader for FsLayerLoader {
    fn load_layer_assets(&self, layer: &mut Layer) -> Result<(), LayerLoadError> {
        // 验证逻辑保留
        let layer_root = layer.path.resolved();
        if !layer_root.exists() { ... }
        if !layer_root.is_dir() { ... }
        if let Err(e) = std::fs::read_dir(layer_root) { ... }
        
        // 使用公共函数
        let layers = std::slice::from_mut(layer);
        crate::application::layer_ops::load_resolved_layers(&self.asset_repo, layers)
            .map_err(|e| LayerLoadError::LoadFailed { message: e })?;
        
        Ok(())
    }
}
```

### 3.7 阶段六：清理 FsAssetRepository::load_all

恢复 `load_all` 为简单实现（不含 ignore 逻辑）：

```rust
impl AssetRepository for FsAssetRepository {
    fn load_all(&self, source: &Path) -> Result<Vec<Asset>> {
        // 简单实现：不过滤，让调用者负责 ignore
        // 用于向后兼容或单文件加载场景
        let ignore = IgnorePatterns::empty();
        let (assets, _) = self.load_all_with_ignore(source, &ignore)?;
        Ok(assets)
    }
}
```

---

## 4. 依赖影响分析

### 4.1 需要更新的 Mock

| Mock | 文件 | 变更 |
|------|------|------|
| `MockAssetRepository` | `application/deploy/tests.rs` | 添加 `load_all_with_ignore` |
| `MockAssetRepository` | `application/diff.rs` (tests) | 添加 `load_all_with_ignore` |

### 4.2 不需要改动的组件

| 组件 | 原因 |
|------|------|
| `LayerResolver` | 只负责解析 layers，不加载 assets |
| `merge_layers()` | 接收已加载的 layers |
| `WatchUseCase` | 通过 `DeployUseCase` 间接使用，自动生效 |
| 所有 adapters | 不涉及 asset 加载 |

### 4.3 需要验证的场景

| 场景 | 测试方法 |
|------|---------|
| CLI deploy + calvinignore | 新增端到端测试 |
| Interactive deploy + calvinignore | 新增端到端测试 |
| calvin diff + calvinignore | 新增端到端测试 |
| calvin layers --verbose + calvinignore | 验证 ignored_count 显示 |

---

## 5. 测试计划

按照 `TESTING_PHILOSOPHY.md`：

### 5.1 合约测试（Contracts）

| 文件 | 测试 | 合约 |
|------|------|------|
| `tests/contracts/calvinignore.rs` | `contract_calvinignore_excludes_matching_assets_from_deploy` | .calvinignore 模式在 deploy 时被尊重 |
| `tests/contracts/calvinignore.rs` | `contract_calvinignore_excludes_skill_directories` | 目录模式正确匹配 |
| `tests/contracts/calvinignore.rs` | `contract_calvinignore_works_for_all_targets` | 所有平台行为一致 |

### 5.2 回归测试（Regression）

| 文件 | 测试 | Bug |
|------|------|-----|
| `tests/regression/calvinignore.rs` | `regression_calvinignore_skills_template_deployed` | 原始 issue 复现 |

### 5.3 场景测试（Scenarios）

| 文件 | 测试 | 场景 |
|------|------|------|
| `tests/scenarios/calvinignore.rs` | `scenario_user_excludes_draft_files` | 用户忽略草稿文件 |
| `tests/scenarios/calvinignore.rs` | `scenario_user_excludes_template_skills` | 用户忽略模板 skill |

---

## 6. 迁移步骤（TDD 顺序）

| 步骤 | 任务 | 预期结果 |
|------|------|---------|
| 1 | 创建 `tests/contracts/calvinignore.rs` | 测试失败（功能未实现） |
| 2 | 创建 `tests/regression/calvinignore.rs` | 测试失败 |
| 3 | 扩展 `AssetRepository` trait | 编译失败（mock 缺失方法） |
| 4 | 更新所有 mock | 编译通过 |
| 5 | 创建 `application/layer_ops.rs` | 编译通过 |
| 6 | 更新 `DeployUseCase` 使用新函数 | 现有测试通过 |
| 7 | 更新 `DiffUseCase` 使用新函数 | 现有测试通过 |
| 8 | 简化 `FsLayerLoader` | 现有测试通过 |
| 9 | 运行新合约测试 | **测试通过** |
| 10 | 运行回归测试 | **测试通过** |
| 11 | 运行全量测试 | 全部通过 |

---

## 7. 最终架构图

```
┌──────────────────────────────────────────────────────────────────┐
│                    Unified Asset Loading                          │
└──────────────────────────────────────────────────────────────────┘

            ┌─────────────────────────────────────────┐
            │         load_resolved_layers()          │  ← 唯一加载入口
            │         (application/layer_ops.rs)      │
            │                                         │
            │  Uses: AssetRepository.load_all_with_   │
            │        ignore()                         │
            │  Does: ensure_unique_asset_ids()        │
            │        populate layer.assets            │
            │        track layer.ignored_count        │
            └─────────────────────────────────────────┘
                                │
            ┌───────────────────┼───────────────────┐
            │                   │                   │
            ▼                   ▼                   ▼
    ┌───────────────┐   ┌───────────────┐   ┌───────────────┐
    │ DeployUseCase │   │ DiffUseCase   │   │ FsLayerLoader │
    │               │   │               │   │               │
    │ load_assets   │   │ load_assets   │   │ load_layer    │
    │ _from_layers()│   │ _from_layers()│   │ _assets()     │
    │     │         │   │     │         │   │     │         │
    │     └─────────┼───┼─────┴─────────┼───┼─────┘         │
    └───────────────┘   └───────────────┘   └───────────────┘
            │                   │                   │
            └───────────────────┼───────────────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │   AssetRepository     │  ← trait (扩展)
                    │                       │
                    │ load_all()            │
                    │ load_all_with_ignore()│
                    │ load_by_path()        │
                    └───────────────────────┘
```

**关键改进**：
1. **唯一入口**：`load_resolved_layers()` 是所有 layer 加载的核心
2. **职责清晰**：`AssetRepository` 只负责文件解析
3. **代码减少**：消除 `FsAssetRepository::load_all()` 中的 ignore 逻辑
4. **可测试**：通过 trait 注入，mock 可完全模拟

---

## 8. 风险与回滚

### 8.1 风险

| 风险 | 概率 | 缓解措施 |
|------|------|---------|
| Mock 更新遗漏 | 中 | 编译会失败，强制修复 |
| 隐藏的 load_all() 调用 | 低 | grep 全量搜索 |
| 性能退化 | 低 | 公共函数逻辑与原实现相同 |

### 8.2 回滚

所有变更可逐阶段回滚：
- 阶段 1-5 是新增代码，回滚即删除
- 阶段 6-8 保留旧代码直到新代码稳定

---

## 9. 结论

此方案：

- ✅ **TDD 驱动**：先写合约测试，再实现
- ✅ **统一路径**：所有场景通过 `load_resolved_layers()` 加载
- ✅ **职责清晰**：`AssetRepository` (低层) vs `load_resolved_layers()` (高层)
- ✅ **不创建上帝对象**：使用公共函数而非超大类
- ✅ **可测试**：trait 扩展，mock 可注入

## 10. 实施记录

### 10.1 已完成的变更

| 文件 | 变更 |
|------|------|
| `src/domain/ports/asset_repository.rs` | 扩展 trait 添加 `load_all_with_ignore()` |
| `src/infrastructure/repositories/asset.rs` | 实现 `load_all_with_ignore()`，简化 `load_all()` |
| `src/application/layer_ops.rs` | **新增** - 统一的 `load_resolved_layers()` 函数 |
| `src/application/mod.rs` | 导出 `layer_ops` 模块 |
| `src/application/deploy/use_case.rs` | 使用 `load_resolved_layers()` |
| `src/application/diff.rs` | 使用 `load_resolved_layers()` |
| `src/infrastructure/layer/fs_loader.rs` | 使用 `load_resolved_layers()` |
| `tests/contracts/calvinignore.rs` | 新增合约测试 |
| `tests/regression/calvinignore_deploy.rs` | 新增回归测试 |
| `tests/common/env.rs` | 添加 `with_calvinignore()` builder 方法 |

### 10.2 测试结果

所有测试通过：
- ✅ 合约测试 (6 个)
- ✅ 回归测试 (2 个)
- ✅ 全量测试套件 (713+ 测试)

### 10.3 架构验证

重构后的代码符合预期：
1. **统一入口**：`load_resolved_layers()` 是所有 layer 资产加载的唯一入口
2. **职责清晰**：`AssetRepository` 负责文件解析，`load_resolved_layers()` 负责加载流程
3. **无上帝对象**：使用公共函数而非超大类
4. **完全可测试**：通过 trait 注入，mock 可完全模拟
