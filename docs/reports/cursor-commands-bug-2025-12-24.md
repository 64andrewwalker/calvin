# Bug 报告：Cursor 单独部署时 Commands 文件丢失

> 报告日期：2025-12-24
> 严重程度：Critical
> 影响范围：Cursor-only 部署到 Home 和 Project

---

## 问题描述

当用户使用 `calvin deploy --home --target cursor`（单独部署 Cursor 到 home 目录）时，`~/.cursor/commands/` 目录不会收到任何 Action 或 Agent 类型的命令文件。

同样，`calvin deploy --target cursor` 也不会在 `.cursor/commands/` 生成任何命令文件。

## 根本原因

### 设计意图（正确）

Cursor 可以读取 Claude Code 的命令目录（`~/.claude/commands/`），因此当同时部署 Cursor 和 Claude Code 时，不需要为 Cursor 重复生成 commands 文件。

公共编译函数 `src/application/compiler.rs::compile_assets` 正确实现了这个逻辑：

```rust
// 第 83-91 行
let has_claude_code = targets.is_empty() || targets.contains(&Target::ClaudeCode);
let has_cursor = targets.is_empty() || targets.contains(&Target::Cursor);

// Cursor 需要生成 commands 仅当：
// 1. Cursor 在目标列表中 AND
// 2. Claude Code 不在目标列表中
let cursor_needs_commands = has_cursor && !has_claude_code;
```

### 问题所在（代码重复导致不一致）

存在**三个独立的** `compile_assets` 实现：

| 位置 | 有 cursor_needs_commands？ | 被谁使用？ |
|------|---------------------------|-----------|
| `src/application/compiler.rs` (公共函数) | ✅ 有 | AssetPipeline, WatchUseCase |
| `src/application/deploy/use_case.rs` (私有方法) | ❌ **没有** | **DeployUseCase** |
| `src/application/diff.rs` (私有方法) | ❌ **没有** | **DiffUseCase** |

### 影响路径分析

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           编译资产的调用路径                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  calvin deploy [--home] [--target cursor]                                   │
│       │                                                                     │
│       └──► DeployUseCase::execute                                           │
│                 │                                                           │
│                 └──► self.compile_assets (私有方法) ❌ 缺少 cursor_needs     │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  calvin diff [--home] [--target cursor]                                     │
│       │                                                                     │
│       └──► DiffUseCase::execute                                             │
│                 │                                                           │
│                 └──► self.compile_assets (私有方法) ❌ 缺少 cursor_needs     │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  calvin watch [--home] [--target cursor]                                    │
│       │                                                                     │
│       └──► WatchUseCase::perform_sync_incremental                           │
│                 │                                                           │
│                 └──► AssetPipeline::compile_incremental                     │
│                           │                                                 │
│                           └──► compile_assets (公共函数) ✅ 有 cursor_needs  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 受影响的场景

| 命令 | 场景 | 状态 |
|------|------|------|
| `calvin deploy --target cursor` | Cursor-only 部署到项目 | ❌ 丢失 commands |
| `calvin deploy --home --target cursor` | Cursor-only 部署到 home | ❌ 丢失 commands |
| `calvin diff --target cursor` | Cursor-only diff 预览 | ❌ 不显示 commands |
| `calvin watch --target cursor` | Cursor-only 监视模式 | ✅ 正确生成 |
| `calvin deploy` (默认所有目标) | 全目标部署 | ✅ 正确（Claude Code 提供 commands） |

## 修复建议

### 方案 A：统一编译逻辑（推荐）

将 `cursor_needs_commands` 逻辑提取为**共享服务**，消除代码重复：

```rust
// src/domain/services/compiler.rs (新文件)

/// 编译服务，包含所有平台特定的编译逻辑
pub struct CompilerService {
    adapters: Vec<Box<dyn TargetAdapter>>,
}

impl CompilerService {
    /// 编译资产，包含 cursor_needs_commands 逻辑
    pub fn compile(
        &self,
        assets: &[Asset],
        targets: &[Target],
    ) -> Result<Vec<OutputFile>, String> {
        let has_claude_code = targets.is_empty() || targets.contains(&Target::ClaudeCode);
        let has_cursor = targets.is_empty() || targets.contains(&Target::Cursor);
        let cursor_needs_commands = has_cursor && !has_claude_code;

        let mut outputs = Vec::new();

        // ... 标准编译逻辑 ...

        // Cursor 命令特殊处理
        if adapter.target() == Target::Cursor && cursor_needs_commands {
            // ... 生成 commands ...
        }

        Ok(outputs)
    }
}
```

然后让 `DeployUseCase`、`DiffUseCase` 都依赖此服务。

### 方案 B：直接修复私有方法

在 `DeployUseCase::compile_assets` 和 `DiffUseCase::compile_assets` 中添加 `cursor_needs_commands` 逻辑。

缺点：继续维护重复代码。

### 方案 C：让 Use Cases 调用公共函数

让 `DeployUseCase` 和 `DiffUseCase` 调用 `src/application/compiler.rs::compile_assets`。

缺点：需要处理类型转换（公共函数使用 legacy types）。

## 测试覆盖分析

现有测试 `compile_assets_cursor_only_generates_commands` 在 `src/application/compiler.rs` 中，覆盖了公共函数的行为，但**没有覆盖** `DeployUseCase` 的私有方法。

应添加的测试：
- `DeployUseCase` 的 Cursor-only 部署测试
- `DiffUseCase` 的 Cursor-only diff 测试
- 集成测试：`calvin deploy --target cursor` 验证 commands 文件

## CursorAdapter 设计说明

`src/infrastructure/adapters/cursor.rs` 中的注释：

```rust
//! Note: Cursor reads Claude's commands from ~/.claude/commands/,
//! so we only generate rules here, not commands.
```

这个设计决策是正确的——CursorAdapter 不应该默认生成 commands。特殊处理应该在**编译器层**根据目标列表动态决定。

## 相关文件

| 文件 | 说明 |
|------|------|
| `src/application/compiler.rs` | 公共 compile_assets 函数 (正确) |
| `src/application/deploy/use_case.rs` | DeployUseCase 私有 compile_assets (缺失逻辑) |
| `src/application/diff.rs` | DiffUseCase 私有 compile_assets (缺失逻辑) |
| `src/infrastructure/adapters/cursor.rs` | CursorAdapter (按设计不生成 commands) |
| `src/application/pipeline.rs` | AssetPipeline (使用公共函数，正确) |

## 优先级

**Critical** - 这是一个严重的功能缺失，导致用户无法正常使用 Cursor-only 部署。

## 下一步

1. 确定采用哪个修复方案
2. 实施修复
3. 添加回归测试
4. 更新文档说明 Cursor 的 commands 生成条件

