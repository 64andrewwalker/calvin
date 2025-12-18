# Calvin CLI 动画系统产品反思

> **核心理念**: 动画让等待变得可预期，特效让成功变得有仪式感  
> **日期**: 2025-12-18  
> **版本**: v0.3.0 规划  
> **对标**: Claude Code, Cursor, GitHub CLI

---

## 一、当前状态分析

### 1.1 现有实现的不足

**问题 1: 静态输出体验**

```
当前:
  $ calvin deploy
  Syncing 12 prompts...
  [OK] .claude/commands/review.md
  [OK] .claude/commands/test.md
  [OK] .cursor/commands/review.md
  ... (等待所有完成后一次性输出)
  
  Done: 12 files synced
```

这种输出方式的问题：
- 用户不知道当前进度
- 无法感知操作是否卡住
- 缺乏视觉层次区分

**问题 2: 错误信息缺乏视觉区分**

```
当前:
  [ERROR] .promptpack/actions/test.md
          Line 2: missing required field 'description'

  FIX: Add 'description' to the frontmatter
```

虽然信息完整，但：
- 与正常输出视觉区分不足
- 严重错误和轻微警告看起来相似
- 缺少引导用户下一步行动的视觉提示

**问题 3: 交互组件基础**

当前使用 `dialoguer` 提供基础交互：
- MultiSelect 用于目标选择
- 缺少进度条、spinner 等动态组件
- 无实时更新能力

### 1.2 与 Claude Code 的差距

| 功能 | Claude Code | Calvin 当前 | 差距 |
|-----|-------------|------------|-----|
| 流式输出 | ✅ 实时显示 | ❌ 批量输出 | 🔴 |
| Spinner 动画 | ✅ 多种样式 | ❌ 无 | 🔴 |
| 进度条 | ✅ 详细进度 | ❌ 无 | 🔴 |
| 颜色主题 | ✅ 语义化颜色 | ⚠️ 基础颜色 | 🟡 |
| 错误可视化 | ✅ 框线+高亮 | ⚠️ 基础格式 | 🟡 |
| 成功仪式 | ✅ 动画效果 | ❌ 简单文本 | 🔴 |
| 输入提示 | ✅ 丰富交互 | ⚠️ 基础交互 | 🟡 |
| SSH 传输进度 | ✅ 详细进度 | ❌ 无进度显示 | 🔴 |
| 统一设计风格 | ✅ 一致性 | ⚠️ 风格混杂 | 🟡 |

### 1.3 当前设计风格问题

**问题 4: 视觉元素不统一**

当前输出中存在的问题：
- 边框样式混用：`+------+` 和 `╭──────╮` 混在一起
- 颜色使用随意：没有统一的调色板
- 图标不一致：有的用 `✓`，有的用 `[OK]`
- 缩进层级混乱：没有固定的缩进规则

**目标：统一的视觉语言**

```
设计约束:

1. 边框统一使用圆角: ╭ ╮ ╰ ╯ │ ─
2. 颜色仅用 5 种: 绿(成功) 红(错误) 黄(警告) 青(信息) 灰(次要)
3. 图标统一: ✓ ✗ ⚠ ● ○ ↳
4. 缩进规则: 0/2/4/6 空格加深
5. 所有 UI 代码集中到 src/ui/
```


---

## 二、什么时候需要这些特效

### 2.1 操作类型与视觉需求矩阵

```
+------------------+----------+----------+----------+----------+-----------+
| 操作              | Spinner  | 进度条   | 流式输出  | 成功动画  | SSH进度   |
+------------------+----------+----------+----------+----------+-----------+
| calvin deploy    | ✓ 编译时 | ✓ 写入时 | ✓ 逐文件  | ✓ 完成时  | -         |
| calvin deploy -r | ✓ 连接时 | ✓ 传输时 | ✓ 逐文件  | ✓ 完成时  | ✓ 必须    |
| calvin check     | ✓ 扫描时 | ✓ 检查时 | ✓ 逐项目  | ✓ 通过时  | -         |
| calvin watch     | ✓ 监听中 | ○        | ✓ 变更时  | ○         | -         |
| calvin explain   | ○        | ○        | ✓ 长文本  | ○         | -         |
| calvin diff      | ○        | ○        | ✓ 差异块  | ○         | -         |
+------------------+----------+----------+----------+----------+-----------+

图例: ✓ 必需  ○ 可选/不需要  - 不适用
```


### 2.2 用户场景详细分析

#### 场景 A: 首次部署 (calvin deploy)

