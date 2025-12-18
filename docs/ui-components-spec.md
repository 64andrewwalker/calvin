# Calvin UI 组件规格说明

> **版本**: v0.3.0  
> **日期**: 2025-12-18  
> **目标**: 模块化、可复用的 UI 组件设计

---

## 一、组件架构概览

```
src/ui/
├── mod.rs              # 统一导出
├── theme.rs            # 设计令牌 (颜色、图标、边框)
├── terminal.rs         # 终端能力检测
├── render.rs           # 渲染引擎 (buffer + flush)
│
├── primitives/         # 原子组件 (不可再分)
│   ├── mod.rs
│   ├── text.rs         # 带颜色的文本
│   ├── icon.rs         # 状态图标
│   └── border.rs       # 边框字符
│
├── widgets/            # 基础组件 (由 primitives 组合)
│   ├── mod.rs
│   ├── spinner.rs      # 加载动画
│   ├── progress.rs     # 进度条
│   ├── list.rs         # 状态列表
│   └── box.rs          # 边框容器
│
├── blocks/             # 复合组件 (业务相关)
│   ├── mod.rs
│   ├── header.rs       # 命令头部 (📦 Calvin Deploy)
│   ├── summary.rs      # 结果摘要框
│   ├── error.rs        # 错误展示框
│   └── check_item.rs   # 检查项 (带展开详情)
│
└── views/              # 完整视图 (对应命令输出)
    ├── mod.rs
    ├── deploy.rs       # deploy 命令视图
    ├── check.rs        # check 命令视图
    ├── watch.rs        # watch 命令视图
    └── interactive.rs  # 交互式菜单视图
```

---

## 二、设计令牌 (theme.rs)

### 2.1 颜色常量

```rust
/// 精选 5 种颜色，严禁使用其他颜色
pub mod colors {
    use owo_colors::OwoColorize;
    
    /// 成功、完成、安全
    pub const SUCCESS: &str = "green";
    
    /// 错误、失败、危险
    pub const ERROR: &str = "red";
    
    /// 警告、注意、进行中
    pub const WARNING: &str = "yellow";
    
    /// 信息、标题、可交互
    pub const INFO: &str = "cyan";
    
    /// 次要信息、边框、注释
    pub const DIM: &str = "dimmed";
}

/// 应用颜色的工具函数
pub fn success<D: std::fmt::Display>(text: D) -> String;
pub fn error<D: std::fmt::Display>(text: D) -> String;
pub fn warning<D: std::fmt::Display>(text: D) -> String;
pub fn info<D: std::fmt::Display>(text: D) -> String;
pub fn dim<D: std::fmt::Display>(text: D) -> String;
```

### 2.2 图标常量

```rust
/// 统一状态图标
pub mod icons {
    pub const SUCCESS: &str = "✓";
    pub const ERROR: &str = "✗";
    pub const WARNING: &str = "⚠";
    pub const PROGRESS: &str = "●";
    pub const PENDING: &str = "○";
    pub const ARROW: &str = "↳";
    pub const WATCH: &str = "⟳";
    pub const DEPLOY: &str = "📦";
    pub const CHECK: &str = "🔍";
    pub const REMOTE: &str = "📡";
}

/// ASCII 降级 (当 supports_unicode = false)
pub mod icons_ascii {
    pub const SUCCESS: &str = "[OK]";
    pub const ERROR: &str = "[FAIL]";
    pub const WARNING: &str = "[WARN]";
    pub const PROGRESS: &str = "[..]";
    pub const PENDING: &str = "[ ]";
    pub const ARROW: &str = "[>]";
}
```

### 2.3 边框常量

```rust
/// 圆角边框字符 (统一样式)
pub mod borders {
    pub const TOP_LEFT: &str = "╭";
    pub const TOP_RIGHT: &str = "╮";
    pub const BOTTOM_LEFT: &str = "╰";
    pub const BOTTOM_RIGHT: &str = "╯";
    pub const HORIZONTAL: &str = "─";
    pub const VERTICAL: &str = "│";
    pub const DIVIDER: &str = "─────────────────────────────────────────────────";
}

/// ASCII 降级
pub mod borders_ascii {
    pub const TOP_LEFT: &str = "+";
    pub const TOP_RIGHT: &str = "+";
    pub const BOTTOM_LEFT: &str = "+";
    pub const BOTTOM_RIGHT: &str = "+";
    pub const HORIZONTAL: &str = "-";
    pub const VERTICAL: &str = "|";
}
```

---

## 三、原子组件 (primitives/)

### 3.1 ColoredText

