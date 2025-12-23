# Clean Command - Product Requirements Document

> **Status**: Draft  
> **Author**: Product Team  
> **Created**: 2025-12-23  

## Executive Summary

用户需要一种安全、可控的方式来清除 Calvin 已部署的 prompt 文件。当前 Calvin 只提供了 `--cleanup` 选项用于删除"孤儿文件"，但缺少完整的清理能力。

## Problem Statement

### 当前痛点

1. **无法完全清除已部署内容**：用户想要移除某个项目部署到 home 目录的所有 prompts，当前没有直接的方法。

2. **禁用 target 后残留文件**：用户在 `config.toml` 中将某个 target 从 `enabled` 列表移除后，之前已部署到该 target 的文件不会被自动清理。

3. **跨项目冲突**：多个项目可能向同一个 home 目录部署内容，用户需要精确控制清理范围。

4. **手动删除风险**：用户不得不手动查找并删除文件，可能误删非 Calvin 管理的配置。

## User Stories

### US-1: 完全清除 Home 目录部署

**As a** 开发者  
**I want to** 清除 Calvin 部署到我 home 目录的所有 prompt 文件  
**So that** 我可以恢复到干净的状态，或准备使用新的 promptpack

**Acceptance Criteria:**

- 能一键清除所有 `home:` scope 的已部署文件
- 显示将要删除的文件列表供确认
- 更新锁文件，移除已删除的条目
- 不影响项目级 (`project:`) 的部署

### US-2: 按 Target 清除

**As a** 开发者  
**I want to** 只清除某个特定平台（如 Cursor）的已部署文件  
**So that** 我可以停止使用某个 AI 工具而不影响其他工具

**Acceptance Criteria:**

- 支持 `--target cursor` 参数指定要清理的平台
- 可以组合多个 targets：`--target cursor --target codex`
- 只删除指定 target 的文件，保留其他 target 的部署

### US-3: 按 Scope 清除

**As a** 开发者  
**I want to** 只清除用户级或项目级的部署  
**So that** 我可以精确控制清理范围

**Acceptance Criteria:**

- 支持 `--home` 只清除 home 目录部署
- 支持 `--project` 只清除项目目录部署
- 两者都不指定时，清除所有 scope

### US-4: 自动清理禁用的 Targets

**As a** 开发者  
**I want to** 当我修改 `config.toml` 禁用某个 target 后，下次 deploy 时自动清理该 target 的已部署文件  
**So that** 配置变更能自动反映到部署结果

**Acceptance Criteria:**

- `calvin deploy` 检测锁文件中存在但当前 `enabled` 列表中没有的 targets
- 提示用户这些 target 已被禁用，询问是否清理
- 用户确认后删除这些 target 的已部署文件
- 支持 `--yes` 自动确认

### US-5: 交互式清理

**As a** 开发者  
**I want to** 通过交互式菜单选择要清理的内容  
**So that** 我不需要记住命令参数，可以安全地操作

**Acceptance Criteria:**

- 直接运行 `calvin clean` 进入交互模式
- 显示当前已部署的统计信息（按 scope 和 target 分组）
- 提供多选菜单让用户勾选要清理的项目
- 确认前显示完整的文件列表

### US-6: Dry Run 预览

**As a** 开发者  
**I want to** 预览将要删除的文件而不实际执行  
**So that** 我可以在执行前确认操作是安全的

**Acceptance Criteria:**

- 支持 `--dry-run` 参数
- 显示所有将被删除的文件路径
- 显示汇总统计（文件数、按 target 分组）
- 不修改任何文件或锁文件

## Command Design

### 命名考量

| 候选名称 | 优点 | 缺点 |
|---------|------|------|
| `uninstall` | 语义明确 | 可能误解为卸载 Calvin CLI 本身 |
| `clean` | 简洁，常见于 build tools | 可能与"清理缓存"混淆 |
| `remove` | 直观 | 与 `rm` 类似，听起来危险 |
| `purge` | 强调彻底删除 | 过于激进，不够友好 |
| `withdraw` | "撤回部署"语义准确 | 不常见，不够直观 |
| `retract` | "收回"语义贴切 | 同上 |

