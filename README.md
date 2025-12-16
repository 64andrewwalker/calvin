# Calvin

> *Making agents behave.* 🤖

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

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
promptctl sync
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
- **🏥 Doctor Command**: Validate your configuration health
- **🌐 Remote Sync**: Push to SSH servers for remote development
- **📦 Zero Dependencies**: Single static binary

## Installation

```bash
# macOS (Homebrew)
brew install calvin

# Windows (Scoop)
scoop install calvin

# Linux / cargo
cargo install calvin

# Or download pre-built binaries from Releases
```

## Quick Start

```bash
# Initialize a new .promptpack directory
promptctl init

# Compile to all platforms
promptctl sync

# Preview what would change
promptctl diff

# Watch for changes
promptctl watch

# Validate configuration
promptctl doctor

# Install to user-level directories
promptctl install --user

# Sync to remote server
promptctl sync --remote user@host:/path/to/project
```

## Documentation

- **[Architecture](docs/architecture.md)**: System design and component overview
- **[Tech Decisions](docs/tech-decisions.md)**: Technology choices and rationale
- **[Implementation Plan](docs/implementation-plan.md)**: Phased development roadmap

## Project Status

**Stage**: Planning Complete ✅

See [TODO.md](TODO.md) for the implementation checklist.

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