```
用户期望:
  1. 知道工具正在工作 (不是卡住了)
  2. 看到处理进度 (12个文件处理了多少)
  3. 实时看到每个文件的状态
  4. 最终得到清晰的成功/失败总结

当前体验:
  $ calvin deploy
  (等待 2-3 秒，没有任何输出)
  [OK] file1.md
  [OK] file2.md
  ... (一次性输出)
  Done.

目标体验:
  $ calvin deploy
  
  Compiling 12 prompts...
  
    ✓ actions/review.md → .claude/, .cursor/
    ✓ actions/test.md → .claude/, .cursor/
    ● policies/style.md → ...            ← 当前正在处理
    ○ agents/helper.md                   ← 待处理
    ○ ...
  
  ━━━━━━━━━━━━━━━━━━━━━━━━━  8/12 (66%)
  
  (处理完成后)
  
  ╭─────────────────────────────────────╮
  │  ✓ Deploy Complete                  │
  │                                     │
  │  12 prompts → 3 targets             │
  │  24 files written                   │
  │  0 warnings, 0 errors               │
  ╰─────────────────────────────────────╯
```

#### 场景 B: 配置检查 (calvin check)

```
当前体验:
  $ calvin check
  Checking configuration...
  (等待)
  All checks passed.

目标体验:
  $ calvin check
  
  Running health checks...
  
    ⠋ Project structure...
    
  (变成)
  
    ✓ Project structure        .promptpack/ found
    ✓ Configuration            12 prompts, 3 targets
    ✓ Security settings        balanced mode, deny list active
    ● MCP servers              checking allowlist...
    ○ Target compatibility
  
  (完成后)
  
    ✓ All 5 checks passed
    
    TIP: Run 'calvin deploy' to sync your prompts.
```

#### 场景 C: 监听模式 (calvin watch)

```
当前体验:
  $ calvin watch
  Watching for changes...
  (文件变更时)
  Synced: actions/review.md

目标体验:
  $ calvin watch
  
  ⟳ Watching .promptpack/ for changes...
    Press Ctrl+C to stop
  
  ─────────────────────────────────────────
  
  [14:32:05] ⠋ Detected change: actions/review.md
  [14:32:05] ✓ Synced to .claude/, .cursor/
  
  [14:35:12] ⠋ Detected change: policies/style.md
  [14:35:12] ✓ Synced to .claude/, .cursor/, .github/
  
  ─────────────────────────────────────────
  
  ^C
  
  ✓ Watch stopped. Synced 2 changes.
```

#### 场景 D: 错误处理

```
当前体验:
  [ERROR] .promptpack/actions/test.md
          Line 2: missing required field 'description'
  
  FIX: Add 'description' to the frontmatter

目标体验:
  
  ┌─ ERROR ───────────────────────────────────────────────────┐
  │                                                            │
  │  📄 .promptpack/actions/test.md                            │
  │                                                            │
  │  Line 2: missing required field 'description'              │
  │                                                            │
  │  Your file:                                                │
  │  ┌──────────────────────────────────────────────────────┐  │
  │  │  1 │ ---                                             │  │
  │  │  2 │ targets: [claude-code]                          │  │
  │  │    │ ↑ 'description' is required                     │  │
  │  │  3 │ ---                                             │  │
  │  └──────────────────────────────────────────────────────┘  │
  │                                                            │
  │  FIX: Add 'description' to line 2:                         │
  │       description: "Your description here"                 │
  │                                                            │
  │  ╭────────────────────────────────────────────────────╮    │
  │  │ Press [Enter] to open in editor, [s] to skip       │    │
  │  ╰────────────────────────────────────────────────────╯    │
  │                                                            │
  └────────────────────────────────────────────────────────────┘
```

#### 场景 E: SSH 远程部署 (calvin deploy --remote)

