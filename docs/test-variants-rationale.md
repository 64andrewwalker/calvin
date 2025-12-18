# Test Variants Rationale

本文档说明为 `SyncEngine` 添加的变体测试及其覆盖的边界情况。

## 变体测试统计

| 类别 | 新增测试数 |
|-----|-----------|
| 输入边界 | 6 |
| 状态变体 | 3 |
| 冲突场景 | 2 |
| Dry Run 验证 | 2 |
| Callback 验证 | 1 |
| Lockfile 路径验证 | 2 |
| **总计** | **16** |

## 输入边界测试

### `engine_sync_with_empty_outputs`
- **场景:** 空输出列表
- **覆盖:** 边界条件处理，确保空列表不会导致错误
- **预期:** 成功返回空结果

### `engine_sync_with_empty_content`
- **场景:** 文件内容为空字符串
- **覆盖:** 空内容处理，零字节文件写入
- **预期:** 创建空文件

### `engine_sync_with_unicode_content`
- **场景:** 包含中文、日文、emoji、特殊字符的内容
- **覆盖:** UTF-8 编码正确性
- **预期:** 完整保留 Unicode 内容

### `engine_sync_with_large_content`
- **场景:** ~1MB 大文件
- **覆盖:** 大文件处理，内存效率
- **预期:** 正确写入和哈希

### `engine_sync_with_special_path_characters`
- **场景:** 路径包含空格、连字符、下划线
- **覆盖:** 路径编码，特殊字符转义
- **预期:** 正确创建目录和文件

### `engine_sync_with_deeply_nested_path`
- **场景:** 8 层嵌套目录
- **覆盖:** 递归目录创建
- **预期:** 创建所有中间目录

## 状态变体测试

### `engine_sync_with_preexisting_lockfile`
- **场景:** 存在旧版本 lockfile
- **覆盖:** Lockfile 兼容性，增量更新
- **预期:** 正确合并新旧条目

### `engine_sync_multiple_files_same_directory`
- **场景:** 多文件写入同一目录
- **覆盖:** 并发目录访问，批量写入
- **预期:** 所有文件正确写入

### `engine_sync_multiple_files_different_directories`
- **场景:** 多文件写入不同目录
- **覆盖:** 多目录创建，路径隔离
- **预期:** 各目录独立正确

## 冲突场景测试

### `engine_sync_skip_mode_leaves_conflicts_unchanged`
- **场景:** 用户选择 Skip 时的冲突处理
- **覆盖:** Skip 选项正确执行
- **预期:** 用户修改内容保留

### `engine_sync_non_interactive_skips_conflicts_silently`
- **场景:** 非交互模式遇到冲突
- **覆盖:** 默认冲突处理策略
- **预期:** 静默跳过，不报错

## Dry Run 验证测试

### `engine_dry_run_does_not_create_files`
- **场景:** Dry run 模式文件创建
- **覆盖:** Dry run 隔离
- **预期:** 报告写入但不实际创建

### `engine_dry_run_does_not_update_lockfile`
- **场景:** Dry run 模式 lockfile 更新
- **覆盖:** Dry run 完整隔离
- **预期:** Lockfile 不被创建/修改

## Callback 验证测试

### `engine_sync_callback_receives_correct_events`
- **场景:** 验证事件回调顺序
- **覆盖:** 事件流完整性
- **预期:** 每个文件收到 ItemStart 和 ItemWritten

## Lockfile 路径验证测试

### `engine_local_uses_project_lockfile`
- **场景:** Local 部署的 lockfile 路径
- **覆盖:** 路径计算正确性
- **预期:** `.promptpack/.calvin.lock`

### `engine_remote_uses_source_lockfile`
- **场景:** Remote 部署的 lockfile 路径
- **覆盖:** Remote 模式本地 lockfile
- **预期:** `source/.calvin.lock`

## 防止过拟合措施

1. **多种输入类型:** Unicode、空内容、大文件
2. **边界值:** 空列表、深嵌套、特殊字符
3. **状态独立:** 使用 TempDir 和 MockFileSystem 隔离
4. **行为验证:** 不仅检查结果，还验证副作用（lockfile、事件）

## 未来可能的扩展

- Property-based testing (使用 proptest)
- 并发访问测试
- 磁盘满/权限拒绝场景
- 网络失败模拟 (remote)