```rust
/// 带颜色的文本片段
pub struct ColoredText {
    text: String,
    color: Option<Color>,
    bold: bool,
}

impl ColoredText {
    pub fn new(text: impl Into<String>) -> Self;
    pub fn success(text: impl Into<String>) -> Self;
    pub fn error(text: impl Into<String>) -> Self;
    pub fn warning(text: impl Into<String>) -> Self;
    pub fn info(text: impl Into<String>) -> Self;
    pub fn dim(text: impl Into<String>) -> Self;
    pub fn bold(mut self) -> Self;
    pub fn render(&self, supports_color: bool) -> String;
}
```

### 3.2 Icon

```rust
/// 可降级的状态图标
pub enum Icon {
    Success,
    Error,
    Warning,
    Progress,
    Pending,
    Arrow,
}

impl Icon {
    pub fn render(&self, supports_unicode: bool) -> &'static str;
    pub fn colored(&self, supports_color: bool, supports_unicode: bool) -> String;
}
```

---

## 四、基础组件 (widgets/)

### 4.1 Spinner

```rust
/// 加载动画组件
pub struct Spinner {
    frames: &'static [char],
    current: usize,
    message: String,
    started: Instant,
}

impl Spinner {
    pub fn new(message: impl Into<String>) -> Self;
    
    /// 更新一帧
    pub fn tick(&mut self);
    
    /// 渲染当前帧
    pub fn render(&self, supports_unicode: bool) -> String;
    
    /// 成功结束
    pub fn succeed(self, message: &str) -> String;
    
    /// 失败结束
    pub fn fail(self, message: &str) -> String;
}

// Braille spinner 帧序列
const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const SPINNER_FRAMES_ASCII: &[char] = &['-', '\\', '|', '/'];
```

**使用场景**:
- `calvin deploy`: 扫描阶段、编译阶段
- `calvin check`: 扫描阶段、各检查项
- `calvin deploy --remote`: SSH 连接阶段

### 4.2 ProgressBar

```rust
/// 进度条组件
pub struct ProgressBar {
    total: u64,
    current: u64,
    width: u16,
    message: String,
    started: Instant,
}

impl ProgressBar {
    pub fn new(total: u64) -> Self;
    pub fn with_message(total: u64, message: impl Into<String>) -> Self;
    
    /// 增加进度
    pub fn inc(&mut self, delta: u64);
    
    /// 设置进度
    pub fn set(&mut self, value: u64);
    
    /// 计算预估剩余时间
    pub fn eta(&self) -> Option<Duration>;
    
    /// 渲染进度条
    pub fn render(&self) -> String;
    // 输出: ━━━━━━━━━━━━━━━━━━━━  12/24 (50%)  ETA: 3s
}
```

**使用场景**:
- `calvin deploy`: 写入阶段
- `calvin deploy --remote`: 文件传输阶段

### 4.3 StatusList

```rust
/// 带状态的项目列表
pub struct StatusList {
    items: Vec<StatusItem>,
    visible_count: Option<usize>,
}

pub struct StatusItem {
    pub label: String,
    pub status: ItemStatus,
    pub detail: Option<String>,
}

pub enum ItemStatus {
    Pending,
    InProgress,
    Success,
    Warning,
    Error,
}

impl StatusList {
    pub fn new() -> Self;
    pub fn with_visible_count(count: usize) -> Self;
    
    pub fn add(&mut self, label: impl Into<String>);
    pub fn update(&mut self, index: usize, status: ItemStatus);
    pub fn update_detail(&mut self, index: usize, detail: impl Into<String>);
    
    pub fn render(&self) -> String;
}
```

**渲染示例**:
```
  ✓ actions/review.md         → .claude/, .cursor/
  ✓ actions/test.md           → .claude/, .cursor/
  ● policies/style.md         compiling...
  ○ agents/helper.md
```

**使用场景**:
- `calvin deploy`: 文件编译列表
- `calvin check`: 检查项列表
- `calvin deploy --remote`: 文件传输列表

### 4.4 Box (边框容器)

```rust
/// 带边框的内容容器
pub struct Box {
    title: Option<String>,
    content: Vec<String>,
    width: Option<u16>,
    style: BoxStyle,
}

pub enum BoxStyle {
    Info,       // 青色边框
    Success,    // 绿色边框  
    Warning,    // 黄色边框
    Error,      // 红色边框
}

impl Box {
    pub fn new() -> Self;
    pub fn with_title(title: impl Into<String>) -> Self;
    pub fn with_style(style: BoxStyle) -> Self;
    
    pub fn add_line(&mut self, line: impl Into<String>);
    pub fn add_empty(&mut self);
    
    pub fn render(&self) -> String;
}
```

