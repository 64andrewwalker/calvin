# 适配器模块迁移 - TDD 计划

> 将 `src/adapters/` 完全迁移到 `src/infrastructure/adapters/`

---

## 1. 当前状态分析

### 1.1 两套适配器对比

| 特性 | src/adapters/ (旧) | infrastructure/adapters/ (新) |
|------|-------------------|------------------------------|
| 输入类型 | `PromptAsset` | `domain::entities::Asset` |
| 输出类型 | `adapters::OutputFile` | `domain::entities::OutputFile` |
| Trait | `adapters::TargetAdapter` | `domain::ports::TargetAdapter` |
| 错误类型 | `CalvinResult` | `Result<_, AdapterError>` |
| 使用者 | sync/compile.rs, sync/pipeline.rs | DeployUseCase, DiffUseCase |

### 1.2 依赖链

```
src/adapters/
├── 被以下使用:
│   ├── sync/compile.rs (compile_assets)
│   ├── sync/pipeline.rs (AssetPipeline)
│   ├── sync/engine.rs (间接通过 pipeline)
│   └── lib.rs (公开导出)
│
└── 被以下间接使用:
    ├── watcher.rs (通过 SyncEngine)
    └── commands/interactive.rs (间接)

infrastructure/adapters/
├── 被以下使用:
│   ├── application/deploy.rs (DeployUseCase)
│   ├── application/diff.rs (DiffUseCase)
│   ├── presentation/factory.rs (创建适配器)
│   └── lib.rs (公开导出)
```

---

## 2. TDD 迁移计划

### Phase A: 准备工作 (1-2 小时)

#### A.1 添加类型转换测试

**文件**: `src/infrastructure/adapters/tests.rs` (新建)

```rust
#[cfg(test)]
mod conversion_tests {
    use crate::domain::entities::Asset;
    use crate::models::PromptAsset;

    /// Test that PromptAsset can be converted to Asset
    #[test]
    fn prompt_asset_converts_to_domain_asset() {
        let prompt_asset = PromptAsset::new_test("test-id", "Test description", "Content");
        let domain_asset = Asset::from(prompt_asset.clone());
        
        assert_eq!(domain_asset.id(), prompt_asset.id);
        assert_eq!(domain_asset.description(), &prompt_asset.frontmatter.description);
    }

    /// Test that adapters::OutputFile can be converted to domain::entities::OutputFile
    #[test]
    fn legacy_output_file_converts_to_domain_output_file() {
        use crate::adapters::OutputFile as LegacyOutput;
        use crate::domain::entities::OutputFile as DomainOutput;
        
        let legacy = LegacyOutput::new("path/to/file.md", "content");
        let domain = DomainOutput::from(legacy);
        
        assert_eq!(domain.path(), Path::new("path/to/file.md"));
        assert_eq!(domain.content(), "content");
    }
}
```

#### A.2 实现类型转换

**文件**: `src/domain/entities/asset.rs`

```rust
impl From<crate::models::PromptAsset> for Asset {
    fn from(pa: crate::models::PromptAsset) -> Self {
        // ... conversion logic
    }
}
```

**文件**: `src/domain/entities/output_file.rs`

```rust
impl From<crate::adapters::OutputFile> for OutputFile {
    fn from(legacy: crate::adapters::OutputFile) -> Self {
        // ... conversion logic
    }
}
```

---

### Phase B: 迁移 sync/compile.rs (2-3 小时)

#### B.1 测试旧 compile_assets 行为

**文件**: `src/sync/compile_migration_test.rs` (临时)

```rust
#[test]
fn compile_assets_produces_same_output_as_new_engine() {
    let assets = vec![create_test_asset()];
    let config = Config::default();
    
    // Old engine
    let old_outputs = compile_assets(&assets, &[], &config).unwrap();
    
    // New engine
    let new_outputs = new_compile(&assets, &[], &config);
    
    // Compare (paths and content should match)
    assert_eq!(old_outputs.len(), new_outputs.len());
    for (old, new) in old_outputs.iter().zip(new_outputs.iter()) {
        assert_eq!(old.path, new.path());
        assert_eq!(old.content, new.content());
    }
}
```

#### B.2 修改 sync/compile.rs 使用新适配器

```rust
// Before
use crate::adapters::{all_adapters, OutputFile};

// After
use crate::infrastructure::adapters::all_adapters;
use crate::adapters::OutputFile; // Keep for compatibility

pub fn compile_assets(...) -> CalvinResult<Vec<OutputFile>> {
    // Convert PromptAsset to Asset
    // Use new adapters
    // Convert OutputFile back to legacy format
}
```

---

### Phase C: 迁移 sync/pipeline.rs (2-3 小时)

#### C.1 测试 AssetPipeline 行为

```rust
#[test]
fn asset_pipeline_compiles_assets_correctly() {
    let temp = tempdir().unwrap();
    setup_promptpack(&temp);
    
    let pipeline = AssetPipeline::new(temp.path().join(".promptpack"), vec![], Config::default());
    let outputs = pipeline.compile().unwrap();
    
    assert!(!outputs.is_empty());
    // Verify specific outputs exist
}
```

#### C.2 更新 AssetPipeline 使用新编译逻辑

---

