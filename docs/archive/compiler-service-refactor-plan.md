# CompilerService 重构计划

> 创建日期：2025-12-24
> 完成日期：2025-12-24
> 分支：feat/compiler-service-complete
> 状态：**已完成** ✅

---

## 背景

### 问题

当用户使用 `calvin deploy --target cursor`（单独部署 Cursor）时，`~/.cursor/commands/` 目录不会收到任何 Action 或 Agent 类型的命令文件。

### 根因

存在三个独立的 `compile_assets` 实现，只有公共函数包含 `cursor_needs_commands` 逻辑：

| 位置 | 有 cursor_needs_commands？ |
|------|---------------------------|
| `src/application/compiler.rs` (公共函数) | ✅ 有 |
| `src/application/deploy/use_case.rs` (私有方法) | ❌ 没有 |
| `src/application/diff.rs` (私有方法) | ❌ 没有 |

### 解决方案

创建 `CompilerService` 领域服务，统一编译逻辑，消除代码重复。

---

## TODO Checklist

```markdown
## Phase 1: CompilerService
- [x] 1.1 创建 CompilerService 骨架（测试 → 实现）
- [x] 1.2 实现 should_cursor_generate_commands（测试 → 实现）
- [x] 1.3 实现 generate_cursor_command_content（测试 → 实现）
- [x] 1.4 实现 compile 核心逻辑（测试 → 实现）
- [x] 1.5 实现 post_compile 支持（测试 → 实现）

## Phase 2: DeployUseCase
- [x] 2.1 使用 CompilerService 静态方法（简化实现）
- [x] 2.2 替换私有 compile_assets 中的重复逻辑
- [x] 2.3 更新 factory.rs（不需要 - 使用静态方法）

## Phase 3: DiffUseCase
- [x] 3.1 使用 CompilerService 静态方法（简化实现）
- [x] 3.2 替换私有 compile_assets 中的重复逻辑

## Phase 4: AssetPipeline
- [x] 4.1 迁移使用 CompilerService
- [x] 4.2 废弃公共 compile_assets（添加 #[deprecated]）

## Phase 5: 集成测试
- [x] 5.1 端到端测试已存在（cli_deploy_targets.rs）

## Phase 6: 清理
- [x] 6.1 移除冗余代码（通过 CompilerService 统一）
- [x] 6.2 更新文档
```

---

## 验收标准

1. ✅ `cargo test` 全部通过
2. ✅ `cargo clippy` 无警告
3. ✅ `calvin deploy --target cursor` 正确生成 `.cursor/commands/`
4. ✅ `calvin diff --target cursor` 正确显示 commands 文件
5. ✅ 无代码重复（只有一个 compile 逻辑）

