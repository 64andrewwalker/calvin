# Calvin CLI 动画系统 TODO

> **目标版本**: v0.3.0  
> **开始日期**: 2025-12-18  
> **状态**: Phase 0 待开始  
> **对标**: Claude Code CLI 体验

---

## 重构进度概览

```
+==============================================================+
|  Phase 0: 基础设施                              ⬜ 待开始    |
|  Phase 1: 核心动画                              ⬜ 待开始    |
|  Phase 2: 视觉增强                              ⬜ 待开始    |
|  Phase 3: 交互增强                              ⬜ 待开始    |
+==============================================================+
```

---

## 当前需要重构的位置

### 代码位置与问题

| 文件 | 行数 | 问题 | 重构方向 |
|-----|------|-----|---------|
| `src/sync/mod.rs` | ~250行 | 批量输出无实时反馈 | 接入 StreamOutput |
| `src/ui/error.rs` | 194行 | 错误格式化简陋 | 使用 ErrorBox 组件 |
| `src/ui/output.rs` | 28行 | 警告视觉区分不足 | 添加颜色和图标 |
| `src/ui/menu.rs` | 162行 | 交互组件样式基础 | 增强视觉效果 |
| `src/commands/deploy.rs` | ~329行 | 无进度指示 | 添加 ProgressBar |
| `src/commands/check.rs` | ~260行 | 无进度指示 | 添加 StreamOutput |
| `src/watcher.rs` | ~300行 | watch 输出静态 | 添加实时更新 |
| `src/fs.rs` (远程同步) | - | SSH传输无进度 | 添加传输进度回调 |
| 全局输出风格 | - | 边框/颜色/图标不统一 | 创建 theme.rs |

---

## Phase 0: 基础设施 ⬜ 待开始

> **预计时间**: 3-5 天  
> **目标**: 建立动画系统的技术基础

### 0.1 依赖管理 ⬜

- [ ] 在 Cargo.toml 添加 `crossterm = "0.28"`
- [ ] 在 Cargo.toml 添加 `owo-colors = "4"`
- [ ] 在 Cargo.toml 添加 `unicode-width = "0.2"`
- [ ] 验证二进制大小增加 <200KB

### 0.2 终端检测 ⬜

- [ ] 创建 `src/ui/terminal.rs`
- [ ] 实现 `TerminalCapabilities` 结构体
  ```rust
  pub struct TerminalCapabilities {
      pub is_tty: bool,
      pub supports_color: bool,
      pub supports_256_color: bool,
      pub supports_true_color: bool,
      pub supports_unicode: bool,
      pub is_ci: bool,
      pub width: u16,
      pub height: u16,
  }
  ```
- [ ] 实现 `detect_capabilities()` 函数
- [ ] 检测 NO_COLOR 环境变量
- [ ] 检测 CI 环境 (CI, GITHUB_ACTIONS, JENKINS, etc.)
- [ ] 检测 TERM=dumb
- [ ] 测试覆盖 (4+ 测试)

### 0.3 配置扩展 ⬜

- [ ] 在 `src/config.rs` 添加 OutputConfig 结构体
  ```toml
  [output]
  color = "auto"       # auto | always | never
  animation = "auto"   # auto | always | never | minimal
  unicode = true       # 允许 Unicode 字符
  ```
- [ ] 实现配置解析
- [ ] 添加 CLI 参数 `--color`, `--no-animation`
- [ ] 测试覆盖

### 0.4 渲染引擎基础 ⬜

- [ ] 创建 `src/ui/render.rs`
- [ ] 实现 `TerminalState` 管理
  - 光标位置
  - 终端尺寸
  - 输出缓冲
- [ ] 实现 `flush()` 写入终端
- [ ] 实现 `clear_line()` / `move_cursor()` 基础操作

### 0.5 创建 theme.rs (设计约束 DC-2, DC-3) ⬜

- [ ] 创建 `src/ui/theme.rs`
- [ ] 定义 5 种精选颜色常量
  ```rust
  pub mod colors {
      pub const SUCCESS: Color = Color::Green;   // #22C55E
      pub const ERROR: Color = Color::Red;       // #EF4444
      pub const WARNING: Color = Color::Yellow;  // #F59E0B
      pub const INFO: Color = Color::Cyan;       // #06B6D4
      pub const DIM: Color = Color::DarkGray;    // #6B7280
  }
  ```
