# Calvin v0.2.0 重构 TODO

> **目标版本**: v0.2.0  
> **开始日期**: 2025-12-17  
> **状态**: Phase 0-4 已完成，Phase 5 进行中  
> **前提**: 项目尚未发布，可大胆重构

---

## 重构进度概览

```
+==============================================================+
|  Phase 0: 消灭上帝对象                           ✅ 已完成    |
|  Phase 1: 交互式入口                             ✅ 已完成    |
|  Phase 2: explain 命令                           ✅ 已完成    |
|  Phase 3: 错误增强                               ✅ 已完成    |
|  Phase 4: sync 模块重构                          ✅ 已完成    |
|  Phase 5: 文档清理                               🔄 进行中    |
+==============================================================+
```

---

## 重构后代码结构

### 行数统计对比

```
+-----------------------------+-------+-------+---------+
| 文件                         | 之前   | 之后   | 状态    |
+-----------------------------+-------+-------+---------+
| src/main.rs                 | 1151  | 123   | ✅ 达标 |
| src/cli.rs                  | -     | 441   | ✅ 新增 |
| src/state.rs                | -     | 138   | ✅ 新增 |
| src/commands/deploy.rs      | -     | 329   | ✅ 新增 |
| src/commands/check.rs       | -     | 260   | ✅ 新增 |
| src/commands/interactive.rs | -     | 479   | ✅ 新增 |
| src/commands/debug.rs       | -     | 200   | ✅ 新增 |
| src/commands/explain.rs     | -     | 95    | ✅ 新增 |
| src/commands/watch.rs       | -     | 81    | ✅ 新增 |
| src/ui/menu.rs              | -     | 68    | ✅ 新增 |
| src/ui/output.rs            | -     | 27    | ✅ 新增 |
| src/ui/error.rs             | -     | 194   | ✅ 新增 |
| src/sync/mod.rs             | 904   | 296   | ✅ 达标 |
| src/sync/compile.rs         | -     | 97    | ✅ 新增 |
| src/sync/conflict.rs        | -     | 85    | ✅ 新增 |
| src/sync/tests.rs           | -     | 461   | ✅ 新增 |
+-----------------------------+-------+-------+---------+
```

### 新命令结构

```bash
# 无参数运行
calvin                 # 交互式菜单（根据项目状态显示不同菜单）

# 核心命令（显示在 --help）
calvin deploy          # 统一部署（替代 sync + install）
calvin check           # 统一检查（替代 doctor + audit）
calvin explain         # AI 自述（新增）
calvin watch           # 监听模式
calvin diff            # 预览变更
calvin version         # 版本信息

# 隐藏命令（向后兼容，带 deprecation warning）
calvin sync            # -> deploy
calvin install         # -> deploy --home
calvin doctor          # -> check
calvin audit           # -> check --strict-warnings
calvin migrate         # 调试用
calvin parse           # 调试用
```

---

## Phase 0: 消灭上帝对象 ✅ 已完成

> **完成日期**: 2025-12-17

### 0.1 创建 commands/ 模块 ✅

- [x] 创建 `src/cli.rs` - CLI 定义 + 测试
- [x] 创建 `src/commands/mod.rs`
- [x] 创建 `src/commands/deploy.rs` - 合并 sync + install
- [x] 创建 `src/commands/check.rs` - 合并 doctor + audit
- [x] 创建 `src/commands/explain.rs` - AI 自述
- [x] 创建 `src/commands/watch.rs`
- [x] 创建 `src/commands/debug.rs` - diff, parse, migrate, version

### 0.2 创建 ui/ 模块 ✅

- [x] 创建 `src/ui/mod.rs`
- [x] 创建 `src/ui/menu.rs` - select_targets_interactive
- [x] 创建 `src/ui/output.rs` - 配置警告输出

### 0.3 统一命令 ✅

- [x] 实现 `calvin deploy` (project / --home / --remote)
- [x] 实现 `calvin check` (doctor + audit 行为)
- [x] 实现 `calvin explain` (人类可读 + --brief + --json)

### 0.4 向后兼容 ✅

- [x] 保留旧命令作为隐藏别名
- [x] 运行旧命令时打印 deprecation warning
- [x] --json 模式下抑制 warning

### 0.5 验证 ✅

- [x] `cargo test` 全部通过
- [x] `cargo clippy` 无警告
- [x] main.rs 从 1151 行降到 123 行 ✅

---

## Phase 1: 交互式入口 ✅ 已完成

> **完成日期**: 2025-12-17

### 1.1 状态检测 ✅

- [x] 创建 `src/state.rs`
- [x] 实现 `ProjectState` 枚举
- [x] 实现 `detect_state(cwd: &Path) -> ProjectState`
- [x] 测试覆盖（6 个测试）

### 1.2 首次运行菜单 (NoPromptPack) ✅

- [x] 实现主菜单（5 个选项）
- [x] 实现 3 步 setup wizard
- [x] 生成 config.toml 和示例文件
- [x] `write_file_if_missing()` 不覆盖现有文件

### 1.3 已有项目菜单 (Configured) ✅

- [x] 实现操作菜单（8 个选项）

### 1.4 main() 更新 ✅

