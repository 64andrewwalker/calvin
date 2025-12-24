# Calvin Configuration Reference

## Configuration Hierarchy

Calvin reads configuration from:

1. **Built-in defaults** (lowest priority)
2. **User config**: `~/.config/calvin/config.toml` (XDG)
3. **Project config**: `.promptpack/config.toml`
4. **Environment variables**: `CALVIN_*`
5. **CLI arguments** (highest priority)

### Merge Rules

- Project config overrides user config at the **top-level section** granularity (no deep merge).
- Exception: `[sources]` is merged so project ignore flags can coexist with user-defined paths.

Example (PRD §11.2):

```toml
# ~/.config/calvin/config.toml
[security]
mode = "balanced"
allow_naked = true

# .promptpack/config.toml
[security]
mode = "strict"  # overrides the entire [security] section
```

Effective result:
- `security.mode = strict` (project)
- `security.allow_naked = false` (default, because project section overrides)

---

## Configuration File Format

Calvin uses TOML for configuration files.

### Full Configuration Example

```toml
# .promptpack/config.toml

#───────────────────────────────────────────────────────────────
# FORMAT VERSION
#───────────────────────────────────────────────────────────────
[format]
version = "1.0"           # Required: source format version

#───────────────────────────────────────────────────────────────
# SECURITY SETTINGS
#───────────────────────────────────────────────────────────────
[security]
mode = "balanced"         # "yolo" | "balanced" | "strict"
allow_naked = false       # true = disable even minimum protections (dangerous!)

# Custom deny patterns (added to hardcoded minimum)
deny = ["*.secret", "credentials/**"]

# MCP server allowlist helpers for Cursor MCP checks
[security.mcp]
allowlist = ["filesystem", "github"]
additional_allowlist = ["my-internal-mcp"]

#───────────────────────────────────────────────────────────────
# TARGET ADAPTERS
#───────────────────────────────────────────────────────────────
[targets]
# Specify which platforms to deploy to. Default: all platforms.
# Valid values: claude-code (or "claude"), cursor, vscode, antigravity, codex, all
enabled = ["claude-code", "cursor", "vscode", "antigravity", "codex"]

# Semantic notes:
# - enabled = ["claude"]       → Deploy only to Claude Code (alias supported)
# - enabled = []               → Deploy to NO targets (explicitly disabled)
# - enabled field missing      → Deploy to ALL targets (default behavior)

#───────────────────────────────────────────────────────────────
# SYNC BEHAVIOR
#───────────────────────────────────────────────────────────────
[sync]
atomic_writes = true      # Use temp file + rename (recommended)
respect_lockfile = true   # Warn on user-modified files

#───────────────────────────────────────────────────────────────
# OUTPUT
#───────────────────────────────────────────────────────────────
[output]
verbosity = "normal"      # "quiet" | "normal" | "verbose" | "debug"

#───────────────────────────────────────────────────────────────
# MCP SERVERS (OPTIONAL)
#───────────────────────────────────────────────────────────────
[mcp.servers.github]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
```

---

## Environment Variables

Calvin’s config module supports environment overrides (prefix `CALVIN_`), but not every CLI code path applies them yet.

Supported env vars:

| Variable | Equivalent | Example |
|----------|------------|---------|
| `CALVIN_SECURITY_MODE` | `security.mode` | `strict` |
| `CALVIN_TARGETS` | `targets.enabled` | `claude-code,cursor` |
| `CALVIN_VERBOSITY` | `output.verbosity` | `debug` |
| `CALVIN_ATOMIC_WRITES` | `sync.atomic_writes` | `false` |

---

## CLI Arguments

CLI arguments override all other configuration sources.

```bash
# Security mode
calvin check --mode strict

# Target selection
calvin deploy --targets claude-code,cursor

# Output
calvin deploy --json
calvin deploy -vv

# Deploy behavior
calvin deploy --force            # Overwrite user-modified files
calvin deploy --yes              # Non-interactive (auto-confirm overwrites)
calvin deploy --dry-run          # Preview changes
calvin deploy --remote user@host:/path
```

---

## Configuration Locations

### User Configuration

| OS | Path |
|----|------|
| macOS | `~/.config/calvin/config.toml` |
| Linux | `~/.config/calvin/config.toml` (XDG compliant) |
| Windows | `%APPDATA%\calvin\config.toml` |

### Project Configuration

Always at `.promptpack/config.toml` relative to project root.

---

## Validation

Calvin validates source files and parses configuration during commands such as `deploy`/`diff`.

```bash
$ calvin deploy

⚠ Unknown config key 'securty' in .promptpack/config.toml:5
   Did you mean 'security'?
```

---

## Defaults

If no configuration is provided, Calvin uses these defaults:

```toml
[format]
version = "1.0"

[security]
mode = "balanced"
allow_naked = false

[targets]
# When 'enabled' field is missing, deploys to all platforms
enabled = ["claude-code", "cursor", "vscode", "antigravity", "codex"]

[sync]
atomic_writes = true
respect_lockfile = true

[output]
verbosity = "normal"
```

---

## Target Name Reference

| Name | Aliases | Platform |
|------|---------|----------|
| `claude-code` | `claude` | Claude Code (Anthropic) |
| `cursor` | - | Cursor IDE |
| `vscode` | `vs-code` | VS Code with GitHub Copilot |
| `antigravity` | - | Google Antigravity/Gemini |
| `codex` | - | OpenAI Codex CLI |
| `all` | - | All platforms (meta-target) |