- [ ] 定义状态图标常量
  ```rust
  pub mod icons {
      pub const SUCCESS: &str = "✓";
      pub const ERROR: &str = "✗";
      pub const WARNING: &str = "⚠";
      pub const PROGRESS: &str = "●";
      pub const PENDING: &str = "○";
      pub const ARROW: &str = "↳";
  }
  ```
- [ ] 定义边框字符常量
  ```rust
  pub mod borders {
      pub const TOP_LEFT: &str = "╭";
      pub const TOP_RIGHT: &str = "╮";
      pub const BOTTOM_LEFT: &str = "╰";
      pub const BOTTOM_RIGHT: &str = "╯";
      pub const HORIZONTAL: &str = "─";
      pub const VERTICAL: &str = "│";
  }
  ```
- [ ] 定义 ASCII fallback 常量
- [ ] 所有现有代码必须使用 theme.rs 中的常量

### 0.6 验证 ⬜

- [ ] `cargo build --release` 成功
- [ ] 二进制大小增加 <200KB
- [ ] `cargo test` 全部通过
- [ ] `cargo clippy` 无警告
- [ ] theme.rs 中的常量被正确导出

---

## Phase 1: 核心动画 ⬜ 待开始

> **预计时间**: 5-7 天  
> **目标**: 实现 Spinner 和 ProgressBar，改造核心命令

### 1.1 Spinner 组件 ⬜

- [ ] 创建 `src/ui/animation/spinner.rs`
- [ ] 实现 `Spinner` 结构体
- [ ] 实现 `SpinnerStyle` 枚举 (Braille, Dots, Line, Arrow)
- [ ] 实现 `tick()` 方法 (帧更新)
- [ ] 实现 `succeed()` / `fail()` / `stop()` 方法
- [ ] 支持自定义消息更新
- [ ] Fallback ASCII 模式 (无 Unicode)
- [ ] 测试覆盖 (3+ 测试)

### 1.2 ProgressBar 组件 ⬜

- [ ] 创建 `src/ui/animation/progress.rs`
- [ ] 实现 `ProgressBar` 结构体
- [ ] 实现 `ProgressStyle` 枚举 (Bar, Blocks, Compact)
- [ ] 实现 `inc()` / `set()` 方法
- [ ] 实现 ETA 计算
- [ ] 实现 `finish()` / `abandon()` 方法
- [ ] 自动宽度适应终端
- [ ] Fallback 简单计数模式
- [ ] 测试覆盖 (3+ 测试)

### 1.3 StreamOutput 组件 ⬜

- [ ] 创建 `src/ui/components/stream.rs`
- [ ] 实现 `StreamOutput` 结构体
- [ ] 实现 `StreamItem` 结构体 (status, label, detail)
- [ ] 实现 `ItemStatus` 枚举 (Pending, InProgress, Success, Warning, Error)
- [ ] 实现实时更新渲染
- [ ] 支持滚动区域 (限制可见行数)
- [ ] 测试覆盖 (3+ 测试)

### 1.4 改造 calvin deploy ⬜

- [ ] 在 `src/commands/deploy.rs` 集成 Spinner
- [ ] 扫描阶段：显示 spinner + "Scanning .promptpack/..."
- [ ] 编译阶段：使用 StreamOutput 逐文件显示
- [ ] 写入阶段：使用 ProgressBar 显示进度
- [ ] 完成阶段：显示成功摘要
- [ ] 集成测试

### 1.5 改造 calvin check ⬜

- [ ] 在 `src/commands/check.rs` 集成 Spinner
- [ ] 使用 StreamOutput 显示检查项
- [ ] 每个检查项实时更新状态
- [ ] 显示最终摘要
- [ ] 集成测试

### 1.6 Ctrl+C 优雅取消 ⬜

- [ ] 在动画组件中添加取消检测
- [ ] 实现取消时的清理逻辑
- [ ] 显示 "Cancelling..." 提示
- [ ] 确保终端状态正确恢复

### 1.7 验证 ⬜

- [ ] `cargo test` 全部通过
- [ ] 手动测试 `calvin deploy` 流式输出
- [ ] 手动测试 `calvin check` 流式输出
- [ ] 测试 Ctrl+C 取消
- [ ] 测试 CI 模式 (CI=true)

---

## Phase 1.5: SSH 传输进度 ⬜ 待开始

> **依赖**: Phase 1 完成  
> **目标**: 为远程同步添加详细进度反馈

### 1.5.1 SSH 连接进度 ⬜

- [ ] 在 `src/fs.rs` 中添加进度回调接口
- [ ] 连接阶段显示 spinner
- [ ] 认证成功显示使用的密钥
- [ ] 连接失败显示详细错误和修复建议

