# Clean Command PRD Checklist

> **Created**: 2024-12-23
> **Purpose**: 对照 PRD 检查实现完整性

## User Stories 实现状态

### US-1: 完全清除 Home 目录部署 ✅

| 需求 | 状态 | 实现位置 |
|------|------|----------|
| 一键清除所有 `home:` scope 的文件 | ✅ | `calvin clean --home --yes` |
| 显示将要删除的文件列表供确认 | ✅ | `CleanUseCase.execute()` 预览模式 |
| 更新锁文件，移除已删除条目 | ✅ | `CleanUseCase.execute_confirmed()` |
| 不影响 `project:` 的部署 | ✅ | `keys_for_scope()` 过滤 |

### US-2: 按 Target 清除 ⚠️ Phase 3

| 需求 | 状态 | 说明 |
|------|------|------|
| `--target cursor` 参数 | ❌ | PRD 定义为 Phase 3，需要 lockfile 扩展 |
| 组合多个 targets | ❌ | 同上 |

**PRD 说明**: Phase 3 需要先扩展 lockfile 添加 `target` 字段。

### US-3: 按 Scope 清除 ✅

| 需求 | 状态 | 实现位置 |
|------|------|----------|
| `--home` 只清除 home 目录 | ✅ | `cli.rs` 选项，`CleanOptions.scope` |
| `--project` 只清除项目目录 | ✅ | `cli.rs` 选项，`CleanOptions.scope` |
| 两者都不指定时清除所有 | ⚠️ | 当前进入交互模式 |

**注意**: PRD 说 "两者都不指定时，清除所有 scope"，但当前实现是进入交互模式。这需要确认是否符合预期。

### US-4: 自动清理禁用的 Targets ❌ 未实现

| 需求 | 状态 | 说明 |
|------|------|------|
| deploy 检测禁用的 targets | ❌ | PRD 定义为不做（命令独立原则） |
| 提示用户清理 | ❌ | 同上 |

**PRD 说明**: 根据"命令独立原则"，deploy 不应自动检测清理需求。这是正确的。

### US-5: 交互式清理 ✅

| 需求 | 状态 | 实现位置 |
|------|------|----------|
| `calvin clean` 进入交互模式 | ✅ | `cmd_clean()` + `run_interactive_clean()` |
| 显示已部署统计信息 | ✅ | `TreeMenu.render_status_bar()` |
| 多选菜单勾选项目 | ✅ | `TreeMenu` 组件 |
| 确认前显示文件列表 | ✅ | 树形展开显示 |

**交互菜单功能**:
- ✅ 多级树形结构 (Scope → Target → Files)
- ✅ 展开/折叠
- ✅ 部分选中状态 (◐)
- ✅ 快捷键 (a/n/i)
- ✅ Enter 确认 / q 退出

### US-6: Dry Run 预览 ✅

| 需求 | 状态 | 实现位置 |
|------|------|----------|
| `--dry-run` 参数 | ✅ | `cli.rs` 选项 |
| 显示所有将被删除的文件 | ✅ | `render_clean_preview()` |
| 显示汇总统计 | ✅ | `render_clean_result()` |
| 不修改任何文件或锁文件 | ✅ | `CleanUseCase.execute()` |

## 命令语法对照

| PRD 命令 | 实现状态 | 说明 |
|----------|---------|------|
| `calvin clean` | ✅ | 交互模式 |
| `calvin clean --home` | ✅ | 需要确认 |
| `calvin clean --target cursor` | ❌ | Phase 3 |
| `calvin clean --all` | ❌ | 未实现，建议：不指定 scope 时清除全部 |
| `calvin clean --home --target cursor` | ❌ | Phase 3 |
| `calvin clean --home --dry-run` | ✅ | |
| `calvin clean --home --yes` | ✅ | 非交互模式 |

## Edge Cases 处理状态

| Edge Case | 状态 | 说明 |
|-----------|------|------|
| EC-2: 锁文件丢失 | ⚠️ | 显示错误，但无 `--scan` 选项 |
| EC-3: 文件被修改 | ✅ | 默认跳过，`--force` 可删除 |
| EC-4: 文件已不存在 | ✅ | 静默跳过，从锁文件移除 |
| EC-5: 权限问题 | ✅ | 跳过+报告 |
| EC-7: Remote 部署 | ✅ | 跳过+提示 |
| EC-12: 无签名 | ✅ | 默认跳过，`--force` 可删除 |
| EC-19: Hash 不匹配 | ✅ | 视为"文件已修改" |

## 安全机制对照

| 机制 | 状态 | 实现位置 |
|------|------|----------|
| 签名验证 | ✅ | `has_calvin_signature()` |
| 锁文件匹配 | ✅ | `process_lockfile()` |
| 确认提示 | ✅ | `dialoguer::Confirm` |
| Dry Run | ✅ | `--dry-run` |

## 输出格式对照

| 格式 | 状态 | 说明 |
|------|------|------|
| 树形菜单 | ✅ | `TreeMenu` 组件 |
| 状态图标 ●/○/◐ | ✅ | `icons::SELECTED/UNSELECTED/PARTIAL` |
| 展开/折叠图标 ▼/▶ | ✅ | `icons::EXPAND/COLLAPSE` |
| 统计信息 | ✅ | `render_status_bar()` |
| JSON 输出 | ✅ | `--json` 全局选项 |

## 未实现的功能

### Phase 3 功能（PRD 定义为可选）

1. **`--target` 过滤**: 需要扩展 lockfile 添加 `target` 字段
2. ~~**`--all` 选项**: 显式清除所有 scope~~ ✅ 已实现
3. **`--scan` 选项**: 扫描无锁文件情况

### 其他

1. ~~**主菜单集成**: `calvin` 交互式菜单中没有 clean 选项~~ ✅ 已有 (选项 8)

## 总结

| 分类 | 完成度 |
|------|--------|
| Phase 1: 基础命令 | 100% |
| Phase 2: 交互式菜单 | 100% |
| Phase 3: Lockfile 扩展 | 0% (按 PRD 为可选) |
| Edge Cases | 90% |

**已完成差距修复**:

1. ✅ `--all` 选项已实现
2. ✅ 主菜单已有 clean 选项 (选项 8)

**仍可选实现**:

1. `--scan` 选项 (EC-18) - 无锁文件时扫描 Calvin 签名文件
2. Phase 3: `--target` 过滤 - 需要扩展 lockfile

