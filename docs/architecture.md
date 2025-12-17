# Calvin Architecture

> *Making agents behave.*

## Overview

Calvin is a PromptOps compiler and synchronization tool that enables teams to maintain AI rules, commands, and workflows in a single source format (PromptPack), then compile and distribute them to multiple AI coding assistant platforms.

## System Context

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         DEVELOPMENT TEAM                                │
│                                                                         │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐                │
│  │  Developer A │   │  Developer B │   │  CI/CD       │                │
│  │  (macOS)     │   │  (Windows)   │   │  (Linux)     │                │
│  └──────┬───────┘   └──────┬───────┘   └──────┬───────┘                │
│         │                  │                  │                        │
│         └──────────────────┼──────────────────┘                        │
│                            │                                           │
│                            ▼                                           │
│                   ┌────────────────┐                                   │
│                   │     CALVIN     │                                   │
│                   │   (calvin)  │                                   │
│                   └────────┬───────┘                                   │
│                            │                                           │
│         ┌──────────────────┼──────────────────┐                        │
│         ▼                  ▼                  ▼                        │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐                  │
│  │ Claude Code │   │   Cursor    │   │  VS Code +  │   ...            │
│  │             │   │             │   │  Copilot    │                  │
│  └─────────────┘   └─────────────┘   └─────────────┘                  │
└─────────────────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Source Parser

**Responsibility**: Parse PromptPack source files (Markdown with YAML frontmatter)

**Input**: `.promptpack/` directory structure
```
.promptpack/
├── policies/     # Long-term rules (code style, security, testing)
├── actions/      # Triggered commands (generate tests, PR review)
├── agents/       # Specialized sub-agents / roles
└── mcp/          # MCP server configurations
```

**Output**: Validated AST of prompt assets with resolved references

**Key Decisions**:
- D1: Frontmatter is REQUIRED - files without valid YAML frontmatter are rejected
- D2: Circular references between assets are compile-time errors
- D3: Input parameters use `$ARGUMENTS` by default for maximum portability

### 2. Compiler Engine

**Responsibility**: Transform parsed assets into platform-specific outputs

**Key Abstractions**:
```rust
trait TargetAdapter {
    /// Generate platform-specific files from a prompt asset
    fn compile(&self, asset: &PromptAsset, config: &Config) -> Vec<OutputFile>;
    
    /// Validate generated output against platform best practices
    fn validate(&self, output: &OutputFile) -> Vec<Diagnostic>;
    
    /// Generate security baseline configuration
    fn security_baseline(&self) -> Vec<OutputFile>;
}
```

**Adapter Responsibilities**:

| Adapter | Output Directories | Key Transformations |
|---------|-------------------|---------------------|
| `ClaudeCodeAdapter` | `.claude/commands/`, `.claude/settings.json` | Slash commands, permission deny lists |
| `CursorAdapter` | `.cursor/rules/<id>/RULE.md`, `.cursor/commands/` | Rule frontmatter (globs, alwaysApply) |
| `VSCodeAdapter` | `.github/copilot-instructions.md`, `.github/instructions/` | Instruction files with applyTo |
| `AntigravityAdapter` | `.agent/rules/`, `.agent/workflows/` | Rules and workflows |
| `CodexAdapter` | `~/.codex/prompts/` | User-level prompts with $ARGUMENTS |

### 3. Sync Engine

**Responsibility**: Write compiled outputs to target locations idempotently

**Sync Modes**:
- `sync`: One-shot generation and write
- `watch`: Continuous incremental sync on file changes
- `diff`: Preview changes without writing
- `doctor`: Validate existing configurations

**Key Decisions**:
- D4: All generated files include a header marker to prevent accidental overwrites
- D5: Non-marked files are NEVER overwritten without `--force`
- D6: Line endings default to LF (configurable to CRLF for Windows)

### 4. Security Module

**Responsibility**: Provide optional security baselines (configurable strictness)

**Security Modes**:
| Mode | Behavior |
|------|----------|
| `strict` | Enforce deny lists, block unknown MCP, warn on turbo |
| `balanced` | Generate deny lists, warn but don't block |
| `yolo` | No security enforcement, user takes responsibility |

