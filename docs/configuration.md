# Calvin Configuration Reference

## Configuration Hierarchy

Calvin loads configuration from multiple sources, with later sources overriding earlier ones:

1. **Built-in defaults** (lowest priority)
2. **User config**: `~/.config/calvin/config.toml`
3. **Project config**: `.promptpack/config.toml`
4. **Environment variables**: `CALVIN_*` prefix
5. **CLI arguments** (highest priority)

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
deny = [
    "*.secret",
    "credentials/**",
]

# MCP server allowlist (required in strict mode)
[security.mcp]
allowlist = [
    "filesystem",
    "github",
]

#───────────────────────────────────────────────────────────────
# TARGET ADAPTERS
#───────────────────────────────────────────────────────────────
[targets]
enabled = ["claude-code", "cursor", "vscode", "antigravity"]  # default: all
# disabled = ["codex"]  # alternative: disable specific targets

[targets.claude-code]
enabled = true
# output_dir = ".claude"  # override default

[targets.cursor]
enabled = true
# min_version = "2.0"  # require minimum IDE version

[targets.vscode]
enabled = true
merge_mode = "single"     # "single" = one copilot-instructions.md, "split" = per-file

[targets.antigravity]
enabled = true

[targets.codex]
enabled = false           # user-scope only, disabled by default

#───────────────────────────────────────────────────────────────
# SYNC BEHAVIOR
#───────────────────────────────────────────────────────────────
[sync]
atomic_writes = true      # Use temp file + rename (recommended)
respect_lockfile = true   # Warn on user-modified files
force = false             # Overwrite user-modified files without asking

[sync.watch]
debounce_ms = 100         # Debounce rapid file changes
ignore = [                # Patterns to ignore in watch mode
    "**/.git/**",
    "**/node_modules/**",
]

#───────────────────────────────────────────────────────────────
# REMOTE SYNC
#───────────────────────────────────────────────────────────────
[remote]
# Default remote for `calvin sync --remote`
# host = "user@server.example.com:/path/to/project"
prefer_rsync = true       # Try rsync before scp

#───────────────────────────────────────────────────────────────
# OUTPUT
#───────────────────────────────────────────────────────────────
[output]
json = false              # Machine-readable output
color = "auto"            # "auto" | "always" | "never"
verbosity = "normal"      # "quiet" | "normal" | "verbose" | "debug"
```

---

## Environment Variables

All environment variables use the `CALVIN_` prefix.

| Variable | Config Equivalent | Example |
|----------|-------------------|---------|
| `CALVIN_SECURITY_MODE` | `security.mode` | `yolo` |
| `CALVIN_OUTPUT_JSON` | `output.json` | `true` |
| `CALVIN_OUTPUT_VERBOSITY` | `output.verbosity` | `debug` |
| `CALVIN_SYNC_FORCE` | `sync.force` | `true` |
| `CALVIN_TARGETS_ENABLED` | `targets.enabled` | `claude-code,cursor` |

### Mapping Rules

- Nested keys use `_` separator: `[security] mode` → `CALVIN_SECURITY_MODE`
- Arrays use comma separation: `CALVIN_TARGETS_ENABLED=claude-code,cursor`
- Booleans: `true`, `false`, `1`, `0`

---

## CLI Arguments

CLI arguments override all other configuration sources.

```bash
# Security mode
calvin sync --security-mode yolo

# Target selection
calvin sync --targets claude-code,cursor
calvin sync --disable-target codex

# Output
calvin sync --json
calvin sync --verbose
calvin sync --quiet

# Sync behavior
calvin sync --force              # Overwrite user-modified files
calvin sync --dry-run            # Preview changes
calvin sync --remote user@host:/path
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

Calvin validates configuration on startup:

```bash
$ calvin sync

✗ ERROR: Invalid configuration
  .promptpack/config.toml:5:1
  Unknown field 'securty' - did you mean 'security'?

$ calvin doctor --config

Checking configuration...
  ✓ User config: ~/.config/calvin/config.toml
  ✓ Project config: .promptpack/config.toml
  ✓ Format version: 1.0 (supported)
  ⚠ Security mode: yolo (no enforcement)
  ✓ Targets: 4 enabled
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
enabled = ["claude-code", "cursor", "vscode", "antigravity"]

[sync]
atomic_writes = true
respect_lockfile = true
force = false

[output]
json = false
color = "auto"
verbosity = "normal"
```