### 1.5.2 文件传输进度 ⬜

- [ ] 传输时显示当前文件名和大小
- [ ] 显示总进度条 (x/n 文件)
- [ ] 显示传输速度 (KB/s 或 MB/s)
- [ ] 显示预估剩余时间 (ETA)
- [ ] 文件列表使用 StreamOutput 组件

### 1.5.3 错误和恢复 ⬜

- [ ] 网络中断显示重连状态 (1/3)
- [ ] 重连成功后继续传输
- [ ] 最终失败显示详细原因
- [ ] 提供 SSH 调试命令建议

### 1.5.4 完成汇总 ⬜

- [ ] 显示上传文件数和总大小
- [ ] 显示用时和平均速度
- [ ] 提供验证命令建议

---

## Phase 2: 视觉增强 ⬜ 待开始

> **预计时间**: 5-7 天  
> **目标**: 增强错误和成功的视觉表现

### 2.1 ErrorBox 组件 ⬜

- [ ] 创建 `src/ui/components/error_box.rs`
- [ ] 实现带边框的错误显示
  ```
  ┌─ ERROR ─────────────────────────────────────┐
  │  文件路径                                    │
  │  错误描述                                    │
  │  代码预览 (带行号和高亮)                      │
  │  修复建议                                    │
  └─────────────────────────────────────────────┘
  ```
- [ ] 支持代码片段高亮
- [ ] 支持行号显示
- [ ] 支持错误位置指示 (↑ 箭头)
- [ ] ASCII 降级模式

### 2.2 SuccessBlock 组件 ⬜

- [ ] 创建 `src/ui/components/success.rs`
- [ ] 实现成功摘要框
  ```
  ╭─────────────────────────────────────╮
  │  ✓ Deploy Complete                  │
  │                                     │
  │  12 prompts → 3 targets             │
  │  0 warnings, 0 errors               │
  ╰─────────────────────────────────────╯
  ```
- [ ] 支持多种成功级别 (simple, detailed)
- [ ] 支持包含下一步提示

### 2.3 WarningBlock 组件 ⬜

- [ ] 创建 `src/ui/components/warning.rs`
- [ ] 实现警告显示 (与错误区分)
- [ ] 黄色主题
- [ ] 支持分组显示多个警告

### 2.4 重构 ui/error.rs ⬜

- [ ] 使用 ErrorBox 替换现有格式化
- [ ] 保持 JSON 输出不变
- [ ] 添加代码上下文显示 (读取文件内容)
- [ ] 测试覆盖

### 2.5 颜色主题支持 ⬜

- [ ] 定义语义化颜色常量
  ```rust
  pub struct Theme {
      pub success: Color,
      pub error: Color,
      pub warning: Color,
      pub info: Color,
      pub muted: Color,
  }
  ```
- [ ] 实现默认主题
- [ ] 支持通过配置自定义

### 2.6 ASCII 降级模式 ⬜

- [ ] 所有组件支持 ASCII fallback
- [ ] Unicode 字符有对应 ASCII 替代
- [ ] 边框使用 +--+| 替代
- [ ] 测试无 Unicode 环境

### 2.7 验证 ⬜

- [ ] 手动测试错误显示效果
- [ ] 手动测试成功显示效果
- [ ] 测试 ASCII 模式 (TERM=dumb)
- [ ] 测试 NO_COLOR 模式

---

## Phase 3: 交互增强 ⬜ 待开始

> **预计时间**: 5-7 天  
> **目标**: 增强交互体验和特殊模式

### 3.1 增强交互式菜单 ⬜

- [ ] 在 `ui/menu.rs` 添加视觉增强
- [ ] 添加 emoji 图标 (可选)
- [ ] 添加选中项高亮
- [ ] 添加键盘提示

### 3.2 Diff 高亮显示 ⬜

- [ ] 创建 `src/ui/components/diff.rs`
- [ ] 实现 unified diff 格式
- [ ] 红色显示删除行
- [ ] 绿色显示添加行
- [ ] 灰色显示上下文行
- [ ] 行号对齐

### 3.3 Watch 模式增强 ⬜

- [ ] 在 `src/watcher.rs` 添加实时更新
- [ ] 显示持续运行的 spinner
- [ ] 变更时显示时间戳
- [ ] 变更后显示同步结果
- [ ] 清晰的退出提示

### 3.4 CI 输出模式 ⬜

