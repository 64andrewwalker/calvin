# Calvin

> *让 AI 乖乖听话。* 🤖

**[English](README.md)** | **简体中文**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/64andrewwalker/calvin/actions/workflows/ci.yml/badge.svg)](https://github.com/64andrewwalker/calvin/actions/workflows/ci.yml)
[![Nightly](https://github.com/64andrewwalker/calvin/actions/workflows/nightly.yml/badge.svg)](https://github.com/64andrewwalker/calvin/releases/tag/nightly)
[![Coverage](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/64andrewwalker/18eca57524acb51f1d2b3d1c7bf2a64a/raw/calvin-coverage.json)](https://github.com/64andrewwalker/calvin/actions/workflows/ci.yml)

**Calvin** 是一个 PromptOps 编译器和同步工具，让你可以用统一的源格式维护 AI 规则、命令和工作流，然后编译分发到多个 AI 编程助手平台。

命名源自阿西莫夫《我，机器人》系列中传奇的机器人心理学家 **苏珊·卡尔文博士** —— Calvin 确保你的 AI 智能体遵守规则、保持安全、在所有平台上行为可预测。

## 痛点

你的团队使用多个 AI 编程助手：**Claude Code**、**Cursor**、**GitHub Copilot**、**Antigravity**、**Codex**。每个都有自己的规则和命令格式：

- Claude: `.claude/commands/`, `CLAUDE.md`
- Cursor: `.cursor/rules/`, `.cursor/commands/`
- VS Code/Copilot: `.github/copilot-instructions.md`, `AGENTS.md`
- Antigravity: `.agent/rules/`, `.agent/workflows/`
- Codex: `~/.codex/prompts/`

在所有这些平台上保持一致的意图既繁琐又容易出错。

## 解决方案

一次编写，到处编译。

```
.promptpack/                    # 你的唯一信息源
├── policies/
│   ├── code-style.md          # → 编译为所有平台的规则
│   └── security.md
├── actions/
│   ├── generate-tests.md      # → 编译为斜杠命令
│   └── pr-review.md
├── agents/
│   └── reviewer.md            # → 编译为子智能体定义
├── skills/
│   └── draft-commit/
│       ├── SKILL.md           # → 编译为 SKILL.md 技能目录（Claude/Codex/Cursor）
│       └── scripts/validate.py
└── mcp/
    └── github.toml            # → MCP 配置（规划中；当前只校验，不生成）
```

然后运行：

```bash
calvin deploy
```

Calvin 会生成平台特定的输出：

```
.claude/commands/generate-tests.md
.claude/settings.json              # 包含安全拒绝列表！
.claude/skills/draft-commit/SKILL.md
.cursor/rules/code-style/RULE.md
.cursor/commands/generate-tests.md
.github/copilot-instructions.md
.agent/rules/code-style.md
.agent/workflows/generate-tests.md
.codex/skills/draft-commit/SKILL.md
```

## 特性

- **📝 单一信息源**：在一处维护所有提示词
- **🔄 多平台编译**：Claude Code、Cursor、VS Code、Antigravity、Codex
- **🧠 Skills 支持**：面向 Claude Code、Codex、Cursor 的目录式技能
- **🔒 默认安全**：自动生成拒绝列表，阻止危险的 MCP 服务器
- **👀 监听模式**：文件变更时自动重新编译
- **🔍 检查命令**：验证配置健康状态
- **🌐 远程同步**：推送到 SSH 服务器进行远程开发
- **📦 零依赖**：单个静态二进制文件

## 安装

### 快速安装（推荐）

**Windows (PowerShell)**：

```powershell
irm https://raw.githubusercontent.com/64andrewwalker/calvin/main/scripts/install-windows.ps1 | iex
```

**macOS / Linux**：

```bash
curl -fsSL https://github.com/64andrewwalker/calvin/releases/download/nightly/calvin-$(uname -m)-$(uname -s | tr '[:upper:]' '[:lower:]' | sed 's/darwin/apple-darwin/;s/linux/unknown-linux-gnu/').tar.gz | tar xz
sudo mv calvin /usr/local/bin/
```

### 包管理器

```bash
# macOS (Homebrew) - 即将推出
brew install calvin

# Windows (Scoop) - 即将推出
scoop install calvin

# 通过 Cargo
cargo install calvin

# 通过 cargo-binstall（预编译二进制）
cargo binstall calvin
```

### 手动下载

从 [Releases](https://github.com/64andrewwalker/calvin/releases) 下载预编译二进制文件：

| 平台 | 下载 |
|------|------|
| Windows x64 | [`calvin-x86_64-pc-windows-msvc.zip`](https://github.com/64andrewwalker/calvin/releases/download/nightly/calvin-x86_64-pc-windows-msvc.zip) |
| macOS Apple Silicon | [`calvin-aarch64-apple-darwin.tar.gz`](https://github.com/64andrewwalker/calvin/releases/download/nightly/calvin-aarch64-apple-darwin.tar.gz) |
| macOS Intel | [`calvin-x86_64-apple-darwin.tar.gz`](https://github.com/64andrewwalker/calvin/releases/download/nightly/calvin-x86_64-apple-darwin.tar.gz) |
| Linux x64 | [`calvin-x86_64-unknown-linux-gnu.tar.gz`](https://github.com/64andrewwalker/calvin/releases/download/nightly/calvin-x86_64-unknown-linux-gnu.tar.gz) |
| Linux ARM64 | [`calvin-aarch64-unknown-linux-gnu.tar.gz`](https://github.com/64andrewwalker/calvin/releases/download/nightly/calvin-aarch64-unknown-linux-gnu.tar.gz) |

## 从源码构建

Calvin 使用 Rust 编写。从源码构建：

```bash
# 前置条件：Rust 1.70+ (https://rustup.rs)

# 克隆仓库
git clone https://github.com/64andrewwalker/calvin.git
cd calvin

# 构建调试版本
cargo build

# 构建发布版本（优化后约 1.1MB）
cargo build --release

# 运行测试
cargo test

# 本地安装
cargo install --path .
```

编译后的二进制文件位于 `target/release/calvin`。

## 快速开始

```bash
# 创建 .promptpack 目录，包含你的策略和操作
mkdir -p .promptpack/policies .promptpack/actions

# 编译到所有平台
calvin deploy

# 预览将要发生的变更
calvin diff

# 预览主目录中的变更 (~/...)
calvin diff --home

# 监听文件变更并自动重新编译
calvin watch

# 验证配置和安全性
calvin check

# 部署到主目录目标 (~/.claude/, ~/.codex/, ...)
calvin deploy --home

# 同步到远程服务器
calvin deploy --remote user@host:/path/to/project
```

## 文档

### 用户文档

- **[命令参考](docs/command-reference.md)**：CLI 命令和选项
- **[配置说明](docs/configuration.md)**：配置文件、环境变量
- **[目标平台](docs/target-platforms.md)**：支持的 IDE 和输出格式

### API 参考

- **[API 概览](docs/api/README.md)**：API 文档索引
- **[库 API](docs/api/library.md)**：Rust 库参考
- **[Frontmatter 规范](docs/api/frontmatter.md)**：源文件格式规范
- **[API 变更日志](docs/api/changelog.md)**：格式版本历史
- **[API 版本策略](docs/api/versioning.md)**：版本管理政策

### 指南

- **[作用域指南](docs/guides/scope-guide.md)**：理解项目作用域 vs 用户作用域
- **[常见问题](docs/guides/pitfall-mitigations.md)**：已知问题及解决方案

### 架构（贡献者）

- **[架构概览](docs/architecture.md)**：系统设计和目标
- **[分层架构](docs/architecture/layers.md)**：四层整洁架构
- **[目录结构](docs/architecture/directory.md)**：代码库组织
- **[技术决策](docs/tech-decisions.md)**：技术选择及理由

### 报告

- **[安全审计](docs/reports/security-audit-report.md)**：安全分析和发现
- **[API 评审](docs/reports/api-review-2025-12-19.md)**：CLI 和库 API 评审

## 项目状态

**版本**：v0.6.0  
**阶段**：功能完整，架构 v2 已部署

最近更新：

- ✅ 整洁架构重构（domain/application/infrastructure 层）
- ✅ 1000+ 测试通过，75%+ 覆盖率（见 CI 徽章）
- ✅ 跨平台 CI（Ubuntu、Windows、macOS）
- ✅ 基于 rsync 加速的 SSH 远程同步
- ✅ 用户作用域安装（`--home` 标志）
- ✅ 安全健康检查（`check` 命令）
- ✅ Skills 支持（`.promptpack/skills/<id>/SKILL.md`）

详细路线图请参阅 [docs/architecture/todo.md](docs/architecture/todo.md)。

## 设计哲学

Calvin 不追求花哨：

1. **显式优于隐式**：每条规则都有清晰的 frontmatter 元数据
2. **安全是必须的**：拒绝列表自动生成，不是可选项
3. **平台原生**：生成每个平台期望的格式，而非 hack
4. **确定性**：相同输入始终产生相同输出
5. **非破坏性**：永不覆盖你手动创建的文件

## 许可证

MIT

---

*"机器人必须服从人类的命令，除非这些命令与第一定律相冲突。"* — 艾萨克·阿西莫夫
