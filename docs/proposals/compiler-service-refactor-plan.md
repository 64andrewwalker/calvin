# CompilerService 重构计划

> 创建日期：2025-12-24
> 分支：fix/cursor-commands-missing
> 状态：Planning

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

## 架构设计

### 目标结构

```
src/
├── domain/
│   └── services/
│       ├── mod.rs                    (已存在)
│       └── compiler_service.rs       (新增)
├── application/
│   ├── compiler.rs                   (废弃，标记 deprecated)
│   ├── deploy/
│   │   └── use_case.rs               (使用 CompilerService)
│   └── diff.rs                       (使用 CompilerService)
└── presentation/
    └── factory.rs                    (创建 CompilerService)
```

### CompilerService 接口

```rust
/// 编译服务 - 负责将 Assets 编译为 OutputFiles
/// 
/// 包含所有平台特定的编译逻辑，如：
/// - Cursor-only 部署时生成 commands
/// - 未来可能的其他平台特殊逻辑
pub struct CompilerService {
    adapters: Vec<Box<dyn TargetAdapter>>,
}

impl CompilerService {
    /// 创建编译服务
    pub fn new(adapters: Vec<Box<dyn TargetAdapter>>) -> Self;
    
    /// 编译资产
    /// 
    /// 包含 cursor_needs_commands 逻辑：
    /// - 如果目标列表包含 Cursor 但不包含 Claude Code
    /// - 则为 Action/Agent 类型生成 .cursor/commands/ 文件
    pub fn compile(
        &self,
        assets: &[Asset],
        targets: &[Target],
    ) -> Result<Vec<OutputFile>, CompileError>;
    
    /// 检查 Cursor 是否需要生成 commands
    fn should_cursor_generate_commands(&self, targets: &[Target]) -> bool;
    
    /// 生成 Cursor command 内容
    fn generate_cursor_command_content(&self, asset: &Asset, footer: &str) -> String;
}
```

### 依赖注入

```rust
// DeployUseCase 将接收 CompilerService
pub struct DeployUseCase<AR, LR, FS>
where
    AR: AssetRepository,
    LR: LockfileRepository,
    FS: FileSystem,
{
    asset_repo: AR,
    lockfile_repo: LR,
    file_system: FS,
    compiler: CompilerService,  // 新增
}
```

---

## TDD 实施计划

### Phase 1: 创建 CompilerService

#### TODO 1.1: 创建 CompilerService 骨架

**测试先行：**

```rust
// src/domain/services/compiler_service.rs

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn compiler_service_new_creates_instance() {
        let adapters: Vec<Box<dyn TargetAdapter>> = vec![];
        let service = CompilerService::new(adapters);
        assert!(service.adapters.is_empty());
    }
}
```

**实现：**

```rust
pub struct CompilerService {
    adapters: Vec<Box<dyn TargetAdapter>>,
}

impl CompilerService {
    pub fn new(adapters: Vec<Box<dyn TargetAdapter>>) -> Self {
        Self { adapters }
    }
}
```

#### TODO 1.2: 实现 should_cursor_generate_commands

**测试先行：**

```rust
#[test]
fn cursor_needs_commands_when_cursor_only() {
    let service = create_test_service();
    
    // Cursor only - should generate commands
    assert!(service.should_cursor_generate_commands(&[Target::Cursor]));
}

#[test]
fn cursor_no_commands_when_with_claude_code() {
    let service = create_test_service();
    
    // Cursor + Claude Code - Claude provides commands
    assert!(!service.should_cursor_generate_commands(&[Target::Cursor, Target::ClaudeCode]));
}

#[test]
fn cursor_no_commands_when_all_targets() {
    let service = create_test_service();
    
    // Empty targets means all - Claude Code included
    assert!(!service.should_cursor_generate_commands(&[]));
}
```

**实现：**

```rust
fn should_cursor_generate_commands(&self, targets: &[Target]) -> bool {
    let has_claude_code = targets.is_empty() || targets.contains(&Target::ClaudeCode);
    let has_cursor = targets.is_empty() || targets.contains(&Target::Cursor);
    has_cursor && !has_claude_code
}
```

