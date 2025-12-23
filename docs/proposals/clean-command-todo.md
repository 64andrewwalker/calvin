# Clean Command - TODO

> **Status**: Phase 2 Complete  
> **PRD**: [clean-command-prd.md](clean-command-prd.md)  
> **Implementation Plan**: [clean-command-implementation-plan.md](clean-command-implementation-plan.md)

---

## Phase 1: 基础 Clean 命令 ✅ COMPLETE

### 1.1 CLI 定义 ✅

- [x] **RED**: 编写 `tests/cli_clean.rs` 基础测试
  - [x] `clean_help_shows_options`
  - [x] `clean_requires_lockfile`
  - [x] `clean_dry_run_no_delete`
- [x] **GREEN**: 添加 `Commands::Clean` 到 `src/presentation/cli.rs`
  - [x] `--home` 选项
  - [x] `--project` 选项
  - [x] `--dry-run` 选项
  - [x] `--yes` / `-y` 选项
  - [x] `--force` 选项
- [x] **REFACTOR**: 确保帮助文本清晰

### 1.2 CleanUseCase ✅

- [x] **RED**: 编写 `CleanUseCase` 单元测试
  - [x] `clean_deletes_files_from_lockfile`
  - [x] `clean_skips_modified_files`
  - [x] `clean_dry_run_does_not_delete`
  - [x] `clean_updates_lockfile_after_delete`
- [x] **GREEN**: 创建 `src/application/clean/` 模块
  - [x] `mod.rs` - 模块导出
  - [x] `options.rs` - `CleanOptions` 结构
  - [x] `result.rs` - `CleanResult`、`SkipReason`
  - [x] `use_case.rs` - `CleanUseCase::execute()`
- [x] **REFACTOR**: 优化错误处理和日志

### 1.3 Lockfile 扩展 ✅

- [x] **RED**: 编写 lockfile 过滤测试
  - [x] `lockfile_entries_by_scope_user`
  - [x] `lockfile_entries_by_scope_project`
  - [x] `lockfile_remove_entry`
- [x] **GREEN**: 扩展 `src/domain/entities/lockfile.rs`
  - [x] `keys_for_scope(scope: Scope) -> impl Iterator` (已存在)
  - [x] `remove(key: &str)` (已存在)
- [x] **REFACTOR**: 确保 API 一致性

### 1.4 签名验证 ✅

- [x] **RED**: 编写签名检测测试
  - [x] `detects_calvin_signature` (在 `orphan_detector.rs`)
  - [x] `rejects_file_without_signature`
  - [x] `handles_binary_files` (隐式处理，不含签名则跳过)
- [x] **GREEN**: 复用 `src/domain/services/orphan_detector.rs`
  - [x] `has_calvin_signature(content: &str) -> bool`
  - [x] `CALVIN_SIGNATURES` 常量
- [x] **REFACTOR**: 考虑多种签名格式 (已支持3种格式)

### 1.5 命令处理器 ✅

- [x] **RED**: 编写命令集成测试
  - [x] `clean_home_removes_files` (在 cli_clean.rs)
  - [x] `clean_project_removes_files`
  - [x] `clean_respects_yes_flag`
- [x] **GREEN**: 创建 `src/commands/clean.rs`
  - [x] `cmd_clean()` 函数
  - [x] 集成 `CleanUseCase`
  - [x] 调用确认提示 (非 `--yes` 时)
- [x] **GREEN**: 更新 `src/main.rs`
  - [x] 添加 `Commands::Clean` 分支
- [x] **REFACTOR**: 统一输出格式

### 1.6 UI 视图 ✅

- [x] 创建 `src/ui/views/clean.rs`
  - [x] `render_clean_header()` - 头部信息
  - [x] `render_clean_preview()` - 预览列表
  - [x] `render_clean_result()` - 结果摘要
- [x] 添加 JSON 输出支持
  - [x] `clean_start` 事件
  - [x] `file_deleted` 事件
  - [x] `file_skipped` 事件
  - [x] `clean_complete` 事件

---

## Phase 2: 交互式树形菜单 ✅ COMPLETE

### 2.1 TreeNode 数据结构 ✅

- [x] **RED**: 编写 TreeNode 测试
  - [x] `tree_node_from_lockfile`
  - [x] `selecting_parent_selects_all_children`
  - [x] `deselecting_parent_deselects_all_children`
  - [x] `partial_child_selection_shows_partial_parent`
- [x] **GREEN**: 创建 `src/ui/widgets/tree_menu.rs`
  - [x] `TreeNode` 结构
  - [x] `SelectionState` 枚举
  - [x] `from_lockfile()` 构造器 (`build_tree_from_lockfile()`)
  - [x] `select()` / `deselect()` / `toggle()`
  - [x] `propagate_selection_up()` - 更新父级状态 (`update_state_from_children`)
  - [x] `propagate_selection_down()` - 更新子级状态 (`select_all`, `select_none`)
