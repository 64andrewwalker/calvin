# Calvin - Project Analysis Report

> **Generated**: 2025-12-18  
> **Version**: 0.2.0  
> **Status**: Feature Complete

---

## Executive Summary

**Calvin** is a PromptOps compiler and synchronization tool written in Rust that enables teams to maintain AI assistant rules, commands, and workflows in a single source format (PromptPack), then compile and distribute them to five AI coding assistant platforms: Claude Code, Cursor, VS Code/GitHub Copilot, Google Antigravity, and OpenAI Codex.

Named after **Dr. Susan Calvin** from Asimov's *I, Robot* series, the tool ensures AI agents follow rules, stay safe, and behave predictably across all platforms.

---

## Architecture Diagram

```mermaid
graph TB
    subgraph Sources["📁 .promptpack/ (Source of Truth)"]
        policies["policies/*.md"]
        actions["actions/*.md"]
        agents["agents/*.md"]
        mcp["mcp/*.toml"]
        config["config.toml"]
    end

    subgraph Parser["🔍 Source Parser"]
        extract["Frontmatter Extraction"]
        validate["YAML Validation"]
        models["PromptAsset Models"]
    end

    subgraph Compiler["⚙️ Compiler Engine"]
        adapters["Target Adapters"]
        security["Security Module"]
        escaping["Context-Aware Escaping"]
    end

    subgraph Adapters["🎯 Platform Adapters"]
        claude["ClaudeCodeAdapter"]
        cursor["CursorAdapter"]
        vscode["VSCodeAdapter"]
        antigravity["AntigravityAdapter"]
        codex["CodexAdapter"]
    end

    subgraph Sync["🔄 Sync Engine"]
        plan["Plan (Diff Detection)"]
        conflict["Conflict Resolution"]
        execute["Execute (Write Files)"]
        lockfile["Lockfile Management"]
    end

    subgraph Outputs["📤 Generated Outputs"]
        claude_out[".claude/commands/\n.claude/settings.json"]
        cursor_out[".cursor/rules/\n.cursor/commands/"]
        vscode_out[".github/copilot-instructions.md\nAGENTS.md"]
        antigravity_out[".agent/rules/\n.agent/workflows/"]
        codex_out["~/.codex/prompts/"]
    end

    subgraph CLI["🖥️ CLI Interface"]
        deploy["deploy"]
        check["check"]
        watch["watch"]
        diff["diff"]
        explain["explain"]
        migrate["migrate"]
    end

    Sources --> Parser
    Parser --> Compiler
    Compiler --> Adapters
    claude --> claude_out
    cursor --> cursor_out
    vscode --> vscode_out
    antigravity --> antigravity_out
    codex --> codex_out
    Adapters --> Sync
    Sync --> Outputs
    CLI --> Parser
    CLI --> Sync
```

---

## Module Dependency Graph