**渲染示例**:
```
╭─────────────────────────────────────────────────╮
│  ✓ Deploy Complete                              │
│                                                 │
│  36 assets → 3 targets                          │
│  110 files written                              │
│  0 skipped, 0 errors                            │
╰─────────────────────────────────────────────────╯
```

---

## 五、复合组件 (blocks/)

### 5.1 CommandHeader

```rust
/// 命令头部信息块
pub struct CommandHeader {
    icon: &'static str,
    title: String,
    items: Vec<(String, String)>,
}

impl CommandHeader {
    pub fn new(icon: &'static str, title: impl Into<String>) -> Self;
    pub fn add(&mut self, label: impl Into<String>, value: impl Into<String>);
    pub fn render(&self) -> String;
}
```

**使用示例**:
```rust
let mut header = CommandHeader::new("📦", "Calvin Deploy");
header.add("Source", ".promptpack/");
header.add("Target", "Home (~/)")
header.add("Mode", "Interactive");
```

**渲染输出**:
```
📦 Calvin Deploy
Source: .promptpack/
Target: Home (~/)
Mode: Interactive
```

### 5.2 ResultSummary

```rust
/// 结果摘要块
pub struct ResultSummary {
    title: String,
    success: bool,
    stats: Vec<(String, usize)>,
    warnings: Vec<String>,
    next_step: Option<String>,
}

impl ResultSummary {
    pub fn success(title: impl Into<String>) -> Self;
    pub fn partial(title: impl Into<String>) -> Self;
    
    pub fn add_stat(&mut self, label: impl Into<String>, count: usize);
    pub fn add_warning(&mut self, message: impl Into<String>);
    pub fn with_next_step(&mut self, hint: impl Into<String>);
    
    pub fn render(&self) -> String;
}
```

**渲染输出**:
```
╭─────────────────────────────────────────────────╮
│  ✓ Deploy Complete                              │
│                                                 │
│  36 assets → 3 targets                          │
│  110 files written                              │
│  0 skipped, 0 errors                            │
│                                                 │
│  Next: Run 'calvin check' to verify             │
╰─────────────────────────────────────────────────╯
```

### 5.3 ErrorBlock

```rust
/// 错误展示块 (带代码预览)
pub struct ErrorBlock {
    file: PathBuf,
    line: Option<usize>,
    message: String,
    code_context: Option<Vec<(usize, String, bool)>>, // (行号, 内容, 是否高亮)
    fix: Option<String>,
}

impl ErrorBlock {
    pub fn new(file: impl Into<PathBuf>, message: impl Into<String>) -> Self;
    pub fn with_line(mut self, line: usize) -> Self;
    pub fn with_code_context(mut self, lines: Vec<(usize, String, bool)>) -> Self;
    pub fn with_fix(mut self, fix: impl Into<String>) -> Self;
    pub fn render(&self) -> String;
}
```

### 5.4 CheckItem

```rust
/// 检查项组件 (用于 check 命令)
pub struct CheckItem {
    platform: String,
    name: String,
    status: CheckStatus,
    message: String,
    recommendation: Option<String>,
    details: Vec<String>,
}

pub enum CheckStatus {
    Pass,
    Warning,
    Error,
}

impl CheckItem {
    pub fn render(&self, verbose: bool) -> String;
}
```

**渲染输出**:
```
Claude Code
  ✓ commands - 36 user commands installed
  ⚠ settings - No settings.json found
    ↳ Run `calvin deploy` to generate security baseline
```

---

## 六、命令视图规格

### 6.1 Deploy 视图

**当前代码位置**: `src/commands/deploy.rs` 第 39-61 行, 226-240 行

**阶段分解**:

| 阶段 | 当前实现 | 目标组件 | 动画 |
|-----|---------|---------|-----|
| 头部信息 | `println!("📦 Calvin Deploy")` | `CommandHeader` | 无 |
| 解析 | `println!("✓ Parsed {} assets")` | `Spinner` → 成功消息 | Spinner |
| 编译 | `println!("✓ Compiled to {} files")` | `StatusList` | 逐项更新 |
| 写入 | (无进度) | `ProgressBar` | 进度条 |
| 结果 | 多个 `println!` | `ResultSummary` | 无 |

