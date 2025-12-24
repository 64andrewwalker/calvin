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

## 设计目标

1. **友好警告**：对无效环境变量值发出警告，不是静默忽略
2. **智能建议**：使用 Levenshtein 距离提供拼写建议（复用现有 `suggest_target` 逻辑）
3. **一致性**：所有环境变量使用相同的验证模式
4. **向后兼容**：无效值仍回退到默认值（警告后），不阻止程序运行

---

## 技术方案

### 1. 创建统一的环境变量验证器

```rust
// src/config/env_validator.rs (新文件)

pub struct EnvVarValidator {
    var_name: &'static str,
    valid_values: &'static [&'static str],
}

impl EnvVarValidator {
    pub fn parse<T, F>(&self, value: &str, parser: F, default: T) -> T
    where
        F: Fn(&str) -> Option<T>,
    {
        match parser(value) {
            Some(parsed) => parsed,
            None => {
                let suggestion = self.suggest(value);
                eprintln!("Warning: Invalid {} value '{}'{}", 
                    self.var_name, value, suggestion);
                eprintln!("Valid values: {}", self.valid_values.join(", "));
                default
            }
        }
    }
    
    fn suggest(&self, value: &str) -> String {
        // 使用 Levenshtein 距离找最相似的
        // ...
    }
}
```

### 2. 重构现有代码

**Before:**
```rust
// loader.rs
if let Ok(mode) = std::env::var("CALVIN_SECURITY_MODE") {
    config.security.mode = match mode.to_lowercase().as_str() {
        "yolo" => SecurityMode::Yolo,
        "strict" => SecurityMode::Strict,
        _ => SecurityMode::Balanced,
    };
}
```

**After:**
```rust
if let Ok(mode) = std::env::var("CALVIN_SECURITY_MODE") {
    config.security.mode = EnvVarValidator::new(
        "CALVIN_SECURITY_MODE",
        SecurityMode::VALID_VALUES,
    ).parse(&mode, SecurityMode::from_str, SecurityMode::Balanced);
}
```

### 3. 为每个 enum 添加 VALID_VALUES 和 from_str

类似 `Target::VALID_NAMES`，为其他 enum 添加：

```rust
impl SecurityMode {
    pub const VALID_VALUES: &'static [&'static str] = &["yolo", "balanced", "strict"];
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "yolo" => Some(Self::Yolo),
            "balanced" => Some(Self::Balanced),
            "strict" => Some(Self::Strict),
            _ => None,
        }
    }
}
```

---

## TDD 实现计划

### Phase 1: 添加 VALID_VALUES 常量

| 步骤 | 测试 | 实现 |
|------|------|------|
| 1.1 | `test_security_mode_valid_values` | 添加 `SecurityMode::VALID_VALUES` |
| 1.2 | `test_security_mode_from_str` | 添加 `SecurityMode::from_str` |
| 1.3 | `test_verbosity_valid_values` | 添加 `Verbosity::VALID_VALUES` |
| 1.4 | `test_verbosity_from_str` | 添加 `Verbosity::from_str` |

### Phase 2: 创建 EnvVarValidator

| 步骤 | 测试 | 实现 |
|------|------|------|
| 2.1 | `test_env_validator_valid_value` | 基础解析功能 |
| 2.2 | `test_env_validator_invalid_value_warning` | 警告输出 |
| 2.3 | `test_env_validator_suggestion` | Levenshtein 建议 |

### Phase 3: 重构 loader.rs

| 步骤 | 测试 | 实现 |
|------|------|------|
| 3.1 | `test_env_security_mode_invalid_warning` | 重构 CALVIN_SECURITY_MODE |
| 3.2 | `test_env_verbosity_invalid_warning` | 重构 CALVIN_VERBOSITY |
| 3.3 | `test_env_targets_invalid_warning` | 重构 CALVIN_TARGETS |

### Phase 4: 集成测试

| 步骤 | 测试 | 实现 |
|------|------|------|
| 4.1 | `cli_env_invalid_warning` | 端到端测试 |

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

## 测试用例示例

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Phase 1.1
    #[test]
    fn test_security_mode_valid_values() {
        assert!(SecurityMode::VALID_VALUES.contains(&"yolo"));
        assert!(SecurityMode::VALID_VALUES.contains(&"balanced"));
        assert!(SecurityMode::VALID_VALUES.contains(&"strict"));
    }

    // Phase 1.2
    #[test]
    fn test_security_mode_from_str() {
        assert_eq!(SecurityMode::from_str("yolo"), Some(SecurityMode::Yolo));
        assert_eq!(SecurityMode::from_str("STRICT"), Some(SecurityMode::Strict));
        assert_eq!(SecurityMode::from_str("invalid"), None);
    }

    // Phase 2.2
    #[test]
    fn test_env_validator_invalid_value_warning() {
        let validator = EnvVarValidator::new("TEST_VAR", &["foo", "bar"]);
        // Capture stderr and verify warning is printed
        let result = validator.parse("fooo", |s| {
            if s == "foo" || s == "bar" { Some(s.to_string()) } else { None }
        }, "default".to_string());
        
        assert_eq!(result, "default");
        // Assert stderr contains "Warning: Invalid TEST_VAR value 'fooo'"
        // Assert stderr contains "Did you mean 'foo'?"
    }

    // Phase 3.1
    #[test]
    fn test_env_security_mode_invalid_warning() {
        std::env::set_var("CALVIN_SECURITY_MODE", "strct"); // typo
        let config = load_or_default(None);
        assert_eq!(config.security.mode, SecurityMode::Balanced);
        // Verify warning was printed to stderr
        std::env::remove_var("CALVIN_SECURITY_MODE");
    }
}
```

---

## 预估工作量

| 阶段 | 时间估计 |
|------|----------|
| Phase 1 | 30 min |
| Phase 2 | 45 min |
| Phase 3 | 30 min |
| Phase 4 | 15 min |
| 文档 | 10 min |
| **总计** | **~2 小时** |

---

## 风险与缓解

| 风险 | 缓解措施 |
|------|----------|
| 破坏现有测试 | 每个 phase 后运行完整测试 |
| stderr 测试困难 | 使用 writer 注入或 capture_stderr crate |
| 建议功能复杂 | 复用现有 `suggest_target` 中的 Levenshtein 实现 |

---

## 相关文档

- `docs/reports/config-silent-failures-audit-2025-12-24.md` - 审查报告
- `docs/reports/targets-config-bug-2025-12-24.md` - 原始 bug 报告
- `src/domain/value_objects/target.rs` - Levenshtein 实现参考

