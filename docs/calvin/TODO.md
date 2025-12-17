# Calvin v0.2.0 重构 TODO

> **目标版本**: v0.2.0  
> **开始日期**: 2025-12-17  
> **状态**: 规划中  
> **前提**: 项目尚未发布，可大胆重构

---

## 当前代码分析

### 文件行数统计

```
+-----------------------------+-------+--------+
| 文件                         | 行数   | 问题   |
+-----------------------------+-------+--------+
| src/main.rs                 | 1151  | GOD    | <- 上帝对象
| src/sync/mod.rs             | 904   | 偏大   |
| src/security.rs             | 764   | 偏大   |
| src/config.rs               | 620   | OK     |
| src/watcher.rs              | 550   | OK     |
| src/adapters/claude_code.rs | 443   | OK     |
| src/parser.rs               | 364   | OK     |
| src/adapters/vscode.rs      | 344   | OK     |
| src/sync/lockfile.rs        | 299   | OK     |
| src/models.rs               | 297   | OK     |
+-----------------------------+-------+--------+
```

### 上帝对象问题

**main.rs (1151行)** 包含了所有命令实现：

```rust
// 当前 main.rs 内容：
fn main()                    // CLI 解析
fn target_display_name()     // UI 辅助
fn select_targets_interactive() // UI 交互
struct Cli                   // CLI 定义
enum Commands                // 8 个子命令定义
fn cmd_version()             // 版本命令 (20行)
fn cmd_migrate()             // 迁移命令 (30行)
fn cmd_install()             // 安装命令 (145行) <- 最大
fn cmd_sync()                // 同步命令 (147行) <- 最大
fn cmd_watch()               // 监听命令 (70行)
fn cmd_diff()                // 差异命令 (90行)
fn cmd_doctor()              // 检查命令 (85行)
fn cmd_audit()               // 审计命令 (70行)
fn cmd_parse()               // 解析命令 (40行)
fn maybe_warn_allow_naked()  // 警告辅助
fn print_config_warnings()   // 警告辅助
mod tests                    // 测试代码 (170行)
```

**问题**：
1. 8 个命令实现全在一个文件
2. UI 逻辑与业务逻辑混杂
3. 重复代码（cmd_install 和 cmd_sync 有大量相似代码）
4. 测试代码也在同一文件

### 命令重复问题

```
当前命令:
  sync              部署到项目目录
  install --user    部署到用户目录（过滤 scope）
  install --global  部署到用户目录（全部）
  sync --remote     部署到远程
  doctor            检查配置
  audit             CI 审计
  watch             监听模式
  diff              显示差异
  version           版本信息
  migrate           迁移工具
  parse             调试工具

实际需要:
  deploy            部署（统一 sync + install）
  check             检查（统一 doctor + audit）
  watch             监听
  explain           AI 自述（新增）
  (无参数)          交互式入口（新增）
```

---

## 重构计划

### 重构顺序原则

```
1. 先拆分，后新增
   - 不要在上帝对象上添加新功能
   - 先把代码拆开，再加新功能

2. 保持测试通过
   - 每一步都要 cargo test 通过
   - 重构不改变外部行为

3. 删除重复，不是增加抽象
   - sync 和 install 合并，不是抽取公共函数
```

---

## Phase 0: 消灭上帝对象 (main.rs)

> **优先级**: P0  
> **目标**: main.rs 从 1151 行降到 <150 行  
> **预计时间**: 1天

### 0.1 创建 commands/ 模块

- [ ] **创建目录结构**
  ```
  src/
  |-- main.rs              # 仅保留 CLI 解析和 dispatch
  |-- cli.rs               # Cli struct + Commands enum
  |-- commands/
  |   |-- mod.rs           # 公开所有命令
  |   |-- deploy.rs        # 新 deploy 命令（合并 sync + install）
  |   |-- check.rs         # 新 check 命令（合并 doctor + audit）
  |   |-- watch.rs         # watch 命令
  |   |-- explain.rs       # 新 explain 命令
  |   +-- debug.rs         # diff, parse, migrate, version
  ```

