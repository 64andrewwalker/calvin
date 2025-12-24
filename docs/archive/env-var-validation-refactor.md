# 环境变量验证重构方案

> 创建日期：2025-12-24
> 完成日期：2025-12-24
> 状态：**已完成** ✅
> 优先级：P2

---

## 问题描述

当前环境变量处理存在静默失败问题：

```rust
// src/config/loader.rs:107-139
config.security.mode = match mode.to_lowercase().as_str() {
    "yolo" => SecurityMode::Yolo,
    "strict" => SecurityMode::Strict,
    _ => SecurityMode::Balanced,  // ❌ 无效值静默回退，无警告
};
```

**影响**：
- 用户设置 `CALVIN_SECURITY_MODE=strct`（拼写错误）→ 静默使用 `balanced`
- 用户设置 `CALVIN_VERBOSITY=quite`（拼写错误）→ 静默使用 `normal`
- 用户设置 `CALVIN_TARGETS=cluade-code`（拼写错误）→ 被静默忽略

---

## TODO

- [x] **Phase 1.1**: 为 SecurityMode 添加 VALID_VALUES 常量
- [x] **Phase 1.2**: 为 SecurityMode 添加 from_str 方法
- [x] **Phase 1.3**: 为 Verbosity 添加 VALID_VALUES 常量
- [x] **Phase 1.4**: 为 Verbosity 添加 from_str 方法
- [x] **Phase 2.1**: 创建 EnvVarValidator 基础结构
- [x] **Phase 2.2**: 实现无效值警告输出
- [x] **Phase 2.3**: 实现 Levenshtein 建议功能
- [x] **Phase 3.1**: 重构 CALVIN_SECURITY_MODE 处理
- [x] **Phase 3.2**: 重构 CALVIN_VERBOSITY 处理
- [x] **Phase 3.3**: 重构 CALVIN_TARGETS 处理
- [x] **Phase 4.1**: 添加端到端测试
- [x] **收尾**: 更新文档，commit

---

## 相关文档

- `docs/reports/config-silent-failures-audit-2025-12-24.md` - 审查报告
- `docs/archive/targets-config-bug-2025-12-24.md` - 原始 bug 报告
- `src/domain/value_objects/target.rs` - Levenshtein 实现参考