```mermaid
graph LR
    subgraph Core["Core Modules"]
        main["main.rs"]
        cli["cli.rs"]
        lib["lib.rs"]
    end

    subgraph Models["Data Models"]
        models["models.rs"]
        config["config.rs"]
        error["error.rs"]
    end

    subgraph Parser["Parsing"]
        parser["parser.rs"]
    end

    subgraph Adapters["Adapters"]
        adapters_mod["adapters/mod.rs"]
        claude_code["claude_code.rs"]
        cursor["cursor.rs"]
        vscode["vscode.rs"]
        antigravity["antigravity.rs"]
        codex["codex.rs"]
        escaping["escaping.rs"]
    end

    subgraph Sync["Sync Engine"]
        sync_mod["sync/mod.rs"]
        engine["engine.rs"]
        plan["plan.rs"]
        execute["execute.rs"]
        lockfile["lockfile.rs"]
        conflict["conflict.rs"]
        remote["remote.rs"]
        scope["scope.rs"]
    end

    subgraph Security["Security"]
        security["security.rs"]
        security_baseline["security_baseline.rs"]
    end

    subgraph UI["User Interface"]
        ui_mod["ui/mod.rs"]
        views["ui/views/"]
        blocks["ui/blocks/"]
        primitives["ui/primitives/"]
        widgets["ui/widgets/"]
    end

    subgraph Commands["Commands"]
        commands_mod["commands/mod.rs"]
        deploy["deploy/"]
        check["check.rs"]
        watch["watch.rs"]
        interactive["interactive.rs"]
    end

    subgraph FS["File System"]
        fs["fs.rs"]
        watcher["watcher.rs"]
    end

    main --> cli
    cli --> commands_mod
    commands_mod --> deploy
    commands_mod --> check
    commands_mod --> watch
    deploy --> sync_mod
    deploy --> parser
    deploy --> adapters_mod
    sync_mod --> engine
    engine --> plan
    engine --> execute
    engine --> lockfile
    engine --> conflict
    engine --> remote
    adapters_mod --> claude_code
    adapters_mod --> cursor
    adapters_mod --> vscode
    adapters_mod --> antigravity
    adapters_mod --> codex
    adapters_mod --> escaping
    adapters_mod --> security_baseline
    parser --> models
    models --> config
    check --> security
    ui_mod --> views
    ui_mod --> blocks
    sync_mod --> fs
```

---

## Core Components

### 1. Source Parser (`src/parser.rs`)

- **Purpose**: Parse PromptPack source files (Markdown with YAML frontmatter)
- **Input**: `.promptpack/` directory structure
- **Output**: `Vec<PromptAsset>` - validated AST with resolved references
- **Key Features**:
  - Frontmatter extraction with `---` delimiters
  - YAML validation via `serde_yaml`
  - Required field validation (`description` field)
  - ID derivation from file paths (kebab-case)

### 2. Data Models (`src/models.rs`)

| Type | Description |
|------|-------------|
| `AssetKind` | Policy, Action, or Agent |
| `Scope` | Project or User level |
| `Target` | ClaudeCode, Cursor, VSCode, Antigravity, Codex, All |
| `Frontmatter` | Parsed YAML metadata |
| `PromptAsset` | Complete parsed source file |

### 3. Target Adapters (`src/adapters/`)

Each adapter implements the `TargetAdapter` trait:

| Adapter | Output Directories | Key Transformations |
|---------|-------------------|---------------------|
| `ClaudeCodeAdapter` | `.claude/commands/`, `.claude/settings.json` | Slash commands, permission deny lists |
| `CursorAdapter` | `.cursor/rules/<id>/RULE.md`, `.cursor/commands/` | Rule frontmatter (globs, alwaysApply) |
| `VSCodeAdapter` | `.github/copilot-instructions.md`, `AGENTS.md` | Instruction files with applyTo |
| `AntigravityAdapter` | `.agent/rules/`, `.agent/workflows/` | Rules and workflows |
| `CodexAdapter` | `~/.codex/prompts/` | User-level prompts with $ARGUMENTS |

### 4. Sync Engine (`src/sync/`)

Three-stage sync lifecycle:

1. **Plan** (`plan.rs`): Detect changes and conflicts via lockfile comparison
2. **Resolve** (`conflict.rs`): Handle conflicts (force/interactive/skip)
3. **Execute** (`execute.rs`): Transfer files using optimal strategy

| Module | Responsibility |
|--------|---------------|
| `engine.rs` | SyncEngine orchestration (1413 lines) |
| `lockfile.rs` | SHA256 hash tracking for change detection |
| `remote.rs` | SSH/SCP sync for remote servers |
| `scope.rs` | ScopePolicy for project/home/remote destinations |

### 5. Security Module (`src/security.rs`)

Three-tier security mode system:

| Mode | Behavior |
|------|----------|
| `yolo` | No enforcement, INFO logs only |
| `balanced` | Generate protections, WARN on issues (default) |
| `strict` | Block on security violations |