### Phase D: 删除 src/adapters/ (1-2 小时)

#### D.1 前置条件检查

```bash
# 确认没有直接使用 src/adapters/
grep -r "use crate::adapters::" src/ --include="*.rs" | grep -v sync/ | grep -v adapters/
```

#### D.2 更新 lib.rs 导出

```rust
// Before
pub use adapters::{all_adapters, get_adapter, OutputFile, TargetAdapter};

// After
pub use infrastructure::adapters::{all_adapters, get_adapter};
pub use domain::entities::OutputFile;
pub use domain::ports::TargetAdapter;
```

#### D.3 删除目录

```bash
rm -rf src/adapters/
```

#### D.4 修复编译错误

---

## 2.5 ⚠️ 发现的关键问题

**日期**: 2025-12-20

### 问题描述

在尝试将 `sync/compile.rs` 切换到新适配器时，发现新旧适配器的 **行为不一致**：

| 适配器 | 旧行为 | 新行为 |
|--------|--------|--------|
| ClaudeCode | 为所有资产类型生成命令 | 只为 Action/Agent 生成命令，Policy 不生成 |
| Cursor | 生成 `.cursorrules` | 生成 `.cursor/rules/<id>/RULE.md` |
| VSCode | 生成聚合文件 | 生成独立的 `.instructions.md` 文件 |

### 影响

这意味着不能简单地替换 `compile_assets` 使用新适配器，因为会破坏：
1. Golden 测试期望旧格式
2. 用户现有的工作流程
3. 向后兼容性

### 解决方案选项

1. **方案 A**: 修改新适配器使其与旧适配器行为一致（推荐）
   - 优点：保持向后兼容
   - 缺点：需要理解每个适配器的预期行为

2. **方案 B**: 保持两套适配器并行
   - 优点：无需修改
   - 缺点：代码重复，维护成本高

3. **方案 C**: 接受新行为，更新所有测试
   - 优点：代码更简洁
   - 缺点：破坏性变更

### 决定

采用 **方案 A**：逐步对齐新适配器的行为，确保输出与旧适配器兼容。

Phase B.2 状态：**暂停** - 需要先对齐适配器行为。

---

## 3. 风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| sync/pipeline 功能回归 | 高 | TDD 确保行为一致 |
| watcher 命令失效 | 中 | 保持 SyncEngine 接口不变 |
| 公开 API 变更 | 低 | 保持类型别名 |

---

## 4. 执行顺序

```
Week 1:
├── Day 1: Phase A - 准备工作
│   ├── A.1 添加转换测试
│   └── A.2 实现类型转换
│
├── Day 2-3: Phase B - 迁移 compile.rs
│   ├── B.1 添加行为测试
│   └── B.2 修改使用新适配器
│
├── Day 4-5: Phase C - 迁移 pipeline.rs
│   ├── C.1 添加测试
│   └── C.2 更新实现
│
└── Day 6: Phase D - 删除 src/adapters/
    ├── D.1 前置检查
    ├── D.2 更新导出
    ├── D.3 删除文件
    └── D.4 修复错误
```

---

## 5. 验证清单

### Phase A 完成条件
- [ ] 类型转换测试全部通过
- [ ] From trait 实现完成
- [ ] 无编译错误

### Phase B 完成条件
- [ ] compile_assets 输出一致性测试通过
- [ ] sync/compile.rs 使用新适配器
- [ ] 所有现有测试通过

### Phase C 完成条件
- [ ] AssetPipeline 测试通过
- [ ] watcher 命令正常工作
- [ ] 所有现有测试通过

### Phase D 完成条件
- [ ] src/adapters/ 目录删除
- [ ] lib.rs 导出更新
- [ ] 所有测试通过
- [ ] cargo clippy 无警告

---

## 6. 当前进度 (2025-12-20)

### 已完成

- [x] Phase A: 类型转换 (`From` traits)
- [x] Phase B.1: 一致性测试
- [x] Phase B.2: 对齐 ClaudeCode 适配器行为
- [x] Phase B.3: `sync/compile.rs` 迁移到新适配器
- [x] Phase C: 统一 OutputFile API (`.path()`, `.content()` 方法)

### 剩余工作

1. **VSCode/Codex/Antigravity infrastructure adapters 仍是 wrapper**
   - 它们持有 `legacy_adapter` 实例，调用旧适配器
   - 需要将逻辑完全移入新适配器

2. **`sync/mod.rs` 仍依赖 `adapters::OutputFile`**
   - 作为 re-export 使用
   - 需要迁移到 `domain::entities::OutputFile`

3. **`lib.rs` 公共导出**
   - `all_adapters`, `get_adapter`, `OutputFile`, `TargetAdapter` 仍从旧模块导出

### 阻塞因素

删除 `src/adapters/` 需要先完成以下工作：
1. 重写 VSCode/Codex/Antigravity 适配器（移除对旧适配器的依赖）
2. 迁移 `sync` 模块使用 `domain::entities::OutputFile`
3. 更新 `lib.rs` 公共 API

### 估计工作量

- 重写 3 个 infrastructure adapters: ~2-3 小时
- 迁移 OutputFile 类型: ~1 小时
- 更新公共 API: ~30 分钟
- 测试和修复: ~1 小时

