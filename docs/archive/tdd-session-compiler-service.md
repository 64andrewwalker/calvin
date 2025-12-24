# TDD Session: CompilerService

> 日期：2025-12-24
> 分支：fix/cursor-commands-missing
> 状态：✅ 完成

---

## 目标

创建 `CompilerService` 领域服务，统一编译逻辑，修复 Cursor-only 部署时 commands 丢失的 bug。

---

## 问题回顾

当用户使用 `calvin deploy --target cursor`（单独部署 Cursor）时，Action/Agent 资产不会生成 `.cursor/commands/` 文件。

### 根因

存在三个独立的 `compile_assets` 实现，只有公共函数包含 `cursor_needs_commands` 逻辑：

| 位置 | 有 cursor_needs_commands？ |
|------|---------------------------|
| `src/application/compiler.rs` (公共函数) | ✅ 有 |
| `src/application/deploy/use_case.rs` (私有方法) | ❌ 没有 |
| `src/application/diff.rs` (私有方法) | ❌ 没有 |

---

## TDD 实施记录

### Phase 1: 创建 CompilerService

#### TODO 1.1-1.5: CompilerService 完整实现

**RED Phase** - 编写失败测试：

```rust
#[test]
fn compiler_service_new_creates_instance() {
    let adapters: Vec<Box<dyn TargetAdapter>> = vec![];
    let service = CompilerService::new(adapters);
    assert!(service.adapters().is_empty());
}

#[test]
fn cursor_needs_commands_when_cursor_only() {
    let service = create_test_service();
    assert!(service.should_cursor_generate_commands(&[Target::Cursor]));
}

#[test]
fn compile_cursor_only_generates_commands_for_action() {
    // ... 使用真实 CursorAdapter 测试 ...
}
```

**GREEN Phase** - 实现 CompilerService：

创建 `src/domain/services/compiler_service.rs`，包含：
- `CompilerService` 结构体
- `cursor_needs_commands()` 静态方法
- `generate_command_content()` 静态方法
- `compile()` 方法

**测试结果**：17 个测试全部通过 ✅

### Phase 2-3: 迁移 UseCase

#### 决策

不修改 UseCase 构造函数签名，而是：
- 添加静态方法 `CompilerService::cursor_needs_commands()` 和 `CompilerService::generate_command_content()`
- 在 `DeployUseCase::compile_assets` 和 `DiffUseCase::compile_assets` 中直接调用这些静态方法

**修改的文件**：
- `src/application/deploy/use_case.rs` - 添加 cursor_needs_commands 逻辑
- `src/application/diff.rs` - 添加 cursor_needs_commands 逻辑

**测试结果**：所有现有测试通过 ✅

### Phase 4: 集成测试

添加两个新的集成测试：

```rust
#[test]
fn deploy_cursor_only_generates_commands() {
    // 验证 Cursor-only 部署生成 .cursor/commands/
}

#[test]
fn deploy_cursor_with_claude_code_no_duplicate_commands() {
    // 验证 Cursor + Claude Code 不会重复生成
}
```

**测试结果**：新测试通过 ✅

---

## 最终验证

```bash
$ cargo test
# 774+ tests passed

$ cargo clippy
# No warnings

$ cargo fmt
# Code formatted
```

---

## 创建的文件

| 文件 | 描述 |
|------|------|
| `src/domain/services/compiler_service.rs` | CompilerService 实现 |
| `docs/reports/cursor-commands-bug-2025-12-24.md` | Bug 分析报告 |
| `docs/proposals/compiler-service-refactor-plan.md` | 重构计划 |

## 修改的文件

| 文件 | 描述 |
|------|------|
| `src/domain/services/mod.rs` | 注册 CompilerService |
| `src/application/deploy/use_case.rs` | 添加 cursor_needs_commands 逻辑 |
| `src/application/diff.rs` | 添加 cursor_needs_commands 逻辑 |
| `tests/cli_deploy_targets.rs` | 添加集成测试 |

---

## 经验总结

1. **代码重复是 bug 的温床**：三份 `compile_assets` 实现导致不一致
2. **TDD 确保正确性**：先写测试，再写实现，避免遗漏边界情况
3. **静态方法复用**：通过静态方法在不修改签名的情况下共享逻辑
4. **集成测试验证端到端**：单元测试不够，需要真实场景验证