- [x] **REFACTOR**: 优化树遍历性能

### 2.2 TreeMenu 交互 ✅

- [x] **GREEN**: 实现键盘导航
  - [x] `↑` / `↓` 移动光标 (`TreeAction::Up/Down`)
  - [x] `Space` 切换选中 (`TreeAction::Toggle`)
  - [x] `→` / `Enter` 展开节点 (`TreeAction::Expand`)
  - [x] `←` / `Backspace` 折叠节点 (`TreeAction::Collapse`)
  - [x] `a` 全选 (`TreeAction::SelectAll`)
  - [x] `n` 全不选 (`TreeAction::SelectNone`)
  - [x] `i` 反选 (`TreeAction::Invert`)
  - [x] `q` 退出 (`TreeAction::Quit`)
  - [x] `Enter` (在 Confirm 上) 确认 (`TreeAction::Confirm`)

### 2.3 TreeMenu 渲染 ✅

- [x] **GREEN**: 实现渲染逻辑
  - [x] 使用 CalvinTheme 图标 (●, ○, ◐, ▼, ▶)
  - [x] 缩进和树形连接线
  - [x] 文件计数显示
  - [x] 状态栏 (选中数量)
  - [x] 快捷键提示
- [x] **REFACTOR**: 优化视觉效果

### 2.4 集成到 Clean 命令 ✅

- [x] 在 `cmd_clean()` 中检测交互模式
- [x] 无参数时启动 TreeMenu
- [x] TreeMenu 选择结果传递给 CleanUseCase

---

## Phase 3: Lockfile 扩展 (可选)

### 3.1 添加 target 字段

- [ ] 扩展 lockfile 结构
  - [ ] `target: String` 字段
  - [ ] `source: String` 字段
- [ ] 更新 deploy 时写入新字段
- [ ] 实现 `entries_by_target(target: Target)`

### 3.2 --target 过滤

- [ ] 添加 `--target <NAME>` CLI 选项
- [ ] 支持多个 `--target` 参数
- [ ] 在 CleanOptions 中添加 targets 字段

### 3.3 迁移旧格式

- [ ] 检测 lockfile 格式版本
- [ ] 从路径推断 target (如 `.claude/` → `claude-code`)
- [ ] 自动迁移并保存

---

## Edge Case 处理

### 必须在 Phase 1 实现 ✅

- [x] **EC-2**: 锁文件丢失 → 显示错误，提示需要 deploy
- [x] **EC-3**: 文件被修改 → 跳过，显示警告 (SkipReason::Modified)
- [x] **EC-4**: 文件已不存在 → 静默跳过，更新锁文件 (SkipReason::Missing)
- [x] **EC-5**: 权限不足 → 继续处理其他文件，汇总报告 (SkipReason::PermissionDenied)
- [x] **EC-12**: 无签名 → 跳过 (`--force` 强制删除) (SkipReason::NoSignature)

### 可延迟到 Phase 2+

- [ ] **EC-7**: Remote 部署 → 跳过，提示不支持
- [ ] **EC-8**: Watch 运行时 → 检测并警告
- [ ] **EC-18**: `--scan` 选项实现

---

## 文档更新

- [x] 更新 `docs/command-reference.md`
- [ ] 更新 `GEMINI.md` 如有需要
- [ ] 添加 `docs/tdd-session-clean.md` 记录 TDD 过程

---

## 验收标准

### Phase 1 完成条件 ✅

- [x] `calvin clean --help` 显示正确选项
- [x] `calvin clean --dry-run` 预览不删除
- [x] `calvin clean --home --yes` 删除 home 部署
- [x] `calvin clean --project --yes` 删除 project 部署
- [x] 锁文件同步更新
- [x] 所有单元测试通过
- [x] 所有集成测试通过

### Phase 2 完成条件 ✅

- [x] TreeNode 数据结构实现
- [x] TreeMenu 键盘导航实现
- [x] TreeMenu 渲染实现
- [x] `calvin clean` 进入交互式菜单
- [x] 树形菜单正确显示层级
- [x] 选择逻辑正确 (父子联动)
- [x] 快捷键全部工作

---

## 预估工时

| Phase | 预估时间 | 说明 | 状态 |
|-------|---------|------|------|
| Phase 1 | 4-6 小时 | 核心功能，TDD 完整循环 | ✅ 完成 |
| Phase 2 | 3-4 小时 | 交互式 UI，复用 CalvinTheme | ✅ 完成 |
| Phase 3 | 2-3 小时 | 可选，仅在需要时实现 | ⏸️ 待定 |

**总计**: 约 9-13 小时

---

## 下一步

Phase 1 和 Phase 2 已完成。

可选的 Phase 3 任务（按需实现）：
1. 扩展 lockfile 添加 target 字段
2. 实现 `--target` 过滤选项
3. 迁移旧格式 lockfile

```bash
# 运行测试确保一切正常
cargo test clean
cargo test tree_menu
```