**目标流程**:
```
📦 Calvin Deploy
Source: .promptpack/
Target: Home (~/)
Mode: Interactive

⠋ Scanning .promptpack/...
✓ Found 36 prompts, 5 policies, 2 agents

  actions/
    ✓ review.md      → .claude/, .cursor/
    ✓ test.md        → .claude/, .cursor/
    ● refactor.md    → compiling...
    ○ docs.md

━━━━━━━━━━━━━━━━━━━━━━━━━━━━  12/43 (28%)

╭─────────────────────────────────────────────────╮
│  ✓ Deploy Complete                              │
│                                                 │
│  43 assets → 3 targets                          │
│  110 files written                              │
│  0 skipped, 0 errors                            │
╰─────────────────────────────────────────────────╯
```

### 6.2 Deploy --remote 视图

**当前代码位置**: `src/commands/deploy.rs` 第 184-214 行

**阶段分解**:

| 阶段 | 当前实现 | 目标组件 | 动画 |
|-----|---------|---------|-----|
| SSH 连接 | `println!("📡 Using rsync...")` | `Spinner` | Spinner |
| 认证结果 | (无) | 成功/失败消息 | 无 |
| 文件传输 | (无进度) | `StatusList` + `ProgressBar` | 双重进度 |
| 速度/ETA | (无) | 内嵌在进度条 | 实时更新 |
| 完成摘要 | 同上 | `ResultSummary` (含传输统计) | 无 |

**目标流程**:
```
📦 Calvin Deploy
Source: .promptpack/
Remote: user@server.example.com:/path

⠋ Connecting via SSH...
✓ Connected (key: ~/.ssh/id_ed25519)

📡 Uploading 24 files...

  ✓ .claude/commands/review.md         2.1 KB
  ✓ .claude/commands/test.md           1.8 KB
  ● .cursor/rules/style.md             uploading...
  ○ .cursor/commands/review.md

━━━━━━━━━━━━━━━━━━━━━━━━━━━━  12/24 (50%)
Speed: 1.2 MB/s  |  ETA: 3s

╭─────────────────────────────────────────────────╮
│  ✓ Remote Deploy Complete                       │
│                                                 │
│  24 files uploaded                              │
│  Total: 48.2 KB in 8s (6.0 KB/s)               │
│                                                 │
│  Verify: ssh user@server 'ls -la .claude/'      │
╰─────────────────────────────────────────────────╯
```

### 6.3 Check 视图

**当前代码位置**: `src/commands/check.rs` 第 41-90 行

**阶段分解**:

| 阶段 | 当前实现 | 目标组件 | 动画 |
|-----|---------|---------|-----|
| 头部 | `println!("🔍 Calvin Check")` | `CommandHeader` | 无 |
| 检查过程 | (无进度) | `Spinner` per check | Spinner |
| 平台分组 | 手动格式化 | `CheckItem` 集合 | 逐项更新 |
| 摘要 | `println!("Summary: ...")` | 简单文本 | 无 |
| 最终状态 | emoji + 文本 | 状态消息 | 无 |

**目标流程**:
```
🔍 Calvin Check
Mode: Balanced

⠋ Running health checks...

Claude Code
  ✓ commands - 36 user commands installed
  ⚠ settings - No settings.json found
    ↳ Run `calvin deploy` to generate security baseline

Cursor
  ✓ rules - 12 rules synced
  ✓ commands - 36 commands synced

Antigravity
  ✓ workflows - 37 global workflows installed

Summary: 8 passed, 1 warning, 0 errors

🟡 Check passed with warnings.
```

### 6.4 Watch 视图

**当前代码位置**: `src/commands/watch.rs` 第 44-76 行

**阶段分解**:

| 阶段 | 当前实现 | 目标组件 | 动画 |
|-----|---------|---------|-----|
| 启动 | `println!("👀 Calvin Watch")` | `CommandHeader` | 无 |
| 监听中 | `println!("📂 Watching: ...")` | `Spinner` (持续) | 持续 Spinner |
| 文件变更 | `println!("📝 Changed: ...")` | 带时间戳消息 | 无 |
| 同步中 | `println!("🔄 Syncing...")` | `Spinner` | Spinner |
| 同步结果 | `println!("✓ Sync: ...")` | 简洁结果 | 无 |
| 退出 | `println!("👋 Shutting down...")` | 关闭消息 | 无 |

**目标流程**:
```
👀 Calvin Watch
Source: .promptpack/

⟳ Watching for changes...
  Press Ctrl+C to stop

─────────────────────────────────────────────────

[14:32:05] 📝 Changed: actions/review.md
[14:32:05] ⠋ Syncing...
[14:32:06] ✓ Synced 3 files to .claude/, .cursor/

[14:35:12] 📝 Changed: policies/style.md
[14:35:12] ⠋ Syncing...
[14:35:12] ✓ Synced 2 files to .claude/, .cursor/

─────────────────────────────────────────────────

^C
👋 Watch stopped. Synced 2 changes total.
```

