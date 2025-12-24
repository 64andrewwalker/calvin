# Bug 报告：Targets 配置不被尊重

> 报告日期：2025-12-24
> 严重程度：Critical
> 影响范围：所有使用 config.toml 配置 targets 的用户

---

## 问题描述

用户在 `.promptpack/config.toml` 中配置 `[targets] enabled = [...]`，但：

1. **有效配置被忽略**：`enabled = ["claude-code"]` 仍然部署到所有目标
2. **无效配置静默失败**：`enabled = ["clau"]` 不报错，静默回退到默认值
3. **空列表语义错误**：`enabled = []` 应该表示"不部署"，但实际表示"全部部署"

---

## 根本原因分析

### 问题 1: 配置加载静默失败

`src/config/loader.rs` 第 47-56 行：

```rust
pub fn load_or_default(project_root: Option<&Path>) -> Config {
    if let Some(root) = project_root {
        let project_config = root.join(".promptpack/config.toml");
        if project_config.exists() {
            if let Ok(config) = Config::load(&project_config) {  // ❌ 错误被静默吞掉
                return with_env_overrides(config);
            }
        }
    }
    // ... 回退到默认配置
    with_env_overrides(Config::default())  // 默认 = 所有 targets
}
```

当 TOML 解析失败（如无效的 target 名称）时，错误被 `if let Ok(...)` 静默吞掉，回退到默认配置。

### 问题 2: 无效 Target 名称导致整个配置失败

`src/domain/value_objects/target.rs` 第 6-22 行：

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Target {
    ClaudeCode,   // → "claude-code"
    Cursor,       // → "cursor"
    VSCode,       // → "vs-code" (or "vscode" via alias)
    Antigravity,  // → "antigravity"
    Codex,        // → "codex"
    All,          // → "all"
}
```

用户可能写 `"claude"` 而不是 `"claude-code"`，导致 serde 反序列化失败。

### 问题 3: 空列表语义不符合 PRD

`src/config/types.rs` 第 266-278 行：

```rust
/// Get enabled targets (all if empty)  // ← 注释说明了当前行为
pub fn enabled_targets(&self) -> Vec<Target> {
    if self.targets.enabled.is_empty() {
        vec![Target::ClaudeCode, Target::Cursor, ...]  // 返回所有
    } else {
        self.targets.enabled.clone()
    }
}
```

但根据 `docs/proposals/clean-command-prd.md` 第 506-510 行：

> 1. `enabled = []` 表示"不部署到任何 target"
> 2. 与 `enabled` 字段**缺失**不同（缺失 = 使用默认值 = all）

---

## 影响场景

| 配置 | 预期行为 | 实际行为 |
|------|---------|---------|
| `enabled = ["claude-code"]` | 只部署到 Claude Code | 所有 targets（如果解析失败） |
| `enabled = ["claude"]` | 报错：无效的 target | 静默回退到所有 targets |
| `enabled = ["clau"]` | 报错：无效的 target | 静默回退到所有 targets |
| `enabled = []` | 不部署任何 target | 部署到所有 targets |
| `enabled` 字段缺失 | 部署到所有 targets | 部署到所有 targets ✓ |

---

## 修复建议

### Fix 1: 配置加载失败时报错而非静默回退

```rust
pub fn load_or_default(project_root: Option<&Path>) -> Config {
    if let Some(root) = project_root {
        let project_config = root.join(".promptpack/config.toml");
        if project_config.exists() {
            match Config::load(&project_config) {
                Ok(config) => return with_env_overrides(config),
                Err(e) => {
                    // 打印警告，让用户知道配置有问题
                    eprintln!("Warning: Failed to load config: {}", e);
                    eprintln!("Using default configuration");
                }
            }
        }
    }
    with_env_overrides(Config::default())
}
```

### Fix 2: 验证 Target 名称并提供友好错误信息

在 deploy 命令中添加验证：

```rust
// 验证配置中的 targets
for target_str in &raw_targets {
    if !is_valid_target(target_str) {
        eprintln!("Error: Invalid target '{}' in config.toml", target_str);
        eprintln!("Valid targets: claude-code, cursor, vscode, antigravity, codex, all");
        std::process::exit(1);
    }
}
```

### Fix 3: 区分空列表和字段缺失

修改 `TargetsConfig` 使用 `Option`：

```rust
pub struct TargetsConfig {
    #[serde(default)]
    pub enabled: Option<Vec<Target>>,  // None = 字段缺失 = 所有 targets
                                        // Some([]) = 空列表 = 不部署
                                        // Some([...]) = 指定的 targets
}

impl Config {
    pub fn enabled_targets(&self) -> Vec<Target> {
        match &self.targets.enabled {
            None => Target::ALL_CONCRETE.to_vec(),  // 字段缺失 = 所有
            Some(targets) if targets.is_empty() => vec![],  // 空列表 = 没有
            Some(targets) => targets.clone(),  // 指定的 targets
        }
    }
}
```

---

## 相关文件

| 文件 | 问题 |
|------|------|
| `src/config/loader.rs` | 静默吞掉加载错误 |
| `src/config/types.rs` | 空列表语义不正确 |
| `src/domain/value_objects/target.rs` | 没有验证无效值的友好错误 |
| `src/commands/deploy/cmd.rs` | 没有验证 targets 配置 |

---

## TODO Checklist

```markdown
## Fix 配置问题
- [ ] 配置加载失败时输出警告而非静默回退
- [ ] 无效 target 名称提供友好错误信息
- [ ] 区分 `enabled = []` 和字段缺失
- [ ] 添加 target 名称验证
- [ ] 添加单元测试覆盖各种场景
- [ ] 添加集成测试
```

---

## 验收标准

1. `enabled = ["claude-code"]` → 只部署到 Claude Code
2. `enabled = ["claude"]` → 报错，提示正确的名称
3. `enabled = []` → 不部署任何 target（可能需要用户确认）
4. `enabled` 字段缺失 → 部署到所有 targets（当前行为）
5. 配置解析失败 → 显示警告，不静默回退

