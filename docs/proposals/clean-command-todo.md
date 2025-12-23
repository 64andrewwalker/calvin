# Clean Command - TODO

> **Status**: Not Started  
> **PRD**: [clean-command-prd.md](clean-command-prd.md)  
> **Implementation Plan**: [clean-command-implementation-plan.md](clean-command-implementation-plan.md)

---

## Phase 1: 基础 Clean 命令

### 1.1 CLI 定义

- [ ] **RED**: 编写 `tests/cli_clean.rs` 基础测试
  - [ ] `clean_help_shows_options`
  - [ ] `clean_requires_lockfile`
  - [ ] `clean_dry_run_no_delete`
- [ ] **GREEN**: 添加 `Commands::Clean` 到 `src/presentation/cli.rs`
  - [ ] `--home` 选项
  - [ ] `--project` 选项
  - [ ] `--dry-run` 选项
  - [ ] `--yes` / `-y` 选项
  - [ ] `--force` 选项
- [ ] **REFACTOR**: 确保帮助文本清晰

### 1.2 CleanUseCase

- [ ] **RED**: 编写 `CleanUseCase` 单元测试
  - [ ] `clean_deletes_files_from_lockfile`
  - [ ] `clean_skips_modified_files`
  - [ ] `clean_dry_run_does_not_delete`
  - [ ] `clean_updates_lockfile_after_delete`
- [ ] **GREEN**: 创建 `src/application/clean/` 模块
  - [ ] `mod.rs` - 模块导出
  - [ ] `options.rs` - `CleanOptions` 结构
  - [ ] `result.rs` - `CleanResult`、`SkipReason`
  - [ ] `use_case.rs` - `CleanUseCase::execute()`
- [ ] **REFACTOR**: 优化错误处理和日志

### 1.3 Lockfile 扩展

- [ ] **RED**: 编写 lockfile 过滤测试
  - [ ] `lockfile_entries_by_scope_user`
  - [ ] `lockfile_entries_by_scope_project`
  - [ ] `lockfile_remove_entry`
- [ ] **GREEN**: 扩展 `src/domain/entities/lockfile.rs`
  - [ ] `entries_by_scope(scope: Scope) -> Vec<LockfileEntry>`
  - [ ] `remove_entry(key: &str) -> Result<()>`
- [ ] **REFACTOR**: 确保 API 一致性

### 1.4 签名验证

- [ ] **RED**: 编写签名检测测试
  - [ ] `detects_calvin_signature`
  - [ ] `rejects_file_without_signature`
  - [ ] `handles_binary_files`
- [ ] **GREEN**: 创建 `src/domain/services/signature.rs`
  - [ ] `has_calvin_signature(content: &str) -> bool`
  - [ ] `SIGNATURE_PATTERN` 常量
- [ ] **REFACTOR**: 考虑多种签名格式

### 1.5 命令处理器

- [ ] **RED**: 编写命令集成测试
  - [ ] `clean_home_removes_files`
  - [ ] `clean_project_removes_files`
  - [ ] `clean_respects_yes_flag`
- [ ] **GREEN**: 创建 `src/commands/clean.rs`
  - [ ] `cmd_clean()` 函数
  - [ ] 集成 `CleanUseCase`
  - [ ] 调用确认提示 (非 `--yes` 时)
- [ ] **GREEN**: 更新 `src/main.rs`
  - [ ] 添加 `Commands::Clean` 分支
- [ ] **REFACTOR**: 统一输出格式

### 1.6 UI 视图

- [ ] 创建 `src/ui/views/clean.rs`
  - [ ] `render_clean_header()` - 头部信息
  - [ ] `render_clean_preview()` - 预览列表
  - [ ] `render_clean_result()` - 结果摘要
- [ ] 添加 JSON 输出支持
  - [ ] `clean_start` 事件
  - [ ] `file_deleted` 事件
  - [ ] `file_skipped` 事件
  - [ ] `clean_complete` 事件

---

## Phase 2: 交互式树形菜单

### 2.1 TreeNode 数据结构

