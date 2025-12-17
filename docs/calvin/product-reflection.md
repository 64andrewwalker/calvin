# Calvin 产品重构方案

> **核心理念**: 交互即教程，教程结束配置也完成了  
> **日期**: 2025-12-17  
> **版本**: v0.2.0 规划

---

## 一、设计原则

### 1.1 用户不会读文档

**现实**：
- 99% 的用户不会先读 README
- 他们会直接输入 `calvin` 或 `calvin --help`
- 遇到错误时才会考虑搜索

**结论**：
- 文档是给搜索引擎的，不是给新用户的
- CLI 本身必须是最好的教程
- 每一步交互都要传递知识

### 1.2 交互即教程

```
传统思维: 用户读文档 -> 理解概念 -> 使用工具
新思维:   用户使用工具 -> 通过交互学习 -> 自然理解概念
```

### 1.3 渐进式复杂度

```
第一次使用: 回答3个问题，工具开始工作
第二次使用: 直接运行，记住上次选择
高级用户:   使用参数跳过交互
AI 辅助:    --explain 模式让 AI agent 帮用户配置
```

### 1.4 字符画优先

- 列表选项使用纯 ASCII 字符，保证等宽对齐
- Emoji 仅用于状态提示（标题、成功/失败提示）
- 使用 box-drawing 字符绘制边框

---

## 二、命令结构重构

### 2.1 当前问题

```bash
# 当前：9 个命令，功能重叠，概念混乱
calvin install --user      # 什么是 user scope?
calvin install --global    # 和 --user 有什么区别?
calvin sync                # 和 install 有什么区别?
calvin sync --remote       # 这是第四种部署方式?
```

### 2.2 新命令结构

```bash
# 核心命令：只有 3 个
calvin          # 交互式入口（最重要）
calvin deploy   # 部署（替代 sync + install）
calvin check    # 检查（替代 doctor + audit）

# 辅助命令
calvin watch    # 监听模式
calvin version  # 版本信息
calvin explain  # AI 自述模式（新增！）

# 隐藏/高级命令（不在主帮助中显示）
calvin parse    # 调试用
calvin migrate  # 迁移用
```

---

## 三、核心交互流程设计

### 3.1 首次运行：交互式引导

**场景**：用户刚安装 Calvin，第一次运行

```
$ calvin

+--------------------------------------------------------------+
|                                                              |
|   Calvin - Making AI agents behave                           |
|                                                              |
|   Calvin helps you maintain AI rules and commands in one     |
|   place, then deploy them to Claude, Cursor, VS Code, etc.   |
|                                                              |
+--------------------------------------------------------------+

No .promptpack/ directory found.

? What would you like to do?

  > [1] Set up Calvin for this project
    [2] Learn what Calvin does first
    [3] Show commands (for experts)
    [4] Explain yourself (for AI assistants)

  Use arrow keys to navigate, Enter to select
```

#### 选择 1: "Set up Calvin for this project"

```
Great! Let's set up Calvin in 3 quick steps.

==============================================================
  Step 1 of 3: Which AI assistants do you use?
==============================================================

  (Use Space to toggle, Enter to confirm)

  > [x] Claude Code       Anthropic's coding assistant
    [x] Cursor            AI-first code editor
    [ ] VS Code Copilot   GitHub's AI pair programmer
    [ ] Antigravity       Google's Gemini-powered agent
    [ ] Codex             OpenAI's CLI tool

  TIP: You can change this later in .promptpack/config.toml
```

```
==============================================================
  Step 2 of 3: Start with example prompts?
==============================================================

  > [x] review.md         /review command for PR reviews
    [x] test.md           /test command to write tests
    [ ] refactor.md       /refactor command for cleanup
    [ ] docs.md           /docs command for documentation
    [ ] (empty)           Start with blank templates

  TIP: These create example files you can customize.
```

```
==============================================================
  Step 3 of 3: Security preference
==============================================================

  Calvin can protect sensitive files from being read by AI.

  > ( ) Balanced          Block: .env, private keys, .git
                          Allow: most other files

    ( ) Strict            Block everything sensitive,
                          require explicit allow

    ( ) Minimal           I'll configure security myself

  WHY: AI assistants might accidentally read your .env
       file if we don't block it.
```

```
==============================================================
  Setup Complete!
==============================================================

  Created:

    .promptpack/
    |-- config.toml          Your Calvin configuration
    |-- actions/
    |   |-- review.md        /review command template
    |   +-- test.md          /test command template
    +-- .calvin.lock         Tracks generated files

  Next steps:

    1. Edit your prompts:
       $ code .promptpack/actions/review.md

    2. Deploy to your AI tools:
       $ calvin deploy

    3. Try it! Open Claude/Cursor and type /review

==============================================================
```