**Hardcoded Minimum Deny List** (even in yolo mode):
- `.env`, `.env.*`, `*.pem`, `*.key`, `id_rsa`, `id_ed25519`, `.git/`

### 6. Configuration (`src/config.rs`)

**Configuration Hierarchy** (highest to lowest priority):
1. CLI flags
2. Environment variables (`CALVIN_*`)
3. Project config (`.promptpack/config.toml`)
4. User config (`~/.config/calvin/config.toml`)
5. Built-in defaults

### 7. CLI Interface (`src/cli.rs`)

| Command | Description |
|---------|-------------|
| `deploy` | Deploy PromptPack assets (primary command) |
| `check` | Validate platform configurations |
| `watch` | Auto-sync on file changes |
| `diff` | Preview changes without writing |
| `explain` | Show usage information for humans/AI |
| `migrate` | Migrate source format or adapter output |
| `version` | Show version and adapter info |

### 8. UI System (`src/ui/`)

Modular TUI system with:
- **Views**: Deploy, Check, Watch, Interactive, Diff, Migrate, Parse, Version
- **Blocks**: Header, Progress, Summary components
- **Primitives**: Box drawing, colors, terminal handling
- **Widgets**: Checklist, Spinner, Live regions

---

## Tech Stack Inventory

### Language & Toolchain

| Component | Version/Details |
|-----------|-----------------|
| **Language** | Rust 2021 Edition |
| **Package Manager** | Cargo |
| **MSRV** | Rust 1.70+ |

### Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `clap` | 4 | CLI framework with derive macros |
| `serde` | 1 | Serialization/deserialization |
| `serde_yaml` | 0.9 | YAML frontmatter parsing |
| `serde_json` | 1 | JSON output generation |
| `toml` | 0.8 | Configuration file parsing |
| `anyhow` | 1 | Application error handling |
| `thiserror` | 2 | Library error definitions |
| `notify` | 8 | Cross-platform file watching |
| `tempfile` | 3 | Atomic file writes |
| `sha2` | 0.10 | Lockfile hash computation |
| `similar` | 2 | Diff generation |
| `dialoguer` | 0.11 | Interactive prompts |
| `crossterm` | 0.28 | Terminal control |
| `owo-colors` | 4 | Colored output |
| `dirs` | 6.0.0 | Platform-specific directories |
| `which` | 8 | Tool detection (rsync/scp) |
| `ctrlc` | 3 | Graceful shutdown |

### Dev Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `insta` | 1.44.3 | Snapshot testing |

### Build Profile (Release)

```toml
[profile.release]
lto = true           # Link-time optimization
codegen-units = 1    # Better optimization
strip = true         # Strip debug symbols
panic = "abort"      # Smaller binary
opt-level = "z"      # Optimize for size
```

**Binary Size**: ~1MB (optimized)

---

## Business Logic

### What Calvin Does

1. **Reads** prompt assets from `.promptpack/` directory
2. **Parses** Markdown files with YAML frontmatter
3. **Compiles** to platform-specific formats via adapters
4. **Syncs** generated files to target locations:
   - Project directories (in repository)
   - Home directories (user-level)
   - Remote servers (via SSH)
5. **Validates** security configurations
6. **Watches** for changes and auto-recompiles

### Core Flow

```
.promptpack/policies/security.md
         │
         ▼
  ┌─────────────────┐
  │  Source Parser  │  Read file → Extract frontmatter → Validate
  └────────┬────────┘
           │
           ▼
  ┌─────────────────┐
  │  PromptAsset    │  id, source_path, frontmatter, content
  └────────┬────────┘
           │
  ┌────────┴────────┬────────────────┬───────────────┐
  ▼                 ▼                ▼               ▼
ClaudeCode      Cursor          VSCode       Antigravity
Adapter         Adapter         Adapter        Adapter
  │                 │                │               │
  ▼                 ▼                ▼               ▼
.claude/       .cursor/rules/   .github/       .agent/
settings.json   security/       copilot-        rules/
                RULE.md         instructions.md
```