- [ ] 检测 CI 环境自动切换
- [ ] 无动画，纯文本输出
- [ ] 阶段性进度报告
- [ ] GitHub Actions 注解格式支持
  ```
  ::error file=path/to/file.md,line=3::Error message
  ::warning file=path/to/file.md,line=5::Warning message
  ```

### 3.5 JSON 输出增强 ⬜

- [ ] 所有命令支持 `--json` 输出
- [ ] 事件流格式 (NDJSON)
  ```json
  {"event": "start", "command": "deploy"}
  {"event": "progress", "current": 5, "total": 12}
  {"event": "success", "file": "actions/review.md"}
  {"event": "complete", "files": 12, "warnings": 0}
  ```

### 3.6 性能优化 ⬜

- [ ] 帧率限制 (30fps cap)
- [ ] 减少不必要的终端写入
- [ ] 批量更新优化
- [ ] CPU 使用监控 (<5%)

### 3.7 文档更新 ⬜

- [ ] 更新 README 展示新功能
- [ ] 添加 GIF 动画演示
- [ ] 更新 CHANGELOG
- [ ] 更新 explain 命令输出

### 3.8 验证 ⬜

- [ ] `cargo test` 全部通过
- [ ] 手动测试所有命令
- [ ] 测试 CI 模式
- [ ] 性能测试 (CPU 使用)
- [ ] 终端兼容性测试

---

## 进度追踪

| Phase | 任务数 | 完成 | 状态 |
|-------|--------|------|------|
| 0. 基础设施 | 25 | 0 | ⬜ 待开始 |
| 1. 核心动画 | 35 | 0 | ⬜ 待开始 |
| 1.5 SSH进度 | 15 | 0 | ⬜ 待开始 |
| 2. 视觉增强 | 30 | 0 | ⬜ 待开始 |
| 3. 交互增强 | 30 | 0 | ⬜ 待开始 |
| **总计** | **135** | **0** | **0%** |

---

## 验证清单 (Final)

### 设计约束检查（强制）

- [ ] theme.rs 存在并包含所有颜色/图标常量
- [ ] 仅使用 5 种精选颜色
- [ ] 所有 UI 代码在 src/ui/ 目录下
- [ ] 边框样式统一使用圆角
- [ ] 状态图标使用 theme.rs 常量

### 功能检查

- [ ] `cargo test` 全部通过
- [ ] `cargo clippy` 无警告
- [ ] 二进制大小增加 <200KB
- [ ] `calvin deploy` 有流式输出
- [ ] `calvin deploy --remote` 有 SSH 进度
- [ ] `calvin check` 有流式输出
- [ ] `calvin watch` 有实时更新
- [ ] 错误显示有边框和高亮
- [ ] 成功显示有摘要框
- [ ] Ctrl+C 能优雅取消
- [ ] CI 模式无动画
- [ ] NO_COLOR 模式无颜色
- [ ] TERM=dumb 模式 ASCII 降级

---

## 新增文件列表

```
src/ui/
├── mod.rs              (修改: 添加导出)
├── theme.rs            (新增: 设计令牌 - 颜色/图标/边框)
├── terminal.rs         (新增: 终端能力检测)
├── render.rs           (新增: 渲染引擎)
│
├── primitives/         (新增: 原子组件层)
│   ├── mod.rs
│   ├── text.rs         # 带颜色的文本
│   ├── icon.rs         # 状态图标
│   └── border.rs       # 边框字符
│
├── widgets/            (新增: 基础组件层)
│   ├── mod.rs
│   ├── spinner.rs      # Spinner 加载动画
│   ├── progress.rs     # ProgressBar 进度条
│   ├── list.rs         # StatusList 状态列表
│   └── box.rs          # Box 边框容器
│
├── blocks/             (新增: 复合组件层)
│   ├── mod.rs
│   ├── header.rs       # CommandHeader 命令头部
│   ├── summary.rs      # ResultSummary 结果摘要
│   ├── error.rs        # ErrorBlock 错误展示
│   └── check_item.rs   # CheckItem 检查项
│
└── views/              (新增: 视图层)
    ├── mod.rs
    ├── deploy.rs       # Deploy 命令视图
    ├── check.rs        # Check 命令视图
    ├── watch.rs        # Watch 命令视图
    └── interactive.rs  # Interactive 菜单视图
```

---

## 相关文档

- [设计原则](./design-principles.md) - 设计约束和原则
- [产品反思](./product-reflection.md) - 用户场景分析
- [UI 组件规格](./ui-components-spec.md) - 详细组件规格说明
- [现有 v0.2.0 TODO](../calvin/TODO.md) - v0.2.0 任务清单

