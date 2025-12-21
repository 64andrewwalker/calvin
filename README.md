# Calvin

> *Making agents behave.* 🤖

**English** | **[简体中文](README_zh-CN.md)**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/64andrewwalker/calvin/actions/workflows/ci.yml/badge.svg)](https://github.com/64andrewwalker/calvin/actions/workflows/ci.yml)
[![Nightly](https://github.com/64andrewwalker/calvin/actions/workflows/nightly.yml/badge.svg)](https://github.com/64andrewwalker/calvin/releases/tag/nightly)
[![Coverage](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/64andrewwalker/18eca57524acb51f1d2b3d1c7bf2a64a/raw/calvin-coverage.json)](https://github.com/64andrewwalker/calvin/actions/workflows/ci.yml)

**Calvin** is a PromptOps compiler and synchronization tool that lets you maintain AI rules, commands, and workflows in a single source format, then compile and distribute them to multiple AI coding assistant platforms.

Named after **Dr. Susan Calvin** — the legendary robot psychologist from Asimov's *I, Robot* series — Calvin ensures your AI agents follow the rules, stay safe, and behave predictably across all platforms.

## The Problem

Your team uses multiple AI coding assistants: **Claude Code**, **Cursor**, **GitHub Copilot**, **Antigravity**, **Codex**. Each has its own format for rules and commands:

- Claude: `.claude/commands/`, `CLAUDE.md`
- Cursor: `.cursor/rules/`, `.cursor/commands/`
- VS Code/Copilot: `.github/copilot-instructions.md`, `AGENTS.md`
- Antigravity: `.agent/rules/`, `.agent/workflows/`
- Codex: `~/.codex/prompts/`

Maintaining the same intent across all these platforms is tedious and error-prone.

## The Solution

Write once, compile everywhere.

```
.promptpack/                    # Your source of truth
├── policies/
│   ├── code-style.md          # → Compiles to rules for all platforms
│   └── security.md
├── actions/
│   ├── generate-tests.md      # → Compiles to slash commands
│   └── pr-review.md
├── agents/
│   └── reviewer.md            # → Compiles to sub-agent definitions
└── mcp/
    └── github.toml            # → Compiles to MCP configs
```

Then run:

```bash
calvin deploy
```

And Calvin generates platform-specific outputs:

```
.claude/commands/generate-tests.md
.claude/settings.json              # With security deny-lists!
.cursor/rules/code-style/RULE.md
.cursor/commands/generate-tests.md
.github/copilot-instructions.md
.agent/rules/code-style.md
.agent/workflows/generate-tests.md
```

## Features

- **📝 Single Source of Truth**: Maintain prompts in one place
- **🔄 Multi-Platform Compilation**: Claude Code, Cursor, VS Code, Antigravity, Codex
- **🔒 Security by Default**: Auto-generates deny lists, blocks risky MCP servers
- **👀 Watch Mode**: Auto-recompile on file changes
- **🔍 Check Command**: Validate your configuration health
- **🌐 Remote Sync**: Push to SSH servers for remote development
- **📦 Zero Dependencies**: Single static binary

## Installation

### Quick Install (Recommended)

**Windows (PowerShell)**:
```powershell
irm https://raw.githubusercontent.com/64andrewwalker/calvin/main/scripts/install-windows.ps1 | iex
```

**macOS / Linux**:
```bash
curl -fsSL https://github.com/64andrewwalker/calvin/releases/download/nightly/calvin-$(uname -m)-$(uname -s | tr '[:upper:]' '[:lower:]' | sed 's/darwin/apple-darwin/;s/linux/unknown-linux-gnu/').tar.gz | tar xz
sudo mv calvin /usr/local/bin/
```

### Package Managers

```bash
# macOS (Homebrew) - coming soon
brew install calvin

# Windows (Scoop) - coming soon
scoop install calvin

# Via Cargo
cargo install calvin

# Via cargo-binstall (pre-built binaries)
cargo binstall calvin
```

### Manual Download

Download pre-built binaries from [Releases](https://github.com/64andrewwalker/calvin/releases):

| Platform | Download |
|----------|----------|
| Windows x64 | [`calvin-x86_64-pc-windows-msvc.zip`](https://github.com/64andrewwalker/calvin/releases/download/nightly/calvin-x86_64-pc-windows-msvc.zip) |
| macOS Apple Silicon | [`calvin-aarch64-apple-darwin.tar.gz`](https://github.com/64andrewwalker/calvin/releases/download/nightly/calvin-aarch64-apple-darwin.tar.gz) |
| macOS Intel | [`calvin-x86_64-apple-darwin.tar.gz`](https://github.com/64andrewwalker/calvin/releases/download/nightly/calvin-x86_64-apple-darwin.tar.gz) |
| Linux x64 | [`calvin-x86_64-unknown-linux-gnu.tar.gz`](https://github.com/64andrewwalker/calvin/releases/download/nightly/calvin-x86_64-unknown-linux-gnu.tar.gz) |
| Linux ARM64 | [`calvin-aarch64-unknown-linux-gnu.tar.gz`](https://github.com/64andrewwalker/calvin/releases/download/nightly/calvin-aarch64-unknown-linux-gnu.tar.gz) |

## Building from Source

Calvin is written in Rust. To build from source:

```bash
# Prerequisites: Rust 1.70+ (https://rustup.rs)

# Clone the repository
git clone https://github.com/your-org/calvin.git
cd calvin

# Build debug version
cargo build

# Build release version (optimized, ~1.1MB binary)
cargo build --release

# Run tests
cargo test

# Install locally
cargo install --path .
```

The compiled binary will be at `target/release/calvin`.

## Quick Start

```bash
# Create a .promptpack directory with your policies and actions
mkdir -p .promptpack/policies .promptpack/actions

# Compile to all platforms
calvin deploy

# Preview what would change
calvin diff

# Preview what would change in home directory (~/...)
calvin diff --home

# Watch for changes and auto-recompile
calvin watch

# Validate configuration and security
calvin check

# Deploy everything to home directory targets (~/.claude/, ~/.codex/, ...)
calvin deploy --home

# Sync to remote server
calvin deploy --remote user@host:/path/to/project
```

## Documentation

### User Documentation

- **[Command Reference](docs/command-reference.md)**: CLI commands and options
- **[Configuration](docs/configuration.md)**: Config file, environment variables
- **[Target Platforms](docs/target-platforms.md)**: Supported IDEs and output formats

### API Reference

- **[API Overview](docs/api/README.md)**: API documentation index
- **[Library API](docs/api/library.md)**: Rust library reference
- **[Frontmatter Spec](docs/api/frontmatter.md)**: Source file format specification
- **[API Changelog](docs/api/changelog.md)**: Format version history
- **[API Versioning](docs/api/versioning.md)**: Versioning policy

### Guides

- **[Scope Guide](docs/guides/scope-guide.md)**: Understanding project vs user scope
- **[Pitfall Mitigations](docs/guides/pitfall-mitigations.md)**: Known issues and solutions

### Architecture (for Contributors)

- **[Architecture Overview](docs/architecture/overview.md)**: System design and goals
- **[Layered Architecture](docs/architecture/layers.md)**: Four-layer clean architecture
- **[Directory Structure](docs/architecture/directory.md)**: Codebase organization
- **[Tech Decisions](docs/tech-decisions.md)**: Technology choices and rationale

### Reports

- **[Security Audit](docs/reports/security-audit-report.md)**: Security analysis and findings
- **[API Review](docs/reports/api-review-2025-12-19.md)**: CLI and library API review

## Project Status

**Version**: v0.2.0  
**Stage**: Feature complete, architecture v2 deployed

Recent additions:
- ✅ Clean Architecture refactoring (domain/application/infrastructure layers)
- ✅ 600+ tests passing with 75%+ coverage
- ✅ Cross-platform CI (Ubuntu, Windows, macOS)
- ✅ SSH remote sync with rsync acceleration
- ✅ User-scope installations (`--home` flag)
- ✅ Security health checks (`check` command)

See [docs/architecture/TODO.md](docs/architecture/TODO.md) for detailed roadmap.

## Philosophy

Calvin doesn't try to be clever:

1. **Explicit over implicit**: Every rule has clear frontmatter metadata
2. **Security is mandatory**: Deny lists are auto-generated, not optional
3. **Platform-native**: We generate what each platform expects, not hacks
4. **Deterministic**: Same input always produces same output
5. **Non-destructive**: Never overwrites files you created manually

## License

MIT

---

*"A robot must obey orders given it by human beings except where such orders would conflict with the First Law."* — Isaac Asimov