**建议：`clean`**  

- 与 `cargo clean`、`npm clean` 等工具一致
- 配合子命令或参数可以清晰表达意图
- 不会与"卸载 Calvin"混淆

### 命令语法

```bash
# 交互式模式（默认）
calvin clean

# 清除所有 home 目录部署
calvin clean --home

# 清除指定 target
calvin clean --target cursor

# 清除所有部署（home + project）
calvin clean --all

# 组合使用
calvin clean --home --target cursor --target codex

# 预览模式
calvin clean --home --dry-run

# 非交互式确认
calvin clean --home --yes
```

### 输出示例

```
📋 Calvin Clean

Current deployments:
  Home (~/):
    ├─ claude-code: 27 files
    ├─ cursor: 27 files
    ├─ antigravity: 27 files
    └─ codex: 27 files
  
  Project (/path/to/project):
    ├─ claude-code: 27 files
    └─ cursor: 27 files

? What would you like to clean?
  [ ] All home deployments (108 files)
  [ ] claude-code home (27 files)
  [ ] cursor home (27 files)  
  [ ] antigravity home (27 files)
  [ ] codex home (27 files)
  [ ] All project deployments (54 files)
  
> Confirm selection (Space to toggle, Enter to proceed)
```

## Integration with Deploy

### 自动检测禁用的 Targets

当 `calvin deploy` 执行时：

1. 读取锁文件中的已部署 targets（去重）
2. 对比 `config.toml` 中的 `targets.enabled`
3. 如果存在已部署但未启用的 target：

```
⚠️  Detected disabled targets with existing deployments:
    - cursor (27 files in home, 27 files in project)
    - codex (27 files in home)

These targets are no longer in your enabled list.
? Clean up files for disabled targets? [Y/n]
```

4. 用户确认后执行清理
5. 使用 `--yes` 可跳过确认

## Technical Considerations

### 锁文件依赖

- `clean` 命令依赖锁文件 (`.calvin.lock`) 来识别 Calvin 管理的文件
- 锁文件中的 `home:` 前缀用于区分 scope
- 锁文件记录了文件的 target 来源

### 安全机制

1. **签名验证**：只删除包含 Calvin 签名的文件（防止误删用户手动创建的文件）
2. **锁文件匹配**：只删除锁文件中记录的文件
3. **确认提示**：默认需要用户确认
4. **Dry Run**：支持预览模式

### 跨项目场景

- 每个项目有独立的锁文件
- `clean --home` 只清理当前项目在 home 目录的部署
- 不影响其他项目的 home 部署

## Open Questions

1. **是否需要 `--force` 跳过签名验证？**
   - 场景：用户手动修改了 Calvin 部署的文件，想强制清理

2. **是否支持按 asset 名称清理？**
   - 例如：`calvin clean --asset my-workflow.md`

3. **清理后是否自动更新锁文件？**
   - 建议：是，保持锁文件与实际状态一致

4. **是否记录清理日志？**
   - 便于审计和回滚

## Success Metrics

- 用户能在 30 秒内完成 home 目录清理
- 零误删非 Calvin 管理的文件
- 交互模式的用户满意度 > 90%

## Rollout Plan

1. **Phase 1**: 实现基础 `clean` 命令（--home, --target, --dry-run）
2. **Phase 2**: 添加交互式模式
3. **Phase 3**: 集成到 `deploy` 命令的自动检测

---

## Appendix: Current Lockfile Structure

```toml
[files."home:~/.claude/commands/my-workflow.md"]
hash = "sha256:abc123..."
source = "workflows/my-workflow.md"
target = "claude-code"     # 新增：记录 target
scope = "user"             # 新增：记录 scope

[files."project:.claude/commands/my-workflow.md"]  
hash = "sha256:def456..."
source = "workflows/my-workflow.md"
target = "claude-code"
scope = "project"
```

注意：当前锁文件可能未包含 `target` 和 `scope` 字段，需要评估是否需要迁移。