#### TODO 1.3: 实现 generate_cursor_command_content

**测试先行：**

```rust
#[test]
fn generate_content_with_description() {
    let service = create_test_service();
    let asset = create_action_asset("test", "My description", "# Content");
    
    let content = service.generate_cursor_command_content(&asset, "<!-- footer -->");
    
    assert!(content.starts_with("My description"));
    assert!(content.contains("# Content"));
    assert!(content.ends_with("<!-- footer -->"));
}

#[test]
fn generate_content_without_description() {
    let service = create_test_service();
    let asset = create_action_asset("test", "", "# Content");
    
    let content = service.generate_cursor_command_content(&asset, "<!-- footer -->");
    
    assert!(content.starts_with("# Content"));
}
```

#### TODO 1.4: 实现 compile 方法核心逻辑

**测试先行：**

```rust
#[test]
fn compile_cursor_only_generates_commands_for_action() {
    let adapters = vec![
        Box::new(CursorAdapter::new()) as Box<dyn TargetAdapter>,
    ];
    let service = CompilerService::new(adapters);
    
    let asset = create_action_asset("test-action", "desc", "content");
    
    let outputs = service.compile(&[asset], &[Target::Cursor]).unwrap();
    
    // Should have both rule and command
    let has_command = outputs.iter().any(|o| 
        o.path().to_string_lossy().contains("commands")
    );
    assert!(has_command, "Cursor-only should generate commands");
}

#[test]
fn compile_with_claude_code_no_cursor_commands() {
    let adapters = vec![
        Box::new(CursorAdapter::new()) as Box<dyn TargetAdapter>,
        Box::new(ClaudeCodeAdapter::new()) as Box<dyn TargetAdapter>,
    ];
    let service = CompilerService::new(adapters);
    
    let asset = create_action_asset("test-action", "desc", "content");
    
    let outputs = service.compile(&[asset], &[Target::Cursor, Target::ClaudeCode]).unwrap();
    
    // Cursor should NOT generate commands (Claude Code provides them)
    let cursor_command = outputs.iter().any(|o| 
        o.path().to_string_lossy().contains(".cursor/commands")
    );
    assert!(!cursor_command, "Cursor should not generate commands when Claude Code is present");
}
```

#### TODO 1.5: 实现 compile 方法 post_compile 支持

**测试先行：**

```rust
#[test]
fn compile_calls_post_compile_for_each_adapter() {
    let adapters = vec![
        Box::new(VSCodeAdapter::new()) as Box<dyn TargetAdapter>,
    ];
    let service = CompilerService::new(adapters);
    
    let asset = create_action_asset("test", "desc", "content");
    
    let outputs = service.compile(&[asset], &[Target::VSCode]).unwrap();
    
    // VSCode generates AGENTS.md in post_compile
    let has_agents = outputs.iter().any(|o| 
        o.path().file_name().map(|f| f == "AGENTS.md").unwrap_or(false)
    );
    assert!(has_agents, "Should call post_compile");
}
```

### Phase 2: 迁移 DeployUseCase

#### TODO 2.1: 修改 DeployUseCase 构造函数

**测试先行：**

```rust
#[test]
fn deploy_use_case_accepts_compiler_service() {
    let asset_repo = FsAssetRepository::new();
    let lockfile_repo = TomlLockfileRepository::new();
    let fs = LocalFs::new();
    let adapters = all_adapters();
    let compiler = CompilerService::new(adapters.clone());
    
    let _use_case = DeployUseCase::new(asset_repo, lockfile_repo, fs, compiler);
    // Should compile without error
}
```

#### TODO 2.2: 替换 DeployUseCase 私有 compile_assets

移除 `DeployUseCase::compile_assets` 私有方法，改用 `self.compiler.compile()`。

**测试先行：**

