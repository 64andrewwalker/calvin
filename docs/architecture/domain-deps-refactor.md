# Domain 层依赖重构计划

> **Created**: 2025-12-20
> **Status**: ✅ 完成
> **Priority**: P1
> **Updated**: 2025-12-20

---

## 问题描述

Domain 层（`src/domain/`）违反了"领域层不依赖外层"的架构原则，存在以下依赖：

### 原违规（已修复）

| 文件 | 依赖 | 问题 | 状态 |
|------|------|------|------|
| `domain/policies/security.rs` | `crate::config::SecurityMode` | 依赖 config 模块的类型 | ✅ 已修复 |
| `domain/ports/config_repository.rs` | `crate::config::{Config, ConfigWarning, DeployTargetConfig}` | 依赖整个 Config 结构 | ✅ 已修复 |
| `domain/policies/scope_policy.rs` | `crate::models::PromptAsset` | 依赖 legacy models 模块 | ✅ 已修复 |

### 为什么这是问题？

1. **违反分层原则**: Domain 层应该是最内层，不应依赖任何外层
2. **测试困难**: 测试 Domain 时需要引入外部依赖
3. **循环依赖风险**: 外层修改可能影响核心领域逻辑
4. **可移植性差**: Domain 层无法独立于其他模块使用

---

## 重构目标

将所有 Domain 需要的类型**内化**到 Domain 层，外层依赖 Domain 而非相反。

### 目标依赖图

```
Before:                           After:
domain → config                   config → domain
domain → models                   models → domain
                                  domain → (nothing external)
```

---

## 重构步骤

### Step 1: 移动 SecurityMode 到 Domain

**任务**: 将 `SecurityMode` 枚举移动到 `domain/value_objects/`

**当前位置**: `src/config/types.rs`
```rust
pub enum SecurityMode {
    Yolo,
    Balanced,
    Strict,
}
```

**目标位置**: `src/domain/value_objects/security_mode.rs`

**更新依赖**:
- `src/config/types.rs` → `use crate::domain::value_objects::SecurityMode`
- `src/domain/policies/security.rs` → 直接使用本模块

### Step 2: 提取 Domain 需要的 Config 类型

**任务**: 创建 Domain 专用的配置值对象

**目标**: `src/domain/value_objects/deploy_target.rs`

```rust
/// 部署目标配置 (Domain 层定义)
#[derive(Debug, Clone, Default)]
pub struct DeployTarget {
    pub scope: Option<Scope>,
    pub targets: Vec<Target>,
}
```

**更新 ConfigRepository port**:
```rust
pub trait ConfigRepository: Send + Sync {
    fn load_deploy_target(&self, path: &Path) -> Result<DeployTarget>;
    fn save_deploy_target(&self, path: &Path, target: &DeployTarget) -> Result<()>;
    // ... 其他方法返回 Domain 类型
}
```

### Step 3: 替换 PromptAsset 为 Asset

**任务**: 在 `scope_policy.rs` 中使用 `domain::entities::Asset`

**当前代码**:
```rust
use crate::models::PromptAsset;

impl ScopePolicy {
    pub fn filter_by_scope(&self, assets: &[PromptAsset]) -> Vec<&PromptAsset>
}
```

**目标代码**:
```rust
use crate::domain::entities::Asset;

impl ScopePolicy {
    pub fn filter_by_scope(&self, assets: &[Asset]) -> Vec<&Asset>
}
```

### Step 4: 更新 config 模块依赖方向

**任务**: 让 `config/types.rs` 依赖 `domain/value_objects`

**更新文件**:
- `src/config/types.rs` - 导入 SecurityMode from domain
- `src/config/mod.rs` - re-export SecurityMode

### Step 5: 更新 models 模块依赖方向

**任务**: 让 `models.rs` 依赖 `domain/value_objects`

**考虑**: `models.rs` 本身是 legacy 代码，可能需要更大范围重构

---

## 执行检查清单

- [x] **Step 1**: 移动 SecurityMode ✅ 完成
  - [x] 创建 `domain/value_objects/security_mode.rs`
  - [x] 移动 SecurityMode 枚举
  - [x] 更新 `domain/value_objects/mod.rs`
  - [x] 更新 `domain/policies/security.rs` 导入
  - [x] 更新 `config/types.rs` 为 re-export
  - [x] 运行测试验证

- [x] **Step 2**: 提取 DeployTarget 和 ConfigWarning ✅ 完成
  - [x] 创建 `domain/value_objects/deploy_target.rs`
  - [x] 创建 `domain/value_objects/config_warning.rs`
  - [x] 重构 `ConfigRepository` trait 使用泛型 `DomainConfig`
  - [x] 创建 `DomainConfig` trait 抽象配置接口
  - [x] `Config` 实现 `DomainConfig` trait
  - [x] 移动 `Target` 到 domain 层，`models.rs` 重新导出
  - [x] 运行测试验证 (636 tests passed)

- [x] **Step 3**: 移动 ScopePolicyExt ✅ 完成
  - [x] 将 `ScopePolicyExt` trait 移到 `application/pipeline.rs`
  - [x] 从 `domain/policies/scope_policy.rs` 移除 PromptAsset 依赖
  - [x] 将相关测试移到 application 层
  - [x] 运行测试验证

- [x] **Step 4**: 验证依赖图 ✅ 完成
  - [x] domain/policies/security.rs 不再依赖 crate::config
  - [x] domain/policies/scope_policy.rs 不再依赖 crate::models
  - [x] domain/ports/config_repository.rs 不再依赖 crate::config (使用 DomainConfig trait)
  - [x] domain/entities/asset.rs 测试中仍依赖 crate::models (可接受，用于 From trait 测试)
  - [x] 全部 636 个测试通过

---

## 风险评估

| 风险 | 可能性 | 影响 | 缓解措施 |
|------|--------|------|----------|
| 破坏现有测试 | 中 | 中 | 每步后运行测试 |
| API 变更影响外部代码 | 低 | 高 | 保持 re-export 兼容 |
| 循环依赖 | 低 | 高 | 仔细规划导入顺序 |

---

## 预期结果

重构后的依赖关系：

```
┌─────────────────────────────────────────────────────────────┐
│  presentation/commands → application → domain               │
│                                           ↑                  │
│  config ─────────────────────────────────┘                  │
│  models ─────────────────────────────────┘                  │
│  infrastructure ─────────────────────────┘                  │
└─────────────────────────────────────────────────────────────┘
```

Domain 层将完全自包含，不依赖任何外部模块。