```
当前体验:
  $ calvin deploy --remote user@server
  Syncing to remote...
  (长时间等待，无任何反馈)
  Done.

目标体验:
  $ calvin deploy --remote user@server.example.com
  
  ╭─────────────────────────────────────────────────────────╮
  │  📤 Remote Deploy                                        │
  │                                                          │
  │  Target: user@server.example.com                         │
  │  Path:   /home/user/project/                             │
  ╰─────────────────────────────────────────────────────────╯
  
  ⠋ Connecting via SSH...
  ✓ Connected (key: ~/.ssh/id_ed25519)
  
  Uploading 24 files...
  
    ✓ .claude/commands/review.md         2.1 KB
    ✓ .claude/commands/test.md           1.8 KB
    ✓ .claude/settings.json              0.5 KB
    ● .cursor/rules/style.md             uploading...
    ○ .cursor/commands/review.md
    ○ ...
  
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━  12/24 (50%)
  Speed: 1.2 MB/s  |  ETA: 3s
  
  (传输完成)
  
  ╭─────────────────────────────────────────────────────────╮
  │  ✓ Remote Deploy Complete                                │
  │                                                          │
  │  24 files uploaded                                       │
  │  Total: 48.2 KB in 8s                                    │
  │                                                          │
  │  Verify: ssh user@server 'ls -la .claude/'               │
  ╰─────────────────────────────────────────────────────────╯

连接失败处理:
  $ calvin deploy --remote user@server
  
  ⠋ Connecting via SSH...
  ✗ Connection failed
  
  ╭─ ERROR ─────────────────────────────────────────────────╮
  │                                                          │
  │  SSH connection to user@server failed                    │
  │                                                          │
  │  Reason: Permission denied (publickey)                   │
  │                                                          │
  │  FIX: Ensure your SSH key is loaded:                     │
  │       ssh-add ~/.ssh/id_ed25519                          │
  │                                                          │
  │  Or test connection manually:                            │
  │       ssh user@server 'echo ok'                          │
  │                                                          │
  ╰─────────────────────────────────────────────────────────╯

网络中断恢复:
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━  12/24 (50%)
  
  ⚠ Connection interrupted
  ⠋ Reconnecting (1/3)...
  ✓ Reconnected, resuming upload...
  
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━  12/24 (50%)
```


---

## 三、技术实现方案

### 3.1 渲染架构

```
+------------------------------------------------------------------+
|                        Application Layer                          |
|  (calvin deploy, calvin check, calvin watch, ...)                |
+------------------------------------------------------------------+
                              |
                              v
+------------------------------------------------------------------+
|                         UI Layer (新增)                           |
|                                                                   |
|  +---------------+  +---------------+  +-------------------+      |
|  | Spinner       |  | ProgressBar   |  | StreamOutput      |      |
|  | Component     |  | Component     |  | Component         |      |
|  +---------------+  +---------------+  +-------------------+      |
|                                                                   |
|  +---------------+  +---------------+  +-------------------+      |
|  | ErrorBox      |  | SuccessBlock  |  | InteractivePrompt |      |
|  | Component     |  | Component     |  | Component         |      |
|  +---------------+  +---------------+  +-------------------+      |
|                                                                   |
+------------------------------------------------------------------+
                              |
                              v
+------------------------------------------------------------------+
|                       Render Engine (新增)                        |
|                                                                   |
|  +----------------+  +----------------+  +-----------------+      |
|  | TerminalState  |  | FrameScheduler |  | OutputManager   |      |
|  | (cursor, size) |  | (30fps cap)    |  | (buffer/flush)  |      |
|  +----------------+  +----------------+  +-----------------+      |
|                                                                   |
+------------------------------------------------------------------+
                              |
                              v
+------------------------------------------------------------------+
|                    Terminal Abstraction (crossterm)               |
|                                                                   |
|  ANSI Control Sequences | Cursor Management | Color Support       |
+------------------------------------------------------------------+
```

### 3.2 核心组件设计

#### Spinner 组件

```rust
pub struct Spinner {
    frames: Vec<char>,
    current: usize,
    message: String,
    speed: Duration,
    style: SpinnerStyle,
}

pub enum SpinnerStyle {
    Braille,      // ⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏
    Dots,         // ⣾⣽⣻⢿⡿⣟⣯⣷
    Line,         // -\|/
    Arrow,        // ←↖↑↗→↘↓↙
    Bounce,       // ⠁⠂⠄⠂
}

impl Spinner {
    pub fn new(message: &str) -> Self;
    pub fn tick(&mut self);
    pub fn succeed(self, message: &str);
    pub fn fail(self, message: &str);
    pub fn stop(self);
}
```

#### ProgressBar 组件

```rust
pub struct ProgressBar {
    total: u64,
    current: u64,
    message: String,
    style: ProgressStyle,
    start_time: Instant,
}

pub enum ProgressStyle {
    Bar,          // ━━━━━━━━━━━━━━━━━━━━ 50%
    Blocks,       // ████████████░░░░░░░░ 50%
    Compact,      // [========>          ] 50%
}

impl ProgressBar {
    pub fn new(total: u64) -> Self;
    pub fn inc(&mut self, delta: u64);
    pub fn set_message(&mut self, msg: &str);
    pub fn eta(&self) -> Duration;
    pub fn finish(self);
    pub fn abandon(self);
}
```

#### StreamOutput 组件