- [ ] **迁移 CLI 定义到 cli.rs**
  - 移动 `Cli` struct
  - 移动 `Commands` enum
  - main.rs 只保留 `use crate::cli::*`

- [ ] **迁移命令实现到 commands/**
  - 每个命令一个文件
  - 函数签名保持不变

### 0.2 统一 deploy 命令

- [ ] **合并 cmd_sync 和 cmd_install**
  ```rust
  // commands/deploy.rs
  pub fn run(
      source: &Path,
      target: DeployTarget,  // Project | Home | Remote(String)
      force: bool,
      dry_run: bool,
      json: bool,
  ) -> Result<()>
  ```

- [ ] **删除 cmd_sync 和 cmd_install**

- [ ] **更新 Commands enum**
  ```rust
  enum Commands {
      Deploy {
          #[arg(long)]
          home: bool,
          #[arg(long)]
          remote: Option<String>,
          // ...
      },
      // sync 和 install 删除
  }
  ```

### 0.3 统一 check 命令

- [ ] **合并 cmd_doctor 和 cmd_audit**
  ```rust
  // commands/check.rs
  pub fn run(
      mode: SecurityMode,
      strict: bool,  // 原 audit 的 --strict-warnings
      json: bool,
  ) -> Result<()>
  ```

- [ ] **删除 cmd_doctor 和 cmd_audit**

### 0.4 创建 ui/ 模块

- [ ] **抽取 UI 相关代码**
  ```
  src/ui/
  |-- mod.rs
  |-- menu.rs      # select_targets_interactive
  |-- output.rs    # 格式化输出
  +-- prompt.rs    # 确认提示
  ```

- [ ] **移动 target_display_name()**

- [ ] **移动 select_targets_interactive()**

### 0.5 验证

- [ ] `cargo test` 全部通过
- [ ] `cargo clippy` 无警告
- [ ] main.rs < 150 行

---

## Phase 1: 交互式入口

> **优先级**: P0  
> **目标**: `calvin` 无参数运行显示交互菜单  
> **预计时间**: 2天

### 1.1 状态检测

- [ ] **实现项目状态检测**
  ```rust
  // src/state.rs
  pub enum ProjectState {
      NoPromptPack,           // .promptpack/ 不存在
      EmptyPromptPack,        // .promptpack/ 存在但为空
      Configured(AssetCount), // 有配置和 assets
  }
  
  pub fn detect_state(cwd: &Path) -> ProjectState
  ```

### 1.2 首次运行菜单

- [ ] **实现 "No .promptpack" 菜单**
  ```
  ? What would you like to do?
  
    > [1] Set up Calvin for this project
      [2] Learn what Calvin does first
      [3] Show commands (for experts)
      [4] Explain yourself (for AI assistants)
  ```

- [ ] **实现 3 步 setup 流程**
  - Step 1: 选择目标平台
  - Step 2: 选择示例模板
  - Step 3: 选择安全模式

- [ ] **生成配置和示例文件**

### 1.3 已有项目菜单

- [ ] **实现操作菜单**
  ```
  ? What would you like to do?
  
    > [1] Deploy to this project
      [2] Deploy to home directory
      [3] Preview changes
      [4] Watch mode
      [5] Check configuration
  ```

### 1.4 main() 更新

- [ ] **无参数时进入交互模式**
  ```rust
  fn main() {
      let cli = Cli::try_parse();
      match cli {
          Ok(c) => dispatch(c),
          Err(_) if args_empty() => interactive_mode(),
          Err(e) => e.exit(),
      }
  }
  ```

---

## Phase 2: explain 命令

> **优先级**: P0  
> **目标**: AI 助手可以通过 explain 了解工具  
> **预计时间**: 1天

### 2.1 实现 explain 命令

- [ ] **人类可读输出**
  - 工具用途
  - 目录结构
  - Frontmatter 格式
  - 常用命令

- [ ] **JSON 输出 (`--json`)**
  ```json
  {
    "name": "calvin",
    "version": "0.2.0",
    "purpose": "...",
    "commands": {...},
    "examples": {...}
  }
  ```

- [ ] **简短模式 (`--brief`)**

---

## Phase 3: 错误增强

> **优先级**: P1  
> **目标**: 每个错误都是教学时刻  
> **预计时间**: 1天

### 3.1 错误信息模板

- [ ] **创建错误格式规范**
  ```
  [ERROR] 标题
          位置信息

  问题描述/上下文

  FIX: 修复建议
  ```

- [ ] **更新现有错误类型**
  - NoFrontmatter
  - InvalidFrontmatter
  - MissingField

### 3.2 一键修复

- [ ] **检测 $EDITOR**
- [ ] **实现 "Press Enter to open in editor"**

### 3.3 Did you mean?

- [ ] **未知配置键检测**
  - 使用编辑距离
  - 提供最接近的建议

---

## Phase 4: sync 模块重构

> **优先级**: P1  
> **目标**: sync/mod.rs 从 904 行降到 <400 行  
> **预计时间**: 1天

### 4.1 拆分 sync 模块

- [ ] **创建 compile.rs**
  - 移动 compile_assets()

- [ ] **创建 conflict.rs**
  - 移动 ConflictReason, ConflictChoice
  - 移动 SyncPrompter trait
  - 移动 StdioPrompter

- [ ] **验证测试通过**

---

## Phase 5: 文档和清理

> **优先级**: P2  
> **目标**: 代码和文档同步  
> **预计时间**: 1天

### 5.1 更新 README

- [ ] **Quick Start 使用新命令**
  ```bash
  brew install calvin
  cd your-project
  calvin
  ```

### 5.2 更新 --help

- [ ] **简化命令列表**
- [ ] **添加 explain 到帮助**

### 5.3 删除废弃代码

- [ ] **删除旧命令定义**
- [ ] **删除未使用的函数**

---

## 进度追踪

| Phase | 任务数 | 完成 | 状态 |
|-------|--------|------|------|
| 0. 消灭上帝对象 | 12 | 0 | ⬜ TODO |
| 1. 交互式入口 | 8 | 0 | ⬜ TODO |
| 2. explain 命令 | 3 | 0 | ⬜ TODO |
| 3. 错误增强 | 5 | 0 | ⬜ TODO |
| 4. sync 重构 | 3 | 0 | ⬜ TODO |
| 5. 文档清理 | 4 | 0 | ⬜ TODO |
| **总计** | **35** | **0** | **0%** |

---

## 目标代码结构

```
src/
|-- main.rs              # <150 行，仅 CLI 解析
|-- cli.rs               # CLI 定义
|-- state.rs             # 项目状态检测
|-- commands/
|   |-- mod.rs
|   |-- deploy.rs        # 统一部署命令
|   |-- check.rs         # 统一检查命令
|   |-- watch.rs
|   |-- explain.rs       # AI 自述
|   |-- interactive.rs   # 交互式入口
|   +-- debug.rs         # diff, parse, version
|-- ui/
|   |-- mod.rs
|   |-- menu.rs          # 菜单选择
|   |-- output.rs        # 格式化输出
|   +-- prompt.rs        # 确认提示
|-- sync/
|   |-- mod.rs           # <400 行
|   |-- compile.rs       # 编译逻辑
|   |-- conflict.rs      # 冲突处理
|   |-- lockfile.rs      # 锁文件
|   +-- writer.rs        # 原子写入
|-- adapters/            # 保持不变
|-- config.rs            # 保持不变
|-- models.rs            # 保持不变
|-- parser.rs            # 保持不变
|-- security.rs          # 保持不变
|-- fs.rs                # 保持不变
+-- error.rs             # 保持不变
```

---

## 成功标准

### 代码质量

| 指标 | 当前 | 目标 |
|------|------|------|
| main.rs 行数 | 1151 | <150 |
| sync/mod.rs 行数 | 904 | <400 |
| 命令数量 | 10 | 5 |
| 上帝对象 | 1 | 0 |

### 用户体验

| 指标 | 当前 | 目标 |
|------|------|------|
| 首次部署时间 | 15分钟 | 3分钟 |
| 需要读文档 | 必须 | 可选 |
| AI 可辅助配置 | ✗ | ✓ |

---

## 相关文档

- [产品重构方案](./product-reflection.md) - 交互设计详情
- [设计原则](./design-principles.md) - 10 条核心原则
