# Selection Icons Refactor Plan

> **Status**: Draft  
> **Created**: 2025-12-23  
> **Priority**: Low (cosmetic improvement)

## Objective

将 Calvin CLI 中的多选框样式从 `[x]`/`[ ]` 更新为 `●`/`○`，以符合 `docs/ui-components-spec.md` 中定义的设计规范。

## Current State

当前使用 `dialoguer` 库进行交互式选择，默认样式为：

```text
  [x] Claude Code       Anthropic's coding assistant
  [x] Cursor            AI-first code editor
  [ ] VS Code Copilot   GitHub's AI pair programmer
```

## Target State

更新后的样式应为：

```text
  ● Claude Code       Anthropic's coding assistant
  ● Cursor            AI-first code editor
  ○ VS Code Copilot   GitHub's AI pair programmer
```

## Affected Files

### 使用 dialoguer::MultiSelect 的文件

| 文件 | 行号 | 用途 |
|------|------|------|
| `src/ui/menu.rs` | L55, L79 | 目标平台选择 (`select_targets_interactive`) |
| `src/commands/interactive/wizard.rs` | L6, L74 | 模板选择 (`select_templates`) |

### 使用 dialoguer::Select 的文件

| 文件 | 行号 | 用途 |
|------|------|------|
| `src/commands/interactive/menu.rs` | L6 | 主菜单 |
| `src/commands/interactive/wizard.rs` | L118 | 安全模式选择 |

## Implementation Options

### Option A: 自定义 dialoguer Theme (推荐)

使用 `dialoguer::theme::Theme` trait 创建自定义主题：

```rust
use dialoguer::theme::Theme;
use console::Style;

pub struct CalvinTheme;

impl Theme for CalvinTheme {
    fn format_multi_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        checked: bool,
        active: bool,
    ) -> fmt::Result {
        let icon = if checked { "●" } else { "○" };
        if active {
            write!(f, "  {} {}", icon.cyan(), text)
        } else {
            write!(f, "  {} {}", icon, text)
        }
    }
}
```

**优点**：

- 保留 dialoguer 的键盘导航、滚动等功能
- 只需要修改主题，不需要重写逻辑
- dialoguer 官方支持的方式

**缺点**：

- 需要为每种 prompt 类型实现格式化方法

### Option B: 使用 inquire 库替换 dialoguer

`inquire` 库提供更灵活的自定义选项：

```rust
use inquire::MultiSelect;
use inquire::ui::{RenderConfig, Checkbox};

let render_config = RenderConfig::default()
    .with_checkbox(Checkbox::new("●", "○", "◐"));
```

**优点**：

- 更现代的 API
- 内置更好的自定义支持
- 更好的 Unicode 支持

**缺点**：

- 需要替换依赖
- 需要适配所有现有用法

### Option C: 自定义 MultiSelect 组件

在 `src/ui/widgets/` 中实现自定义多选组件，完全控制渲染。

**优点**：

- 完全控制 UI
- 可以实现树形菜单等高级功能

**缺点**：

- 工作量大
- 需要处理键盘输入、终端控制等

## Recommended Approach

**Option A (自定义 dialoguer Theme)** 是最佳选择：

1. 最小改动
2. 保留现有功能
3. 易于维护

## Implementation Steps

### Step 1: 创建 CalvinTheme

在 `src/ui/theme.rs` 中添加：

```rust
use console::Style;
use dialoguer::theme::Theme;
use std::fmt;

/// Calvin 自定义主题
pub struct CalvinTheme {
    pub unicode: bool,
}

impl CalvinTheme {
    pub fn new(unicode: bool) -> Self {
        Self { unicode }
    }

    fn selected_icon(&self) -> &'static str {
        if self.unicode { "●" } else { "[x]" }
    }

    fn unselected_icon(&self) -> &'static str {
        if self.unicode { "○" } else { "[ ]" }
    }

    fn partial_icon(&self) -> &'static str {
        if self.unicode { "◐" } else { "[-]" }
    }
}

impl Theme for CalvinTheme {
    fn format_multi_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        checked: bool,
        active: bool,
    ) -> fmt::Result {
        let icon = if checked { self.selected_icon() } else { self.unselected_icon() };
        let style = if active { Style::new().cyan() } else { Style::new() };
        write!(f, "  {} {}", style.apply_to(icon), text)
    }

    // 实现其他必要的 trait 方法...
}
```

### Step 2: 更新 src/ui/menu.rs

```rust
use crate::ui::theme::CalvinTheme;

let theme = CalvinTheme::new(supports_unicode());

let selection = MultiSelect::with_theme(&theme)
    .items(&items)
    .defaults(&defaults)
    .interact()?;
```

### Step 3: 更新 wizard.rs

同样应用 `CalvinTheme`：

```rust
let theme = CalvinTheme::new(ui.unicode);

let selection = MultiSelect::with_theme(&theme)
    .items(&items)
    .defaults(&[true, true, false, false, false])
    .interact()?;
```

### Step 4: 添加测试

```rust
#[test]
fn test_calvin_theme_icons() {
    let theme = CalvinTheme::new(true);
    assert_eq!(theme.selected_icon(), "●");
    assert_eq!(theme.unselected_icon(), "○");

    let theme_ascii = CalvinTheme::new(false);
    assert_eq!(theme_ascii.selected_icon(), "[x]");
}
```

## Files to Create/Modify

| Action | File | Description |
|--------|------|-------------|
| Modify | `src/ui/theme.rs` | 添加 CalvinTheme struct |
| Modify | `src/ui/mod.rs` | 导出 CalvinTheme |
| Modify | `src/ui/menu.rs` | 使用 CalvinTheme |
| Modify | `src/commands/interactive/wizard.rs` | 使用 CalvinTheme |
| Create | `src/ui/theme/tests.rs` | 单元测试 |

## Testing Checklist

- [ ] `cargo test` 全部通过
- [ ] Unicode 终端显示 `●`/`○`
- [ ] ASCII 终端显示 `[x]`/`[ ]`
- [ ] 键盘导航仍正常工作
- [ ] 默认选中项仍正确
- [ ] 选择结果正确返回

## Future Considerations

1. **树形菜单**：`calvin clean` 需要的树形多选菜单可能需要 Option C 自定义组件
2. **半选状态**：`◐` 图标需要在树形结构中实现
3. **颜色主题**：后续可以扩展 CalvinTheme 支持更多颜色配置

## Dependencies

当前 `dialoguer` 版本：

```toml
dialoguer = { version = "0.12", default-features = false, features = ["fuzzy-select"] }
```

`dialoguer 0.12` 支持 `with_theme()` 方法。

## References

- [dialoguer Theme 文档](https://docs.rs/dialoguer/latest/dialoguer/theme/trait.Theme.html)
- [Calvin UI Components Spec](../ui-components-spec.md)
- [Clean Command PRD](clean-command-prd.md) - 交互式菜单设计参考