```rust
pub struct StreamOutput {
    items: Vec<StreamItem>,
    visible_count: usize,
    auto_scroll: bool,
}

pub struct StreamItem {
    pub status: ItemStatus,
    pub label: String,
    pub detail: Option<String>,
}

pub enum ItemStatus {
    Pending,      // ○
    InProgress,   // ●
    Success,      // ✓
    Warning,      // ⚠
    Error,        // ✗
}

impl StreamOutput {
    pub fn new() -> Self;
    pub fn add_item(&mut self, label: &str);
    pub fn update_item(&mut self, index: usize, status: ItemStatus);
    pub fn render(&self) -> String;
}
```

### 3.3 渲染循环设计

```rust
/// 主渲染循环：处理动画帧和用户输入
pub struct RenderLoop {
    frame_rate: u32,  // 通常 30fps
    terminal: Terminal,
    components: Vec<Box<dyn Renderable>>,
}

impl RenderLoop {
    pub async fn run(&mut self) {
        let frame_duration = Duration::from_millis(1000 / self.frame_rate as u64);
        
        loop {
            let frame_start = Instant::now();
            
            // 1. 处理输入事件
            if let Some(event) = self.poll_input() {
                if self.handle_event(event) == ControlFlow::Break {
                    break;
                }
            }
            
            // 2. 更新所有组件状态
            for component in &mut self.components {
                component.tick();
            }
            
            // 3. 渲染帧
            self.render_frame();
            
            // 4. 帧率控制
            let elapsed = frame_start.elapsed();
            if elapsed < frame_duration {
                tokio::time::sleep(frame_duration - elapsed).await;
            }
        }
    }
}
```

### 3.4 与现有代码集成

**修改 src/ui/mod.rs:**

```rust
pub mod animation;    // 新增：动画组件
pub mod components;   // 新增：UI组件
pub mod error;        // 现有
pub mod menu;         // 现有
pub mod output;       // 现有
pub mod render;       // 新增：渲染引擎

// 重新导出常用组件
pub use animation::{Spinner, SpinnerStyle};
pub use components::{ProgressBar, StreamOutput, ErrorBox, SuccessBlock};
pub use render::RenderLoop;
```

**修改 Cargo.toml:**

```toml
[dependencies]
# 现有依赖保持不变

# 新增动画相关依赖
crossterm = "0.28"           # 终端控制
indicatif = "0.17"           # 进度条 (可选，或自行实现)
owo-colors = "4"             # 颜色 (更轻量)
unicode-width = "0.2"        # Unicode 宽度计算
```

---

## 四、重构优先级

### 4.1 当前做得不完美的地方

| 问题 | 位置 | 严重程度 | 重构优先级 |
|-----|------|---------|-----------|
| 批量输出无实时反馈 | `sync_outputs()` | 🔴 高 | P0 |
| 错误格式化简陋 | `ui/error.rs` | 🟡 中 | P1 |
| 无进度指示 | 所有长操作 | 🔴 高 | P0 |
| 警告视觉区分不足 | `ui/output.rs` | 🟡 中 | P1 |
| 成功提示无仪式感 | 所有命令 | 🟢 低 | P2 |
| 交互组件样式基础 | `ui/menu.rs` | 🟡 中 | P1 |
| 无 CI 适配输出 | 全局 | 🔴 高 | P0 |

### 4.2 重构路径

```
Phase 0: 基础设施 (1周)
├── 添加 crossterm 依赖
├── 创建 ui/render.rs 渲染引擎
├── 创建 ui/animation.rs Spinner 组件
├── 添加终端检测逻辑 (TTY, CI, NO_COLOR)
└── 添加配置项 output.color, output.animation

Phase 1: 核心动画 (1周)
├── 实现 Spinner 组件
├── 实现 ProgressBar 组件
├── 将 calvin deploy 改为流式输出
├── 将 calvin check 改为流式输出
└── 添加 Ctrl+C 优雅取消

Phase 2: 视觉增强 (1周)
├── 实现 ErrorBox 组件
├── 实现 SuccessBlock 组件
├── 重构 ui/error.rs 使用新组件
├── 添加颜色主题支持
└── 添加 ASCII 降级模式

Phase 3: 交互增强 (1周)
├── 实现增强的交互式菜单
├── 添加 diff 高亮显示
├── 实现 watch 模式实时更新
├── 添加 CI 输出模式
└── 性能优化和测试
```

---

## 五、具体交互流程设计

### 5.1 calvin deploy 完整流程

