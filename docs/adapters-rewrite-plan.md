# 适配器重写计划：VSCode / Codex / Antigravity

> **目标**: 将这3个 infrastructure 适配器从 wrapper 模式重写为独立实现，移除对 `src/adapters/` 的依赖

---

## 1. 当前状态分析

### 1.1 存在的问题

这3个 infrastructure 适配器目前是 **wrapper**，包装旧适配器：

```rust
// 当前实现
pub struct VSCodeAdapter {
    legacy_adapter: crate::adapters::vscode::VSCodeAdapter,  // 依赖旧模块
}
```

这导致无法删除 `src/adapters/` 目录。

### 1.2 旧适配器缺陷分析

| 适配器 | 缺陷 | 改进建议 |
|--------|------|----------|
| **VSCode** | `merge_mode` 字段需要在构造时决定 | 改为编译时参数或配置驱动 |
| **VSCode** | `split_mode` 和 `merge_mode` 容易混淆 | 统一为单一配置项 |
| **VSCode** | AGENTS.md 生成在 `post_compile` 中硬编码 | 应该由专门的 post_compile 处理 |
| **Codex** | 始终添加 `$ARGUMENTS` 占位符 | 应根据资产类型决定 |
| **Codex** | frontmatter 固定格式 | 应该更灵活 |
| **Antigravity** | 所有资产都放 `workflows/` | 应区分 rules 和 workflows |
| **Antigravity** | 缺少 globs/apply 支持 | 应支持文件匹配模式 |
| **通用** | 错误处理返回 `CalvinResult` | 新适配器应返回 `AdapterError` |
| **通用** | 使用 `PromptAsset` 类型 | 应使用 `domain::entities::Asset` |

---

## 2. 重写策略

### 2.1 总体原则

1. **不使用 wrapper** - 完全独立实现
2. **使用 domain 类型** - `Asset`, `OutputFile`, `Scope`, `Target`
3. **使用 AdapterError** - 统一错误处理
4. **TDD** - 先写测试，再实现

### 2.2 文件结构

```
src/infrastructure/adapters/
├── mod.rs           # 导出和 all_adapters()
├── claude_code.rs   # ✅ 已独立实现
├── cursor.rs        # ✅ 已独立实现
├── vscode.rs        # 🔄 需要重写
├── codex.rs         # 🔄 需要重写
├── antigravity.rs   # 🔄 需要重写
└── common.rs        # 共享工具函数 (可选)
```

---

## 3. VSCode 适配器重写

### 3.1 输出规范

| 资产类型 | Scope | 输出路径 |
|---------|-------|---------|
| Policy | Project | `.github/instructions/<id>.instructions.md` |
| Policy | User | `~/.vscode/instructions/<id>.instructions.md` |
| Action | Project | `.github/instructions/<id>.instructions.md` |
| Action | User | `~/.vscode/instructions/<id>.instructions.md` |
| Agent | Project | `.github/instructions/<id>.instructions.md` |
| Agent | User | `~/.vscode/instructions/<id>.instructions.md` |

### 3.2 Frontmatter 格式

```yaml
---
description: <asset.description>
applyTo: "<asset.apply>"  # 仅当有 apply 时
---
```

### 3.3 改进点

1. **移除 `merge_mode`** - 始终生成独立文件（默认行为）
2. **简化 API** - 不需要 `with_split_mode` / `with_merge_mode`
3. **AGENTS.md** - 移到通用 `post_compile` 逻辑

### 3.4 测试用例

```rust
#[test] fn compile_policy_project_scope()
#[test] fn compile_policy_user_scope()
#[test] fn compile_action_generates_instruction()
#[test] fn compile_with_apply_includes_applyto()
#[test] fn compile_includes_footer()
#[test] fn validate_empty_content_warns()
#[test] fn security_baseline_returns_empty()
```

---

## 4. Codex 适配器重写

### 4.1 输出规范