### 6.5 Interactive 菜单视图

**当前代码位置**: `src/commands/interactive.rs`

#### 6.5.1 首次运行菜单 (第 47-84 行)

**当前实现**: 使用 `dialoguer::Select`

**目标增强**:
- 使用 `Box` 组件包裹 banner
- 菜单项保持 dialoguer 但统一样式
- 添加更清晰的退出提示

**目标流程**:
```
╭─────────────────────────────────────────────────╮
│                                                 │
│  Calvin - Making AI agents behave               │
│                                                 │
│  Maintain AI rules in one place, deploy to      │
│  Claude, Cursor, VS Code, and more.             │
│                                                 │
╰─────────────────────────────────────────────────╯

No .promptpack/ directory found.

What would you like to do?

  > [1] Set up Calvin for this project
    [2] Learn what Calvin does first
    [3] Show commands (for experts)
    [4] Explain yourself (for AI assistants)
    [q] Quit

Use ↑↓ to navigate, Enter to select
```

#### 6.5.2 已有项目菜单 (第 86-137 行)

**目标流程**:
```
╭─────────────────────────────────────────────────╮
│                                                 │
│  Calvin - Making AI agents behave               │
│                                                 │
│  Found .promptpack/ with 36 prompts             │
│  Last deployed: 2 hours ago                     │
│                                                 │
╰─────────────────────────────────────────────────╯

What would you like to do?

  > [1] Deploy to this project
    [2] Deploy to home directory
    [3] Deploy to remote server
    [4] Preview changes (diff)
    [5] Watch mode
    [6] Check configuration
    [7] Explain yourself
    [q] Quit

Use ↑↓ to navigate, Enter to select
```

#### 6.5.3 Setup Wizard (第 139-161 行)

**阶段分解**:
- Step 1: 目标选择 (MultiSelect)
- Step 2: 模板选择 (MultiSelect)
- Step 3: 安全模式 (Select)
- 完成摘要

**目标流程**:
```
Great! Let's set up Calvin in 3 quick steps.

╭─ Step 1 of 3 ───────────────────────────────────╮
│  Which AI assistants do you use?                │
╰─────────────────────────────────────────────────╯

  [x] Claude Code       Anthropic's coding assistant
  [x] Cursor            AI-first code editor
  [ ] VS Code Copilot   GitHub's AI pair programmer
  [ ] Antigravity       Google's Gemini-powered agent
  [ ] Codex             OpenAI's CLI tool

TIP: You can change this later in .promptpack/config.toml

(Space to toggle, Enter to confirm)
```

---

## 七、组件复用矩阵

| 组件 | deploy | deploy --remote | check | watch | interactive |
|-----|--------|-----------------|-------|-------|-------------|
| Spinner | ✓ | ✓ | ✓ | ✓ | - |
| ProgressBar | ✓ | ✓ | - | - | - |
| StatusList | ✓ | ✓ | ✓ | - | - |
| Box | ✓ | ✓ | - | - | ✓ |
| CommandHeader | ✓ | ✓ | ✓ | ✓ | - |
| ResultSummary | ✓ | ✓ | - | - | - |
| ErrorBlock | ✓ | ✓ | ✓ | ✓ | - |
| CheckItem | - | - | ✓ | - | - |

---

## 八、实现优先级

### Phase 0: 基础设施 (必须)

1. `theme.rs` - 颜色、图标、边框常量
2. `terminal.rs` - 能力检测 (TTY, color, unicode, CI)
3. `primitives/*` - ColoredText, Icon

### Phase 1: 核心组件 (必须)

1. `widgets/spinner.rs` - Spinner
2. `widgets/progress.rs` - ProgressBar
3. `widgets/list.rs` - StatusList
4. `widgets/box.rs` - Box

### Phase 2: 复合组件 (高优)

1. `blocks/header.rs` - CommandHeader
2. `blocks/summary.rs` - ResultSummary
3. `blocks/error.rs` - ErrorBlock

### Phase 3: 命令集成 (高优)

1. 改造 `commands/deploy.rs`
2. 改造 `commands/check.rs`
3. 改造 `commands/watch.rs`

### Phase 4: 交互增强 (可选)

1. `blocks/check_item.rs` - CheckItem
2. 改造 `commands/interactive.rs` banner
3. Setup wizard 视觉增强

---

## 九、相关文档

- [设计原则](./design-principles.md) - 设计约束和原则
- [产品反思](./product-reflection.md) - 用户场景分析
- [TODO](./TODO.md) - 实施任务清单