```
$ calvin deploy

  ⠋ Scanning .promptpack/...
  
(扫描完成)

  Found 12 prompts, 2 policies, 1 agent
  
  Targets: Claude Code, Cursor, VS Code
  
  ⠋ Compiling prompts...

(编译过程 - 流式输出)

  actions/
    ✓ review.md      → .claude/, .cursor/, .github/
    ✓ test.md        → .claude/, .cursor/, .github/
    ● refactor.md    → compiling...
    ○ docs.md
    ○ debug.md
  
  policies/
    ○ style.md
    ○ security.md
  
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━  5/15 (33%)
  ETA: 2s

(全部完成)

  ╭─────────────────────────────────────────────────╮
  │                                                 │
  │   ✓ Deploy Complete                             │
  │                                                 │
  │   15 prompts compiled                           │
  │   45 files written to 3 targets                 │
  │                                                 │
  │   ⚠ 1 warning (use --verbose to see details)   │
  │                                                 │
  ╰─────────────────────────────────────────────────╯

  Next: Run 'calvin check' to verify configuration.
```

### 5.2 calvin check 完整流程

```
$ calvin check

  ⠋ Running health checks...

(检查过程)

  Project Structure
    ✓ .promptpack/ directory exists
    ✓ config.toml is valid
    ✓ 15 prompts parsed successfully
  
  Security
    ✓ Security mode: balanced
    ✓ Deny list: 8 patterns active
    ⚠ MCP servers: 2 unchecked (see below)
  
  Target Compatibility
    ● Checking Claude Code...
    ○ Checking Cursor...
    ○ Checking VS Code...

(完成)

  ─────────────────────────────────────────────────

  Summary: 8 passed, 1 warning, 0 errors

  ⚠ WARNING: 2 MCP servers not in allowlist
  
    • my-internal-server
    • experimental-tool
  
    FIX: Add to config.toml:
         [security.mcp]
         additional_allowlist = ["my-internal-server"]
  
  ─────────────────────────────────────────────────
```

### 5.3 交互式菜单增强

```
$ calvin

  ╭─────────────────────────────────────────────────╮
  │                                                 │
  │   Calvin - Making AI agents behave              │
  │                                                 │
  │   Found .promptpack/ with 15 prompts            │
  │   Last deployed: 2 hours ago                    │
  │                                                 │
  ╰─────────────────────────────────────────────────╯

  What would you like to do?

  > [1] 🚀 Deploy to this project
    [2] 🏠 Deploy to home directory
    [3] 📋 Preview changes (diff)
    [4] 🔍 Check configuration
    [5] 👁 Watch mode
    [6] ❓ Explain Calvin (for AI)
    
    [q] Quit

  Use ↑↓ to navigate, Enter to select
```

---

## 六、成功指标

### 用户体验指标

| 指标 | 当前 | 目标 |
|-----|------|------|
| 感知响应时间 | 即时（批量输出） | <100ms 首次反馈 |
| 操作进度可见性 | 无 | 100% 长操作有进度 |
| 错误可理解性 | 基础 | 视觉突出 + 上下文 |
| 成功确认感 | 简单文本 | 有成就感的反馈 |
| CI 集成友好度 | 需要解析文本 | 结构化输出 |

### 技术指标

| 指标 | 目标 |
|-----|------|
| 动画帧率 | 稳定 30fps |
| CPU 使用 (动画中) | <5% |
| 二进制大小增加 | <500KB |
| 测试覆盖率 | 80% (UI 组件) |
| 终端兼容性 | 99% 现代终端 |

---

## 七、附录

### A. 参考实现

1. **Claude Code CLI** - 流式输出、spinner 动画、颜色主题
2. **GitHub CLI (gh)** - 进度条、表格输出、CI 适配
3. **Rust indicatif** - 多进度条、模板系统
4. **Charm CLI tools** - bubbletea 的 Rust 移植思路

### B. 终端兼容性矩阵

| 功能 | macOS Terminal | iTerm2 | Windows Terminal | VS Code | SSH |
|-----|---------------|--------|------------------|---------|-----|
| ANSI 颜色 | ✓ | ✓ | ✓ | ✓ | ✓ |
| 256 色 | ✓ | ✓ | ✓ | ✓ | 取决于客户端 |
| True Color | ✓ | ✓ | ✓ | ✓ | 取决于客户端 |
| Unicode | ✓ | ✓ | ✓ | ✓ | ✓ |
| 光标控制 | ✓ | ✓ | ✓ | ✓ | ✓ |
| 交替屏幕 | ✓ | ✓ | ✓ | ✓ | ✓ |

---

*"最好的 CLI 体验是让用户感觉在和一个响应迅速的助手对话，而不是在执行冰冷的命令。"*