| 资产类型 | Scope | 输出路径 |
|---------|-------|---------|
| Policy | Project | `.codex/prompts/<id>.md` |
| Policy | User | `~/.codex/prompts/<id>.md` |
| Action | Project | `.codex/prompts/<id>.md` |
| Action | User | `~/.codex/prompts/<id>.md` |
| Agent | Project | `.codex/prompts/<id>.md` |
| Agent | User | `~/.codex/prompts/<id>.md` |

### 4.2 Frontmatter 格式

```yaml
---
description: <asset.description>
argument-hint: <arguments>  # 仅对 Action/Agent
---
```

### 4.3 改进点

1. **条件 `$ARGUMENTS`** - 仅对 Action/Agent 添加
2. **Policy 不需要参数提示** - 区分处理

### 4.4 测试用例

```rust
#[test] fn compile_action_includes_arguments()
#[test] fn compile_policy_no_arguments()
#[test] fn compile_user_scope_uses_home()
#[test] fn compile_project_scope_local_path()
#[test] fn validate_undocumented_placeholder_warns()
#[test] fn validate_documented_placeholder_ok()
```

---

## 5. Antigravity 适配器重写

### 5.1 输出规范

| 资产类型 | Scope | 输出路径 |
|---------|-------|---------|
| Policy | Project | `.agent/rules/<id>.md` |
| Policy | User | `~/.gemini/antigravity/global_rules/<id>.md` |
| Action | Project | `.agent/workflows/<id>.md` |
| Action | User | `~/.gemini/antigravity/global_workflows/<id>.md` |
| Agent | Project | `.agent/workflows/<id>.md` |
| Agent | User | `~/.gemini/antigravity/global_workflows/<id>.md` |

### 5.2 改进点

1. **区分 rules 和 workflows** - Policy → rules, Action/Agent → workflows
2. **添加 globs 支持** - 如果有 `apply` 字段

### 5.3 Frontmatter 格式

```yaml
---
description: <asset.description>
globs: "<asset.apply>"  # 仅当有 apply 时
---
```

### 5.4 测试用例

```rust
#[test] fn compile_policy_to_rules_dir()
#[test] fn compile_action_to_workflows_dir()
#[test] fn compile_agent_to_workflows_dir()
#[test] fn compile_with_apply_includes_globs()
#[test] fn compile_user_scope_uses_home()
#[test] fn validate_empty_content_warns()
```

---

## 6. 执行计划

### Phase 1: VSCode 适配器 (~1 小时)

1. 创建测试用例 (`vscode.rs` tests)
2. 实现独立的 compile 逻辑
3. 移除 `legacy_adapter` 依赖
4. 验证所有测试通过

### Phase 2: Codex 适配器 (~45 分钟)

1. 创建测试用例
2. 实现独立逻辑
3. 条件处理 `$ARGUMENTS`
4. 验证测试

### Phase 3: Antigravity 适配器 (~45 分钟)

1. 创建测试用例
2. 区分 rules/workflows 目录
3. 添加 globs 支持
4. 验证测试

### Phase 4: 集成验证 (~30 分钟)

1. 运行全部单元测试
2. 运行 golden 测试
3. 运行集成测试
4. 确认无回归

### Phase 5: 清理 (~30 分钟)

1. 从 `sync/mod.rs` 移除 OutputFile re-export
2. 更新 `lib.rs` 导出
3. 删除 `src/adapters/` 目录
4. 修复编译错误
5. 最终测试验证

---

## 7. 风险评估

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 输出格式不兼容 | 高 | TDD 确保与 golden 测试一致 |
| 遗漏功能 | 中 | 对照旧适配器逐行检查 |
| 公共 API 变更 | 低 | 保持类型别名向后兼容 |

---

## 8. 验证清单

### 每个适配器完成条件

- [ ] 所有 TDD 测试通过
- [ ] 无 `legacy_adapter` 依赖
- [ ] 使用 `domain::entities::Asset`
- [ ] 使用 `domain::entities::OutputFile`
- [ ] 返回 `Result<_, AdapterError>`

### 最终完成条件

- [ ] `src/adapters/` 目录删除
- [ ] `lib.rs` 导出更新
- [ ] 所有 588+ 测试通过
- [ ] 所有 golden 测试通过
- [ ] cargo clippy 无警告