```rust
#[test]
fn deploy_cursor_only_generates_commands() {
    // Integration test
    let dir = tempdir().unwrap();
    let source = dir.path().join(".promptpack");
    fs::create_dir_all(&source).unwrap();
    
    // Create an action asset
    fs::write(source.join("actions/test.md"), r#"---
description: Test action
kind: action
---
# Test Content
"#).unwrap();
    
    let options = DeployOptions::new(&source)
        .with_targets(vec![Target::Cursor])
        .with_dry_run(true);
    
    let use_case = create_deploy_use_case_for_targets(&[Target::Cursor]);
    let result = use_case.execute(&options);
    
    // Should have .cursor/commands output
    let has_cursor_command = result.written.iter().any(|p| 
        p.to_string_lossy().contains(".cursor/commands")
    );
    assert!(has_cursor_command, "Cursor-only deploy should generate commands");
}
```

#### TODO 2.3: 更新 factory.rs

更新 `create_deploy_use_case` 系列函数，注入 `CompilerService`。

### Phase 3: 迁移 DiffUseCase

#### TODO 3.1: 修改 DiffUseCase 构造函数

与 DeployUseCase 类似，添加 `CompilerService` 依赖。

#### TODO 3.2: 替换 DiffUseCase 私有 compile_assets

移除 `DiffUseCase::compile_assets` 私有方法。

**测试先行：**

```rust
#[test]
fn diff_cursor_only_shows_commands() {
    // Similar to deploy test
}
```

### Phase 4: 更新 AssetPipeline

#### TODO 4.1: 迁移 AssetPipeline 使用 CompilerService

`AssetPipeline::compile` 当前调用公共 `compile_assets` 函数，需要改为使用 `CompilerService`。

#### TODO 4.2: 废弃公共 compile_assets 函数

标记 `src/application/compiler.rs::compile_assets` 为 deprecated，但保持兼容。

### Phase 5: 集成测试

#### TODO 5.1: 添加端到端测试

```rust
// tests/cli_deploy_targets.rs

#[test]
fn deploy_cursor_only_to_home_creates_commands() {
    // End-to-end test: calvin deploy --home --target cursor
}

#[test]
fn diff_cursor_only_shows_commands_output() {
    // End-to-end test: calvin diff --target cursor
}
```

### Phase 6: 清理

#### TODO 6.1: 移除冗余代码

- 删除 `DeployUseCase::compile_assets` 私有方法
- 删除 `DiffUseCase::compile_assets` 私有方法

#### TODO 6.2: 更新文档

- 更新 `docs/architecture/layers.md` 说明 CompilerService
- 更新 `docs/target-platforms.md` 说明 Cursor commands 生成条件

---

## TODO Checklist

```markdown
## Phase 1: CompilerService
- [ ] 1.1 创建 CompilerService 骨架（测试 → 实现）
- [ ] 1.2 实现 should_cursor_generate_commands（测试 → 实现）
- [ ] 1.3 实现 generate_cursor_command_content（测试 → 实现）
- [ ] 1.4 实现 compile 核心逻辑（测试 → 实现）
- [ ] 1.5 实现 post_compile 支持（测试 → 实现）

## Phase 2: DeployUseCase
- [ ] 2.1 修改构造函数接受 CompilerService
- [ ] 2.2 替换私有 compile_assets
- [ ] 2.3 更新 factory.rs

## Phase 3: DiffUseCase
- [ ] 3.1 修改构造函数接受 CompilerService
- [ ] 3.2 替换私有 compile_assets

## Phase 4: AssetPipeline
- [ ] 4.1 迁移使用 CompilerService
- [ ] 4.2 废弃公共 compile_assets

## Phase 5: 集成测试
- [ ] 5.1 添加端到端测试

## Phase 6: 清理
- [ ] 6.1 移除冗余代码
- [ ] 6.2 更新文档
```

---

## 风险与注意事项

1. **向后兼容**：`compile_assets` 公共函数可能被外部使用，标记 deprecated 但保留

2. **类型系统**：确保 `CompilerService` 使用 domain types，不依赖 legacy models

3. **测试覆盖**：每个 TODO 都需要测试先行

4. **渐进式迁移**：可以分 PR 完成各 Phase

---

## 验收标准

1. `cargo test` 全部通过
2. `cargo clippy` 无警告
3. `calvin deploy --target cursor` 正确生成 `.cursor/commands/`
4. `calvin diff --target cursor` 正确显示 commands 文件
5. 无代码重复（只有一个 compile 逻辑）

