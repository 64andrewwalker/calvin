# Multi-Layer PromptPack Design Principles

> Core design principles guiding the multi-layer feature implementation.

## Core Philosophy

### 1. Single Source of Truth (实现而非违反)

**原则**：每个 prompt 规则只在一个地方定义，通过层级引用复用。

**错误做法**：
```
项目A/.promptpack/code-style.md  ← 复制
项目B/.promptpack/code-style.md  ← 复制
```

**正确做法**：
```
~/.calvin/.promptpack/code-style.md  ← 定义一次
    ↓ 被引用
项目A/.promptpack/  （可选覆盖）
项目B/.promptpack/  （可选覆盖）
```

### 2. Convention over Configuration

**原则**：合理的默认值，最小化必需配置。

- 默认用户层路径：`~/.calvin/.promptpack`
- 如果用户层不存在：静默跳过，不报错
- 如果项目层不存在但有其他层：使用其他层
- 合并策略：高优先级完全覆盖，无需配置

### 3. Explicit over Implicit

**原则**：关键行为必须清晰可见，不静默失败。

- 层覆盖：verbose 模式下显示覆盖关系
- 产物来源：lockfile 记录每个输出的来源
- 错误消息：包含 Fix 建议和 Docs 链接

### 4. Backward Compatibility

**原则**：现有项目无需修改即可正常工作。

- 没有 `~/.calvin/.promptpack`：行为与旧版完全一致
- 新配置项都有默认值
- Lockfile 格式向后兼容（旧版可读新格式）

---

## Layer Priority Model

```
Priority: Low → High

┌─────────────────────────┐
│  User Layer             │  ~/.calvin/.promptpack
│  (Personal defaults)    │  用户个人的全局规则
└────────────┬────────────┘
             │ can be overridden by
             ▼
┌─────────────────────────┐
│  Custom Layers          │  配置的额外路径
│  (Team/Domain shared)   │  团队共享规则
└────────────┬────────────┘
             │ can be overridden by
             ▼
┌─────────────────────────┐
│  Project Layer          │  ./.promptpack
│  (Project-specific)     │  项目特有规则
└─────────────────────────┘
```

### Merge Rules

1. **Same ID**：高层完全覆盖低层（无字段级合并）
2. **Different ID**：全部保留
3. **Config**：Section 级别覆盖

---

## Key Design Decisions

### D1: Lockfile 位置迁移

**决定**：从 `.promptpack/.calvin.lock` 迁移到 `./calvin.lock`

**理由**：
- 符合行业惯例（npm, Cargo, Poetry）
- Lockfile 追踪输出状态，不是输入配置
- 多层场景下只需一个 lockfile

### D2: 用户层路径可配置

**决定**：`sources.user_layer_path` 可以是任意路径

**理由**：
- 用户可能在 dotfiles repo 中维护 prompts
- 跨机器同步更方便
- 默认 `~/.calvin/.promptpack` 保持简单

### D3: 项目层不存在时的行为

**决定**：如果有其他层，警告并继续；无任何层时报错

**理由**：
- 用户可能只想用全局层
- 不应强制每个项目都有 `.promptpack/`
- 但完全没有层是错误状态

### D4: 产物来源追踪

**决定**：Lockfile 记录每个输出文件的来源层、来源 asset、来源文件

**理由**：
- 用户需要知道每个输出来自哪里
- 支持 `calvin provenance` 命令
- 调试和审计更容易

### D5: 全局 Registry

**决定**：`~/.calvin/registry.toml` 追踪所有 calvin 管理的项目

**理由**：
- 支持 `calvin projects` 列出所有项目
- 支持 `calvin clean --all` 批量清理
- 不需要遍历文件系统查找项目

---

## Error Handling Principles

### E1: 友好的错误消息

每个错误必须包含：
- **What**：清晰描述什么出错了
- **Fix**：如何修复
- **Docs**：相关文档链接

### E2: 非静默失败

| 情况 | 行为 |
|------|------|
| 没有任何 layer | 错误 |
| User layer 不存在 | 静默跳过 |
| Additional layer 不存在 | 警告 + 继续 |
| Project layer 不存在 | 警告 + 使用其他层 |
| Asset ID 冲突 (同层) | 错误 |
| Asset ID 冲突 (跨层) | 提示覆盖 |

### E3: 层级上下文

错误消息应包含层级上下文，帮助用户理解问题发生在哪个层。

---

## UI Principles

### U1: 层级可视化

在 verbose 模式下，始终显示层级栈和来源信息。

### U2: 覆盖关系清晰

当发生覆盖时，明确显示哪个层覆盖了哪个层。

### U3: 一致的视觉风格

使用现有 UI 组件（Box, Icon, ColoredText）保持一致性。

---

## TDD Approach

每个阶段的开发必须遵循 TDD：

1. **Red**：先写失败的测试
2. **Green**：写最少代码让测试通过
3. **Refactor**：重构代码，保持测试通过

每个 commit 都应该：
- 有完整的测试覆盖
- 能独立运行端到端
- 不破坏现有功能