### Sync Decision Tree

```
For each target file:
  1. Exists? NO → Write, record hash
  2. In lockfile? NO → User-created, SKIP (or --force)
  3. Hash matches lockfile? YES → Safe to overwrite
     NO → User modified! → SKIP + WARNING
          Use --force to overwrite
```

---

## Integration Points

### External Services & APIs

| Integration | Type | Details |
|-------------|------|---------|
| SSH/SCP/rsync | System tools | Remote sync via shell commands |
| File system | Native | Cross-platform file operations |
| Terminal | Native | TTY detection, color support, animations |

### Input Sources

| Source | Format | Location |
|--------|--------|----------|
| Prompt assets | Markdown + YAML frontmatter | `.promptpack/` |
| Project config | TOML | `.promptpack/config.toml` |
| User config | TOML | `~/.config/calvin/config.toml` |
| Environment vars | `CALVIN_*` prefix | Shell environment |
| CLI arguments | clap derive | Command line |

### Output Targets

| Platform | Project Scope | User Scope |
|----------|---------------|------------|
| Claude Code | `.claude/commands/`, `.claude/settings.json` | `~/.claude/commands/` |
| Cursor | `.cursor/rules/`, `.cursor/commands/` | `~/.cursor/` |
| VS Code | `.github/copilot-instructions.md`, `AGENTS.md` | - |
| Antigravity | `.agent/rules/`, `.agent/workflows/` | `~/.gemini/` |
| Codex | `.codex/prompts/` | `~/.codex/prompts/` |

---

## Key Design Decisions

| ID | Decision | Rationale |
|----|----------|-----------|
| TD-1 | Rust language | Zero runtime deps, small binary, memory safety |
| TD-2 | clap with derive | Industry standard CLI framework |
| TD-3 | serde_yaml + custom extraction | Simple, no extra dependencies |
| TD-4 | Context-aware escaping | Prevent JSON/TOML corruption |
| TD-5 | notify crate | De-facto standard for file watching |
| TD-6 | System SSH tools | Respects user's ~/.ssh/config |
| TD-7 | TOML config format | Native to Rust ecosystem |
| TD-8 | NDJSON output | Streaming for CI integration |
| TD-9 | insta snapshot testing | Ideal for compiler output |
| TD-10 | anyhow + thiserror | Standard error handling pattern |
| TD-11 | GitHub Releases + cargo-binstall | Easy distribution |
| TD-12 | MCP server allowlist | Security for external tools |
| TD-13 | Three-tier security modes | Flexibility with sensible defaults |
| TD-14 | Atomic file writes | Prevent IDE lock conflicts |
| TD-15 | Lockfile with hashes | Detect user modifications |
| TD-16 | Hardcoded minimum deny | Prevent accidental secret exposure |
| TD-17 | Binary size optimization | ~1MB vs ~15-20MB unoptimized |
| TD-18 | Versioned adapters | Handle IDE format changes |

---

## Identified Technical Debt

### P0 - Critical (Before Release)

| Item | Status | Description |
|------|--------|-------------|
| Clippy warnings | 🔄 Pending | deploy targets need cleanup |
| Parser ID derivation | 🔄 Pending | Inconsistent ID handling |
| Doctor sink dedup | 🔄 Pending | Duplicate check results |

### P1 - High Priority

| Item | Status | Description |
|------|--------|-------------|
| Conflict resolution consolidation | 🔄 Pending | Multiple similar implementations |
| Security module split | 🔄 Pending | 1030 lines, could be modularized |
| Watch incremental parsing | 🔄 Pending | Performance improvements |

### P2 - Medium Priority

| Item | Status | Description |
|------|--------|-------------|
| Workspace rustfmt pass | 🔄 Pending | Code formatting consistency |
| Test coverage | 🔄 Pending | Currently ~57%, target >80% |
| Format versioning | 🔄 Pending | Read/validate format version |