**Default**: `balanced` (generates protections, doesn't block)

**Example Configuration**:
```toml
# .promptpack/config.toml
[security]
mode = "yolo"  # "strict" | "balanced" | "yolo"

# Or per-platform overrides:
[security.claude]
mode = "balanced"
deny = [".env", "secrets/**"]  # Optional custom list
```

**When `balanced` or `strict`** - auto-generates:
```yaml
# .claude/settings.json
permissions:
  deny:
    - ".env"
    - ".env.*"
    - "secrets/**"
    - "*.pem"
    - "*.key"
```

**Doctor Behavior by Mode**:
- `strict`: ERROR on missing deny lists, BLOCK unknown MCP
- `balanced`: WARNING on missing protections
- `yolo`: INFO only, no enforcement

## Data Flow

```
┌──────────────────────────────────────────────────────────────────────┐
│                         COMPILATION FLOW                             │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  .promptpack/policies/security.md                                    │
│           │                                                          │
│           ▼                                                          │
│  ┌─────────────────┐                                                 │
│  │  Source Parser  │  1. Read file                                   │
│  │                 │  2. Extract YAML frontmatter                    │
│  │                 │  3. Validate required fields                    │
│  └────────┬────────┘  4. Parse content body                          │
│           │                                                          │
│           ▼                                                          │
│  ┌─────────────────┐                                                 │
│  │ PromptAsset {   │                                                 │
│  │   id: "security"│                                                 │
│  │   kind: Policy  │                                                 │
│  │   scope: Project│                                                 │
│  │   targets: [*]  │                                                 │
│  │   content: ...  │                                                 │
│  │ }               │                                                 │
│  └────────┬────────┘                                                 │
│           │                                                          │
│  ┌────────┴────────┬────────────────┬───────────────┐                │
│  ▼                 ▼                ▼               ▼                │
│ ClaudeCode      Cursor          VSCode       Antigravity             │
│ Adapter         Adapter         Adapter        Adapter               │
│  │                 │                │               │                │
│  ▼                 ▼                ▼               ▼                │
│ .claude/       .cursor/rules/   .github/       .agent/rules/         │
│ settings.json   security/       copilot-        security.md          │
│                 RULE.md         instructions.md                      │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

## Key Design Decisions

| ID | Decision | Rationale |
|----|----------|-----------|
| D1 | Frontmatter required | Explicit > implicit; reject ambiguity early |
| D2 | No circular refs | Predictable compilation order |
| D3 | `$ARGUMENTS` default | Most portable across Claude/Cursor/Codex |
| D4 | Generated file headers | Protect user edits from overwrites |
| D5 | Never overwrite unmarked | User trust; avoid "surprise deletions" |
| D6 | LF default line endings | Cross-platform git consistency |
| D7 | Security baseline mandatory | "Safe by default" - deny lists auto-generated |
| D8 | Merge policies by default | Determinism over granularity |

## Security Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                      SECURITY LAYERS                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Layer 1: SOURCE VALIDATION                                        │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │ • Reject unknown/unsafe MCP servers (whitelist enforcement)    │ │
│  │ • Validate agent definitions don't escalate permissions        │ │
│  │ • Lint for secrets/keys in source content                     │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                     │
│  Layer 2: COMPILATION ENFORCEMENT                                   │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │ • Auto-inject deny lists (Claude Code permissions.deny)       │ │
│  │ • Force non-Turbo mode (Antigravity)                          │ │
│  │ • Warn on overlapping instruction file scopes (VS Code)       │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                     │
│  Layer 3: DOCTOR VALIDATION                                         │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │ • ERROR: Missing deny lists                                   │ │
│  │ • WARNING: Turbo mode detected                                │ │
│  │ • BLOCK: Unknown MCP server in output                         │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Extension Points

1. **Custom Adapters**: Implement `TargetAdapter` trait for new platforms
2. **Validation Rules**: Add platform-specific lint rules via config
3. **Template Overrides**: Per-project templates in `.promptpack/templates/`
4. **MCP Allowlist**: Extensible server allowlist in config

## Cross-Cutting Concerns

### Logging & Observability
- Structured JSON logs with `--json` flag for CI
- Verbosity levels: `-v` (info), `-vv` (debug), `-vvv` (trace)

### Error Handling
- Fail-fast on validation errors
- Collect warnings and emit summary
- Exit code conventions: 0=success, 1=validation error, 2=system error

### Configuration Hierarchy
```
1. CLI flags (highest priority)
2. Environment variables (CALVIN_*)
3. Project config (.promptpack/config.toml)
4. User config (~/.config/calvin/config.toml)
5. Built-in defaults (lowest priority)
```