- [ ] **RED**: 编写 TreeNode 测试
  - [ ] `tree_node_from_lockfile`
  - [ ] `selecting_parent_selects_all_children`
  - [ ] `deselecting_parent_deselects_all_children`
  - [ ] `partial_child_selection_shows_partial_parent`
- [ ] **GREEN**: 创建 `src/ui/widgets/tree_menu.rs`
  - [ ] `TreeNode` 结构
  - [ ] `SelectionState` 枚举
  - [ ] `from_lockfile()` 构造器
  - [ ] `select()` / `deselect()` / `toggle()`
  - [ ] `propagate_selection_up()` - 更新父级状态
  - [ ] `propagate_selection_down()` - 更新子级状态
- [ ] **REFACTOR**: 优化树遍历性能

### 2.2 TreeMenu 交互

- [ ] **GREEN**: 实现键盘导航
  - [ ] `↑` / `↓` 移动光标
  - [ ] `Space` 切换选中
  - [ ] `→` / `Enter` 展开节点
  - [ ] `←` / `Backspace` 折叠节点
  - [ ] `a` 全选
  - [ ] `n` 全不选
  - [ ] `i` 反选
  - [ ] `q` 退出
  - [ ] `Enter` (在 Confirm 上) 确认

### 2.3 TreeMenu 渲染

- [ ] **GREEN**: 实现渲染逻辑
  - [ ] 使用 CalvinTheme 图标 (●, ○, ◐, ▼, ▶)
  - [ ] 缩进和树形连接线
  - [ ] 文件计数显示
  - [ ] 状态栏 (选中数量)
  - [ ] 快捷键提示
- [ ] **REFACTOR**: 优化视觉效果

### 2.4 集成到 Clean 命令

- [ ] 在 `cmd_clean()` 中检测交互模式
- [ ] 无参数时启动 TreeMenu
- [ ] TreeMenu 选择结果传递给 CleanUseCase

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

### 必须在 Phase 1 实现

- [ ] **EC-2**: 锁文件丢失 → 显示错误，提示 `--scan`
- [ ] **EC-3**: 文件被修改 → 跳过，显示警告
- [ ] **EC-4**: 文件已不存在 → 静默跳过，更新锁文件
- [ ] **EC-5**: 权限不足 → 继续处理其他文件，汇总报告
- [ ] **EC-12**: 无签名 → 跳过 (`--force` 强制删除)

### 可延迟到 Phase 2+

- [ ] **EC-7**: Remote 部署 → 跳过，提示不支持
- [ ] **EC-8**: Watch 运行时 → 检测并警告
- [ ] **EC-18**: `--scan` 选项实现

---

## 文档更新

- [ ] 更新 `docs/command-reference.md`
- [ ] 更新 `GEMINI.md` 如有需要
- [ ] 添加 `docs/tdd-session-clean.md` 记录 TDD 过程

---

## 验收标准

### Phase 1 完成条件

- [ ] `calvin clean --help` 显示正确选项
- [ ] `calvin clean --dry-run` 预览不删除
- [ ] `calvin clean --home --yes` 删除 home 部署
- [ ] `calvin clean --project --yes` 删除 project 部署
- [ ] 锁文件同步更新
- [ ] 所有单元测试通过
- [ ] 所有集成测试通过

### Phase 2 完成条件

- [ ] `calvin clean` 进入交互式菜单
- [ ] 树形菜单正确显示层级
- [ ] 选择逻辑正确 (父子联动)
- [ ] 快捷键全部工作

---

## 预估工时

| Phase | 预估时间 | 说明 |
|-------|---------|------|
| Phase 1 | 4-6 小时 | 核心功能，TDD 完整循环 |
| Phase 2 | 3-4 小时 | 交互式 UI，复用 CalvinTheme |
| Phase 3 | 2-3 小时 | 可选，仅在需要时实现 |

**总计**: 约 9-13 小时

---

## 开始 TDD 会话

准备好后，从以下测试开始：

```bash
# 创建测试文件
touch tests/cli_clean.rs

# 编写第一个失败测试
# 然后运行
cargo test cli_clean
```