---

## Test Coverage

| Category | Count | Description |
|----------|-------|-------------|
| Unit tests | 142 | Parser, adapters, models, config |
| CLI tests | 13 | Command-line argument parsing |
| Integration tests | 9 | Full compile cycles |
| **Total** | **164** | All passing ✅ |

### Testing Strategy

1. **Unit Tests**: In-module `#[cfg(test)]` blocks
2. **Snapshot Tests**: Using `insta` crate for generated output
3. **Golden Tests**: Reference `.promptpack/` → expected output
4. **Escaping Tests**: JSON with quotes corruption prevention
5. **Variant Tests**: Edge cases for sync engine and scope policy

---

## Recommendations for Onboarding

### Getting Started

1. **Install Rust**: `rustup install stable`
2. **Build**: `cargo build --release`
3. **Run tests**: `cargo test`
4. **Try it**: 
   ```bash
   ./target/release/calvin explain --brief
   ./target/release/calvin deploy --dry-run
   ```

### Key Files to Read First

1. `README.md` - Overview and quick start
2. `spec.md` - Original specification (Chinese)
3. `docs/architecture.md` - System design
4. `docs/tech-decisions.md` - Technology choices
5. `src/models.rs` - Core data structures
6. `src/adapters/mod.rs` - Adapter trait definition

### Development Workflow

1. Source files live in `.promptpack/`
2. Run `cargo test` before committing
3. Use `cargo clippy` for lint checks
4. Update `TODO.md` when completing tasks
5. Follow existing patterns in adapters/

### Common Tasks

| Task | Command |
|------|---------|
| Build release | `cargo build --release` |
| Run all tests | `cargo test` |
| Check lints | `cargo clippy` |
| Format code | `cargo fmt` |
| Preview changes | `./target/release/calvin diff` |
| Deploy locally | `./target/release/calvin deploy` |
| Deploy to home | `./target/release/calvin deploy --home` |

---

## File Statistics

| Category | Count |
|----------|-------|
| Source files (`.rs`) | ~80 |
| Total lines of code | ~12,000+ |
| Documentation files | 32+ |
| Test files | 19 |
| Example workflows | 36 |

### Largest Files

| File | Lines | Purpose |
|------|-------|---------|
| `src/sync/engine.rs` | 1413 | Unified sync engine |
| `src/security.rs` | 1030 | Doctor validation |
| `src/config.rs` | 789 | Configuration hierarchy |
| `src/watcher.rs` | 691 | File watcher |
| `src/cli.rs` | 499 | CLI argument parsing |

---

## CI/CD Status

| Workflow | Status | Description |
|----------|--------|-------------|
| `ci.yml` | ✅ Configured | Build and test |
| `release.yml` | ✅ Configured | Cross-compilation and releases |

### Release Targets

- `x86_64-apple-darwin` (Intel Mac)
- `aarch64-apple-darwin` (Apple Silicon)
- `x86_64-pc-windows-msvc` (Windows)
- `x86_64-unknown-linux-gnu` (Linux x64)
- `aarch64-unknown-linux-gnu` (Linux ARM)

### Distribution Channels

- GitHub Releases (pre-built binaries)
- Homebrew (`dist/homebrew/calvin.rb`)
- Scoop (`dist/scoop/calvin.json`)
- cargo-binstall support
- Docker (`Dockerfile`)

---

## Conclusion

Calvin is a well-architected, feature-complete PromptOps tool with:

- ✅ Clean separation of concerns (parser, adapters, sync engine)
- ✅ Comprehensive platform support (5 AI assistants)
- ✅ Strong security defaults with flexibility
- ✅ Cross-platform distribution
- ✅ Good test coverage with 164 passing tests

**Next Steps**:
1. Address P0 technical debt items
2. Increase test coverage to >80%
3. Complete IDE version detection (TD-18)
4. Finalize shell completions
