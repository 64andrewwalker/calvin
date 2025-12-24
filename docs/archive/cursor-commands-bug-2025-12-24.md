# Bug 报告：Cursor 单独部署时 Commands 文件丢失

> 报告日期：2025-12-24
> 修复日期：2025-12-24
> 严重程度：Critical
> 状态：**已修复** ✅

---

## 问题描述

当用户使用 `calvin deploy --home --target cursor`（单独部署 Cursor 到 home 目录）时，`~/.cursor/commands/` 目录不会收到任何 Action 或 Agent 类型的命令文件。

## 根本原因

存在**三个独立的** `compile_assets` 实现，只有公共函数包含 `cursor_needs_commands` 逻辑。

| 位置 | 有 cursor_needs_commands？ |
|------|---------------------------|
| `src/application/compiler.rs` (公共函数) | ✅ 有 |
| `src/application/deploy/use_case.rs` (私有方法) | ❌ **没有** |
| `src/application/diff.rs` (私有方法) | ❌ **没有** |

---

## 修复方案

创建 `CompilerService` 领域服务，统一编译逻辑：

```rust
// src/domain/services/compiler_service.rs
impl CompilerService {
    pub fn cursor_needs_commands(targets: &[Target]) -> bool {
        let has_claude_code = targets.is_empty() || targets.contains(&Target::ClaudeCode);
        let has_cursor = targets.is_empty() || targets.contains(&Target::Cursor);
        has_cursor && !has_claude_code
    }
    
    pub fn generate_command_content(asset: &Asset, footer: &str) -> String {
        // ...
    }
}
```

所有使用点现在调用 `CompilerService::cursor_needs_commands()` 静态方法。

---

## 测试覆盖

- `application::compiler::tests::compile_assets_cursor_only_generates_commands`
- `deploy_cursor_only_generates_commands` (e2e)
- `deploy_cursor_with_claude_code_no_duplicate_commands` (e2e)

---

## 相关文档

- `docs/archive/compiler-service-refactor-plan.md` - 重构计划