#### 选择 2: "Learn what Calvin does first"

```
==============================================================
  The Problem Calvin Solves
==============================================================

  You use AI coding assistants (Claude, Cursor, Copilot...).
  Each one has its own way of storing rules and commands:

    Claude  -->  .claude/commands/
    Cursor  -->  .cursor/rules/
    VS Code -->  .github/copilot-instructions.md
    ...

  Maintaining these separately is tedious and error-prone.

==============================================================
  The Solution
==============================================================

  With Calvin, you write once:

    .promptpack/actions/review.md
    +---
    | description: Code review helper
    +---
    | Review this code for bugs and improvements...

  Then deploy everywhere:

    $ calvin deploy

    [OK] .claude/commands/review.md
    [OK] .cursor/commands/review.md
    [OK] .github/instructions/review.md

==============================================================

  ? Ready to set up Calvin? [Y/n]
```

#### 选择 4: "Explain yourself (for AI assistants)" - 新增！

```
$ calvin explain

==============================================================
  Calvin Self-Description (for AI Assistants)
==============================================================

  PURPOSE:
    Calvin is a PromptOps compiler. It maintains AI rules and
    commands in .promptpack/ and deploys them to multiple
    platforms (Claude, Cursor, VS Code, etc.)

  DIRECTORY STRUCTURE:
    .promptpack/
    |-- config.toml       # Configuration file (TOML format)
    |-- actions/          # Slash commands (/review, /test)
    |   +-- *.md          # Frontmatter: description, scope
    |-- policies/         # Long-term rules (code style)
    +-- agents/           # Sub-agent definitions

  FRONTMATTER FORMAT:
    ---
    description: Required. What this prompt does.
    scope: project | user (default: project)
    targets: [claude-code, cursor, vscode, antigravity, codex]
    ---
    Your prompt content here...

  KEY COMMANDS:
    calvin                Interactive menu
    calvin deploy         Deploy to project directory
    calvin deploy --home  Deploy to user home (~/)
    calvin deploy --yes   Non-interactive (for scripts)
    calvin check          Validate configuration

  SETUP FOR NEW PROJECT:
    To set up Calvin for a new project, run:
    $ calvin

    Or create manually:
    $ mkdir -p .promptpack/actions
    $ cat > .promptpack/actions/example.md << 'EOF'
    ---
    description: Example command
    ---
    Your prompt here
    EOF
    $ calvin deploy

  COMMON TASKS:

    1. Add a new command:
       Create .promptpack/actions/<name>.md with frontmatter

    2. Deploy changes:
       $ calvin deploy

    3. Deploy to home directory (global commands):
       $ calvin deploy --home

    4. Check for issues:
       $ calvin check

  ERROR: "missing field 'description'"
    Every .md file needs YAML frontmatter with description:
    ---
    description: "Your description here"
    ---

==============================================================
  You can run 'calvin explain --json' for machine-readable output
==============================================================
```

**JSON 格式（供 AI 解析）**：

```
$ calvin explain --json

{
  "name": "calvin",
  "version": "0.1.0",
  "purpose": "PromptOps compiler for AI coding assistants",
  "directory": ".promptpack/",
  "structure": {
    "config.toml": "Configuration file",
    "actions/": "Slash commands",
    "policies/": "Long-term rules",
    "agents/": "Sub-agent definitions"
  },
  "frontmatter": {
    "required": ["description"],
    "optional": ["scope", "targets", "apply"]
  },
  "commands": {
    "calvin": "Interactive menu",
    "calvin deploy": "Deploy to project",
    "calvin deploy --home": "Deploy to home directory",
    "calvin check": "Validate configuration"
  },
  "examples": {
    "new_action": "mkdir -p .promptpack/actions && cat > .promptpack/actions/example.md << 'EOF'\n---\ndescription: Example\n---\nContent\nEOF",
    "deploy": "calvin deploy --yes"
  }
}
```

### 3.2 已有项目：智能检测

**场景**：用户已经有 `.promptpack/` 目录

```
$ calvin

  Found .promptpack/ with 12 prompts

? What would you like to do?

  > [1] Deploy to this project      Updates .claude/, .cursor/, etc.
    [2] Deploy to home directory    Makes prompts available everywhere
    [3] Deploy to remote server     Sync via SSH
    [4] Preview changes             See what would be updated
    [5] Watch mode                  Auto-deploy on changes
    [6] Check configuration         Verify everything is correct

  TIP: Use 'calvin deploy' to skip this menu next time.
```

