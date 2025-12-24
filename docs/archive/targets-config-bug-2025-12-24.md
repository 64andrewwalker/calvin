# Bug 报告：Targets 配置不被尊重

> 报告日期：2025-12-24
> 修复日期：2025-12-24
> 严重程度：Critical
> 状态：**已修复** ✅

---

## 问题描述

用户在 `.promptpack/config.toml` 中配置 `[targets] enabled = [...]`，但：

1. **有效配置被忽略**：`enabled = ["claude-code"]` 仍然部署到所有目标
2. **无效配置静默失败**：`enabled = ["clau"]` 不报错，静默回退到默认值
3. **空列表语义错误**：`enabled = []` 应该表示"不部署"，但实际表示"全部部署"

---

## 修复内容

### Fix 1: 配置加载失败时输出警告

`src/config/loader.rs`:
```rust
match Config::load(&project_config) {
    Ok(config) => return with_env_overrides(config),
    Err(e) => {
        eprintln!("Warning: Failed to load config: {}", e);
        eprintln!("Using default configuration");
    }
}
```

### Fix 2: 区分 `enabled = []` 和字段缺失

`src/config/types.rs`:
```rust
pub struct TargetsConfig {
    pub enabled: Option<Vec<Target>>, // None = 全部, Some([]) = 无
}

impl Config {
    pub fn enabled_targets(&self) -> Vec<Target> {
        match &self.targets.enabled {
            None => Target::ALL_CONCRETE.to_vec(),
            Some(targets) if targets.is_empty() => vec![],
            Some(targets) => targets.clone(),
        }
    }
}
```

### Fix 3: 无效 target 名称提供友好错误信息

`src/domain/value_objects/target.rs`:
```rust
impl Target {
    pub const VALID_NAMES: &'static [&'static str] = &[
        "claude-code", "claude", "cursor", "vscode", "vs-code", 
        "antigravity", "codex", "all"
    ];
    
    pub fn from_str_with_suggestion(s: &str) -> Result<Target, TargetParseError> {
        // ... with Levenshtein suggestion
    }
}
```

---

## 测试覆盖

- `config::tests::test_enabled_targets_empty_list_means_none`
- `config::tests::test_enabled_targets_section_missing_means_all`
- `deploy_respects_config_empty_targets` (e2e)

---

## 相关 PR

- `fix/targets-not-respected` 分支
- `feat/env-var-validation` 分支

