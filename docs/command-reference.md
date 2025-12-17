# Calvin Command Reference

> Complete reference for all Calvin CLI commands

## Synopsis

```bash
calvin [OPTIONS] <COMMAND>
```

## Global Options

| Option | Short | Description |
|--------|-------|-------------|
| `--verbose` | `-v` | Enable verbose output |
| `--json` | `-j` | Output in JSON format (machine-readable) |
| `--help` | `-h` | Print help information |
| `--version` | `-V` | Print version information |

---

## Commands

### `calvin sync`

Compile PromptPack sources and sync to all target platforms.

```bash
calvin sync [OPTIONS] [SOURCE]
```

**Arguments:**
- `SOURCE` - Path to `.promptpack` directory (default: `.promptpack`)

**Options:**
| Option | Description |
|--------|-------------|
| `--force`, `-f` | Overwrite files even if user-modified |
| `--dry-run`, `-n` | Preview changes without writing |
| `--targets`, `-t` | Comma-separated list of targets to sync |
| `--remote` | Sync to remote host via SSH (format: `user@host:/path`) |

**Examples:**
```bash
# Basic sync
calvin sync

# Preview changes
calvin sync --dry-run

# Only sync to Claude and Cursor
calvin sync --targets claude-code,cursor

# Sync to remote server
calvin sync --remote user@server:/home/user/project
```

---

### `calvin watch`

Watch for file changes and auto-sync.

```bash
calvin watch [OPTIONS] [SOURCE]
```

**Arguments:**
- `SOURCE` - Path to `.promptpack` directory (default: `.promptpack`)

**Options:**
| Option | Description |
|--------|-------------|
| `--targets`, `-t` | Comma-separated list of targets to sync |

**Output Events (NDJSON mode):**
```json
{"event":"started","source":".promptpack"}
{"event":"file_changed","path":"policies/security.md"}
{"event":"sync_started"}
{"event":"sync_complete","written":5,"skipped":2,"errors":0}
{"event":"shutdown"}
```

**Example:**
```bash
# Watch with JSON output for CI
calvin watch --json
```

---

### `calvin diff`

Show differences between current files and what would be generated.

```bash
calvin diff [OPTIONS] [SOURCE]
```

**Arguments:**
- `SOURCE` - Path to `.promptpack` directory (default: `.promptpack`)

**Options:**
| Option | Description |
|--------|-------------|
| `--targets`, `-t` | Comma-separated list of targets to check |

**Example:**
```bash
calvin diff
```

---

### `calvin doctor`

Validate configuration and security settings.

```bash
calvin doctor [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--security-mode` | Security level: `strict`, `balanced`, `yolo` (default: `balanced`) |

**Checks Performed:**
- Claude Code: commands synced, settings.json exists, deny list complete
- Cursor: rules synced, MCP servers validated
- VS Code: copilot-instructions.md exists
- Antigravity: rules synced, turbo mode detection

**Example:**
```bash
calvin doctor --security-mode strict
```

---

### `calvin audit`

Run security audit for CI/CD pipelines. Returns exit code 1 on violations.

```bash
calvin audit [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--security-mode` | Security level: `strict`, `balanced`, `yolo` |
| `--strict-warnings` | Treat warnings as errors (exit 1) |

**Exit Codes:**
- `0` - Audit passed
- `1` - Audit failed (errors or warnings with `--strict-warnings`)

**Example:**
```bash
# CI integration
calvin audit --strict-warnings || exit 1
```

---

### `calvin install`

Install prompts to user-level directories.

```bash
calvin install [OPTIONS] [SOURCE]
```

**Arguments:**
- `SOURCE` - Path to `.promptpack` directory (default: `.promptpack`)

**Options:**
| Option | Description |
|--------|-------------|
| `--user` | Install to user directories (`~/.claude/`, `~/.codex/`, etc.) |
| `--force`, `-f` | Overwrite existing files |

**User Directories:**
- Claude Code: `~/.claude/commands/`
- Codex: `~/.codex/prompts/`
- Antigravity: `~/.gemini/antigravity/`

**Example:**
```bash
calvin install --user
```

---

### `calvin parse`

Parse and validate PromptPack source files (debugging tool).

```bash
calvin parse [SOURCE]
```

**Arguments:**
- `SOURCE` - Path to `.promptpack` directory (default: `.promptpack`)

**Example:**
```bash
calvin parse --json
```

---

### `calvin migrate`

Migrate source format or adapter outputs between versions.

```bash
calvin migrate [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--format` | Target format version (e.g., `1.0`, `2.0`) |
| `--adapter` | Migrate specific adapter output |
| `--dry-run`, `-n` | Preview migration without changes |

**Example:**
```bash
calvin migrate --format 2.0 --dry-run
```

---

### `calvin version`

Display version information for Calvin and all supported adapters.

```bash
calvin version
```

**Output:**
```
Calvin v0.1.0

Adapters:
  claude-code  v1.0  (Claude Code 1.0.x)
  cursor       v1.0  (Cursor 0.44+)
  vscode       v1.0  (VS Code 1.96+)
  antigravity  v1.0  (Gemini CLI)
  codex        v1.0  (Codex CLI Experimental)
```

---

## Configuration File

Calvin can be configured via `.promptpack/config.toml`:

```toml
[targets]
enabled = ["claude-code", "cursor", "vscode"]

[security]
mode = "balanced"  # strict, balanced, yolo
deny = ["secrets/**", "*.credentials"]

[sync]
force = false
dry_run = false
atomic_writes = true
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `CALVIN_TARGETS` | Override enabled targets |
| `CALVIN_SECURITY_MODE` | Set security mode |
| `CALVIN_VERBOSE` | Enable verbose output (`1` or `true`) |
| `CALVIN_ATOMIC_WRITES` | Enable/disable atomic writes |

---

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Error (general failure, audit failure) |
| `2` | Invalid arguments |

---

## See Also

- [Architecture](architecture.md) - System design overview
- [Configuration](configuration.md) - Full configuration reference
- [Tech Decisions](tech-decisions.md) - Design rationale