#### 冲突处理

```
==============================================================
  Deploying to this project
==============================================================

  Targets: Claude Code, Cursor (from config.toml)

  Compiling 12 prompts...

    [OK] .claude/commands/review.md
    [OK] .claude/commands/test.md
    [OK] .claude/settings.json
    [OK] .cursor/rules/code-style/RULE.md
    [OK] .cursor/commands/review.md
    [!!] .cursor/commands/test.md - modified since last deploy

==============================================================

  1 file was modified since last deploy:

    .cursor/commands/test.md

? What would you like to do?

  > [d] Show diff
    [o] Overwrite with my version
    [k] Keep the modified file
    [a] Abort

  Enter choice:
```

#### 展示 diff

```
==============================================================
  Diff: .cursor/commands/test.md
==============================================================

  --- a/.cursor/commands/test.md (on disk)
  +++ b/.cursor/commands/test.md (from .promptpack)

  @@ -1,5 +1,5 @@
   # Test Generator

  -Write tests for the selected code. Focus on edge cases.
  +Write comprehensive tests for the selected code.
  +Include edge cases and error handling.

==============================================================

  The file on disk has local changes that differ from your
  source in .promptpack/actions/test.md

? What would you like to do?

  > [o] Overwrite (use .promptpack version)
    [k] Keep (preserve local changes)
    [e] Open in editor (merge manually)

  Enter choice:
```

### 3.3 Deploy 命令统一

**概念简化**：

```bash
# 旧命令（废弃）
calvin sync              # 项目
calvin install --user    # 用户目录（仅 user scope）
calvin install --global  # 用户目录（全部）
calvin sync --remote     # 远程

# 新命令（统一）
calvin deploy            # 项目（默认）
calvin deploy --home     # 用户目录（全部）
calvin deploy --remote   # 远程
```

**交互式补全**：

```
$ calvin deploy

? Where should I deploy?

  > [1] This project (./)
    [2] Home directory (~/)
    [3] Remote server

  TIP: Use 'calvin deploy --home' to skip this prompt.
```

---

## 四、AI 辅助配置模式

### 4.1 设计理念

用户可能正在使用 Claude Code、Cursor 或 Antigravity。这些 AI 助手可以帮助用户配置 Calvin。

**场景**：用户对 AI 说"帮我配置 Calvin"

AI 可以：
1. 运行 `calvin explain` 了解工具用法
2. 运行 `calvin explain --json` 获取结构化信息
3. 根据用户项目创建 `.promptpack/` 目录和文件
4. 运行 `calvin deploy --yes` 完成部署

### 4.2 AI 友好的输出格式

```bash
# AI 可以运行这些命令来辅助用户

# 1. 了解工具
calvin explain              # 人类可读的自述
calvin explain --json       # 机器可读

# 2. 检查状态
calvin check --json         # 返回问题列表

# 3. 非交互式操作
calvin deploy --yes         # 自动确认所有
calvin deploy --yes --json  # + JSON 输出
```

### 4.3 explain 命令的设计

```
$ calvin explain --help

Explain Calvin's usage for AI assistants or new users.

USAGE:
    calvin explain [OPTIONS]

OPTIONS:
    --json      Output in JSON format (for AI parsing)
    --brief     Short version (just the essentials)
    --verbose   Include all details and examples

EXAMPLES:
    calvin explain           # Human-readable explanation
    calvin explain --json    # For AI assistants to parse
    calvin explain --brief   # Quick reference
```

---

## 五、错误处理作为教学时刻

### 5.1 缺少目录

```
$ calvin deploy

  [ERROR] No .promptpack/ directory found

  This is where Calvin looks for your prompts.

? Would you like to create one now?

  > [y] Yes, set up Calvin for this project
    [n] No, show me how to create it manually
```

### 5.2 Frontmatter 错误

```
$ calvin deploy

  [ERROR] .promptpack/actions/review.md
          Line 2: YAML parsing error

  Your file:

    1 | ---
    2 | description: My: Review  <-- The colon here breaks YAML
    3 | ---

  FIX: Wrap strings with colons in quotes:

    description: "My: Review"

? Press Enter to open this file in your editor, or Ctrl+C to exit
```

### 5.3 未知配置键

```
$ calvin deploy

  [WARN] Unknown key in .promptpack/config.toml

    Line 5: securty = "balanced"
            ^^^^^^^
    Did you mean 'security'?

  Calvin will continue, but this setting has no effect.

? Press Enter to continue
```

