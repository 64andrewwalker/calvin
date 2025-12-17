# Calvin Command Reference

> Reference for Calvin CLI commands (v0.2.0+)

## Synopsis

```bash
calvin [OPTIONS] [COMMAND]
```

**No command**:
- TTY: opens an interactive menu
- `--json`: prints detected project state as JSON

## Global Options

| Option | Short | Description |
|--------|-------|-------------|
| `--json` | - | Output machine-readable JSON |
| `--verbose` | `-v` | Verbosity level (`-v`, `-vv`, `-vvv`) |
| `--help` | `-h` | Print help information |
| `--version` | `-V` | Print version information |

---

## Core Commands

### `calvin deploy`

Deploy PromptPack assets (replaces `sync` + `install`).

```bash
calvin deploy [OPTIONS]
```

**Options:**
| Option | Short | Description |
|--------|-------|-------------|
| `--source <PATH>` | `-s` | Path to `.promptpack` directory (default: `.promptpack`) |
| `--targets <LIST>` | `-t` | Comma-separated targets (e.g. `claude-code,cursor`) |
| `--home` | - | Deploy to user home directory |
| `--remote <DEST>` | - | Deploy to remote destination (`user@host:/path`) |
| `--force` | `-f` | Force overwrite of modified files |
| `--yes` | `-y` | Non-interactive; auto-confirm overwrites |
| `--dry-run` | - | Preview changes without writing |

**Examples:**
```bash
calvin deploy
calvin deploy --targets claude-code,cursor
calvin deploy --home --yes
calvin deploy --remote user@server:/home/user/project --yes
calvin deploy --dry-run
```

---

### `calvin check`

Check configuration and security (replaces `doctor` + `audit`).

```bash
calvin check [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--mode <MODE>` | Security mode: `balanced` (default), `strict`, `yolo` |
| `--strict-warnings` | Fail on warnings too (CI mode) |

**Examples:**
```bash
calvin check
calvin check --mode strict
calvin check --strict-warnings
```

---

### `calvin explain`

Explain Calvin's usage (for humans / AI assistants).

```bash
calvin explain [OPTIONS]
```

**Options:**
| Option | Description |
|--------|-------------|
| `--brief` | Short version (just the essentials) |

**Examples:**
```bash
calvin explain
calvin explain --brief
calvin explain --json
```

---

### `calvin watch`

Watch `.promptpack/` for changes and deploy continuously.

```bash
calvin watch [OPTIONS]
```

**Options:**
| Option | Short | Description |
|--------|-------|-------------|
| `--source <PATH>` | `-s` | Path to `.promptpack` directory (default: `.promptpack`) |

**JSON output (NDJSON):**
```json
{"event":"started","source":".promptpack"}
{"event":"file_changed","path":"policies/security.md"}
{"event":"sync_started"}
{"event":"sync_complete","written":5,"skipped":2,"errors":0}
{"event":"shutdown"}
```

---

### `calvin diff`

Preview what would be generated, without writing.

```bash
calvin diff [OPTIONS]
```

**Options:**
| Option | Short | Description |
|--------|-------|-------------|
| `--source <PATH>` | `-s` | Path to `.promptpack` directory (default: `.promptpack`) |

---

### `calvin version`

Show Calvin + adapter version information.

```bash
calvin version [--json]
```

---

## Deprecated / Hidden Commands

These commands are kept for backward compatibility but are hidden from `--help` and print a deprecation warning (suppressed in `--json`).

| Deprecated | Replacement |
|-----------:|-------------|
| `calvin sync` | `calvin deploy` |
| `calvin install` | `calvin deploy --home` |
| `calvin doctor` | `calvin check` |
| `calvin audit` | `calvin check --strict-warnings` |

Debug/placeholder commands (no stable API yet): `calvin parse`, `calvin migrate`.

---

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Runtime error or failed check |
| `2` | Invalid arguments |

---

## See Also

- [Configuration](configuration.md) - Configuration file reference
- [Architecture](architecture.md) - System design overview
