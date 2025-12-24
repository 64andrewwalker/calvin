# 配置静默失败审查报告

> 审查日期：2025-12-24
> 触发原因：修复 targets-config-bug 后的全面审查
> 审查范围：配置加载、解析、验证相关代码

---

## 背景

在修复 `targets-config-bug-2025-12-24` 时，我们发现了几个配置相关的问题模式：
1. 错误被 `if let Ok(...)` 静默吞掉
2. 空列表 vs 未设置的语义混淆
3. 无效值没有友好错误信息

本报告审查项目中是否存在其他类似问题。

---

## 发现的问题

### 问题 1: 环境变量解析无效值静默回退 [中等]

**位置**: `src/config/loader.rs:107-139`

```rust
// CALVIN_SECURITY_MODE
config.security.mode = match mode.to_lowercase().as_str() {
    "yolo" => SecurityMode::Yolo,
    "strict" => SecurityMode::Strict,
    _ => SecurityMode::Balanced,  // ❌ 无效值静默回退，无警告
};
```

**影响**: 用户设置 `CALVIN_SECURITY_MODE=strct`（拼写错误），会静默使用 `balanced` 而不是报错。

**类似问题**:
- `CALVIN_VERBOSITY` - 无效值回退到 `normal`
- `CALVIN_TARGETS` - 无效值被静默忽略

**建议修复**:
```rust
config.security.mode = match mode.to_lowercase().as_str() {
    "yolo" => SecurityMode::Yolo,
    "balanced" => SecurityMode::Balanced,
    "strict" => SecurityMode::Strict,
    invalid => {
        eprintln!("Warning: Invalid CALVIN_SECURITY_MODE '{}'. Using 'balanced'.", invalid);
        eprintln!("Valid values: yolo, balanced, strict");
        SecurityMode::Balanced
    }
};
```

---

### 问题 2: 安全检查文件解析错误静默忽略 [低]

**位置**: `src/security/checks.rs:68-69, 206-207`

```rust
if let Ok(content) = std::fs::read_to_string(&settings_file) {
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
        // 处理解析后的内容
    }
    // ❌ JSON 解析失败时静默忽略
}
```

**影响**: 如果 `.cursor/settings.json` 或 `.cursor/mcp.json` 文件格式无效，安全检查会静默跳过而不是报告问题。

**评估**: 这是 doctor 命令的一部分，可能是有意为之（文件可能不存在或格式不标准）。但最好添加一个 info 级别的消息。

---

### 问题 3: MockConfigRepository 使用了旧的静默回退模式 [低]

**位置**: `src/domain/ports/config_repository.rs:173`

```rust
if let Ok(config) = self.load(&project_config_path) {
    return config;
}
```

**评估**: 这是测试代码中的 Mock 实现，应该与实际 `load_or_default` 保持一致。但由于是测试代码，优先级较低。

---

## 已正确处理的模式

以下使用 `if let Ok(...)` 的地方是合理的：

| 位置 | 原因 |
|------|------|
| `use_case.rs:645` | 读取文件检查签名，文件可能不存在 |
| `clean.rs:178` | 同上 |
| `parser.rs:144` | 路径前缀处理，可能不匹配 |
| `remote.rs:46` | 远程命令执行，失败是预期的 |
| `json.rs:32` | 事件写入，不影响主流程 |
| `watch/*.rs` | 文件监控事件，需要容错 |

---

## 建议修复优先级

| 优先级 | 问题 | 工作量 |
|--------|------|--------|
| P2 | 环境变量无效值警告 | 1h |
| P3 | security/checks.rs 解析错误信息 | 2h |
| P4 | Mock 代码更新 | 0.5h |

---

## 结论

项目中存在少量类似的静默失败问题，但严重程度都比修复的 targets 配置问题低。主要原因：

1. **环境变量**: 用户较少使用，且错误影响可预测
2. **安全检查文件**: 文件可能不存在或格式不标准，容错是合理的
3. **测试代码**: 不影响生产行为

可以在后续迭代中逐步修复，当前没有阻塞性问题。

---

## 附录：审查的文件

```
src/config/loader.rs
src/config/types.rs
src/security/checks.rs
src/domain/ports/config_repository.rs
src/application/deploy/use_case.rs
src/application/watch/*.rs
src/infrastructure/fs/remote.rs
src/infrastructure/events/json.rs
src/commands/clean.rs
src/parser.rs
```