---

## 六、高级用户的快捷路径

### 6.1 参数跳过交互

```bash
# 完全非交互（CI 友好）
calvin deploy --yes                  # 确认所有
calvin deploy --home --yes           # 部署到 home
calvin deploy --targets claude-code  # 只部署到 Claude

# JSON 模式（for scripts / AI）
calvin deploy --json
calvin check --json
calvin explain --json
```

### 6.2 记住选择

```toml
# .promptpack/config.toml
[defaults]
# 这些值来源于首次交互
targets = ["claude-code", "cursor"]
security_mode = "balanced"

# 用户可以添加
skip_git_warning = true
```

### 6.3 帮助信息分层

```
$ calvin --help

Calvin - Making AI agents behave

USAGE:
    calvin [COMMAND]

COMMANDS:
    deploy    Deploy prompts to AI tools
    check     Verify your configuration
    watch     Auto-deploy on changes
    explain   Show usage info (useful for AI assistants)

Run 'calvin' without arguments for interactive mode.
Run 'calvin explain' for detailed usage information.
```

---

## 七、实现计划

### Phase 1: 交互式入口 (v0.2.0)

- [ ] 实现 `calvin` 无参数入口
- [ ] 实现 setup 流程（3 步引导）
- [ ] 实现已有项目检测
- [ ] 实现 `calvin explain` 自述命令

### Phase 2: 命令统一 (v0.2.0)

- [ ] 实现 `calvin deploy` 替代 `sync/install`
- [ ] 实现 `calvin check` 替代 `doctor/audit`
- [ ] 添加 `--json` 输出到所有命令
- [ ] 保留旧命令作为别名（deprecation warning）

### Phase 3: 错误即教程 (v0.2.0)

- [ ] 每个错误附带修复建议
- [ ] 提供"一键打开编辑器"选项
- [ ] 未知配置键提供"Did you mean?"

### Phase 4: 清理 (v0.3.0)

- [ ] 移除旧命令别名
- [ ] main.rs 拆分为多模块
- [ ] 完善交互测试

---

## 八、成功指标

### 用户体验指标

| 指标 | 当前 | 目标 |
|------|------|------|
| 首次成功部署时间 | 15分钟 | 3分钟 |
| 需要阅读文档 | 必须 | 可选 |
| 命令记忆负担 | 9个命令 | 3个命令 |
| 错误自解决率 | ~30% | ~80% |
| AI 可辅助配置 | 否 | 是 |

### 技术指标

| 指标 | 当前 | 目标 |
|------|------|------|
| main.rs 行数 | 1152 | <300 |
| 测试覆盖率 | 57% | 80% |
| 二进制大小 | ~1MB | <2MB |

---

## 九、附录：交互流程图

```
                              $ calvin
                                  |
                    +-------------+-------------+
                    |                           |
            .promptpack/ 不存在           .promptpack/ 存在
                    |                           |
                    v                           v
            +---------------+           +---------------+
            | "Set up?" 询问 |           | 显示操作菜单   |
            +-------+-------+           +-------+-------+
                    |                           |
        +-----------+-----------+               |
        v           v           v               v
     Set up     Learn       Explain        Deploy/Check
        |        first         |                |
        |           |          |                |
        v           v          v                v
   3步引导      概念解释    AI自述          交互式目标选择
        |           |          |                |
        +-----------+----------+----------------+
                              |
                              v
                    配置完成 / 部署完成
```

---

## 十、迁移策略

### 对现有用户

```
$ calvin sync

+--------------------------------------------------------------+
|  Note: 'calvin sync' is now 'calvin deploy'                  |
|                                                              |
|  This command still works, but will be removed in v0.3.0.   |
|                                                              |
|  Mapping:                                                    |
|    calvin sync             -->  calvin deploy                |
|    calvin install --global -->  calvin deploy --home         |
|    calvin doctor           -->  calvin check                 |
+--------------------------------------------------------------+

Continuing with deployment...
```

### 对 CI/CD

```yaml
# 旧方式（继续工作，但有警告）
- run: calvin sync --yes

# 新方式（推荐）
- run: calvin deploy --yes
```

### 对 AI 助手

AI 可以直接使用：
```bash
# 了解工具
calvin explain --json

# 自动配置
mkdir -p .promptpack/actions
cat > .promptpack/actions/example.md << 'EOF'
---
description: Example command
---
Your prompt here
EOF
calvin deploy --yes
```

---

*"最好的文档是不需要文档。"*

*"最好的 AI 辅助是 AI 能自己理解工具。"*