- [x] CLI 允许无子命令 (`Option<Commands>`)
- [x] 无参数时进入交互模式
- [x] `--json` 模式输出状态 JSON
- [x] 非 TTY 检测（管道模式下不显示菜单）

### 1.5 测试 ✅

- [x] 4 个 interactive.rs 测试

---

## Phase 2: explain 命令 ✅ 已完成

> **完成日期**: 2025-12-17

- [x] 人类可读输出
- [x] `--brief` 简短模式
- [x] `--json` 机器可读模式
- [x] 显示目录结构、frontmatter 格式、命令列表

---

## Phase 3: 错误增强 ✅ 已完成

> **完成日期**: 2025-12-17

### 3.1 错误信息模板 ✅

- [x] 创建 `src/ui/error.rs` (194 行)
- [x] 统一错误格式 `[ERROR] 标题 + 位置 + 描述 + FIX:`
- [x] 更新 NoFrontmatter 错误
- [x] 更新 InvalidFrontmatter 错误
- [x] 更新 MissingField 错误
- [x] 更新 UnclosedFrontmatter 错误

### 3.2 一键修复 ✅

- [x] 检测 `$VISUAL` / `$EDITOR`
- [x] 实现 "Press Enter to open in editor"
- [x] 支持 code/cursor 的 `-g file:line` 格式
- [x] 支持 vim/nvim 的 `+line file` 格式

### 3.3 Did you mean? ⬜ 延期

- [ ] 未知配置键检测（编辑距离算法）
- [ ] 提供最接近的建议
- **原因**: 低优先级，可在后续版本实现

### 3.4 测试 ✅

- [x] `test_format_missing_field_includes_error_header_and_fix`
- [x] `test_format_no_frontmatter_includes_fix`

---

## Phase 4: sync 模块重构 ✅ 已完成

> **完成日期**: 2025-12-17

### 4.1 拆分 sync 模块 ✅

- [x] 创建 `sync/compile.rs` (97 行) - compile_assets()
- [x] 创建 `sync/conflict.rs` (85 行) - ConflictReason, ConflictChoice, SyncPrompter
- [x] 创建 `sync/tests.rs` (461 行) - 单元测试抽离
- [x] 保持 `sync/lockfile.rs` 和 `sync/writer.rs` 不变

### 4.2 验证 ✅

- [x] 所有测试通过 (43 个)
- [x] sync/mod.rs 从 904 行降到 296 行 ✅ (<400 达标)
- [x] `compile_assets` 仍以 `calvin::sync::compile_assets` 对外导出

---

## Phase 5: 文档清理 🔄 进行中

> **优先级**: P2  
> **目标**: 代码和文档同步  
> **预计时间**: 1天

### 5.1 README 更新 ✅

- [x] Quick Start 使用新命令 (deploy, check)

### 5.2 check 命令输出更新 ✅

- [x] `calvin check` 输出提示 "Run `calvin deploy`"

### 5.3 帮助信息 ✅

- [x] --help 只显示核心命令
- [x] 隐藏命令不在帮助中出现

### 5.4 清理

- [x] 更新 TDD 文档归档
- [x] 最终文档审查
- [ ] 提交所有更改

---

## 进度追踪

| Phase | 任务数 | 完成 | 状态 |
|-------|--------|------|------|
| 0. 消灭上帝对象 | 15 | 15 | ✅ 已完成 |
| 1. 交互式入口 | 14 | 14 | ✅ 已完成 |
| 2. explain 命令 | 4 | 4 | ✅ 已完成 |
| 3. 错误增强 | 10 | 8 | ✅ 已完成 (Did you mean 延期) |
| 4. sync 重构 | 5 | 5 | ✅ 已完成 |
| 5. 文档清理 | 6 | 5 | 🔄 进行中 |
| **总计** | **54** | **52** | **96%** |

---

## 验证清单

- [x] `cargo test` 全部通过 (43 个测试)
- [x] `cargo clippy` 无警告
- [x] `calvin --help` 显示新命令结构
- [x] `calvin` 无参数进入交互菜单
- [x] `calvin --json` 输出项目状态 JSON
- [x] `calvin deploy --help` 参数正确
- [x] `calvin explain` 输出完整
- [x] `calvin explain --json` 可被 AI 解析
- [x] `calvin check` 提示 `calvin deploy`
- [x] `calvin sync` 显示 deprecation warning
- [x] main.rs < 150 行 ✅ (123 行)
- [x] sync/mod.rs < 400 行 ✅ (296 行)
- [x] 错误信息包含 FIX 建议 ✅

---

## 相关文档

- [产品重构方案](./product-reflection.md) - 交互设计详情
- [设计原则](./design-principles.md) - 10 条核心原则
- [TDD 会话记录 Phase 0](../tdd-session-v0.2.0-phase0.md)
- [TDD 会话记录 Phase 1](../tdd-session-v0.2.0-phase1.md)
- [TDD 会话记录 Phase 3](../tdd-session-v0.2.0-phase3.md)
- [TDD 会话记录 Phase 4](../tdd-session-v0.2.0-phase4.md)
- [TDD 会话记录 Phase 5](../tdd-session-v0.2.0-phase5.md)
- [TDD 会话记录 (Polish)](../tdd-session-v0.2.0-polish.md)
