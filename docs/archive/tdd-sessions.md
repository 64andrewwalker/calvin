# TDD Session Logs

> This document consolidates historical TDD session logs from Calvin's development.
> For substantial feature-specific sessions, see separate files:
> - `tdd-session-cli-animation.md` (CLI animation system)
> - `tdd-session-sync-refactor.md` (Two-stage sync engine)

---

## Phase 1-3: Parser Implementation

> **Date**: 2025-12-17
> **Status**: ✅ Complete (102 tests passing)

### Requirements Covered

From `TODO.md` Phase 1:
- [x] Define core data structures (`PromptAsset`, `Frontmatter`, `Config`)
- [x] Implement frontmatter extraction (`---` delimiter parsing)
- [x] Implement YAML parsing with serde
- [x] Add validation for required field (`description`)
- [x] Parse `.promptpack/` directory structure recursively
- [x] Write unit tests for parser

From `docs/implementation-plan.md`:
- [x] `Frontmatter` struct with fields: description, targets, scope, apply
- [x] `AssetKind` enum: Policy, Action, Agent
- [x] `Scope` enum: Project, User
- [x] `Target` enum: ClaudeCode, Cursor, VSCode, Antigravity, Codex, All

### TDD Cycles

1. **Core Data Structures**: Define `Frontmatter` struct with minimal required field
2. **Frontmatter Extraction**: Extract YAML frontmatter between `---` delimiters
3. **Required Field Validation**: Reject files missing `description` field
4. **Directory Parsing**: Recursively parse `.promptpack/` directory

### Implementation Decisions

1. Keep `description` as the only required field (per spec)
2. All other frontmatter fields are optional with sensible defaults
3. Use `serde_yaml` for YAML parsing
4. Use `walkdir` or manual recursion for directory parsing

---

## Sprint 1: US-1 + US-4

> **Date**: 2025-12-17
> **Status**: ✅ Complete

### Requirements Covered

From `docs/ux-review.md`:
- [x] US-1: `security.deny.exclude` can remove minimum deny patterns
- [x] US-1: `security.allow_naked = true` disables hardcoded deny injection
- [x] US-1: `calvin doctor` reflects custom deny rules
- [x] US-4: `calvin sync --interactive` prompts on conflicts with `o/s/d/a/A`
- [x] US-4: `diff` shows a unified diff and re-prompts
- [x] US-4: `All` applies the choice to subsequent conflicts

### Tests Written

- Config: `test_config_parse_security_deny_table_with_exclude`, `test_config_parse_security_deny_array_compat`
- Security baseline: deny pattern tests (3)
- Claude adapter: security baseline tests (2)
- Security: deny list tests (3)
- Sync: interactive conflict tests (5)
- CLI: interactive flag parsing tests (2)

### Implementation Decisions

1. **Config compatibility**: Introduced `security.deny` as structured config while keeping legacy array support
2. **Single source of truth**: Centralized deny list calculation in `src/security_baseline.rs`
3. **Exclude semantics**: Removes patterns matching excluded paths
4. **Interactive sync**: Added `SyncPrompter` abstraction with `StdioPrompter` implementation

---

## Sprint 2: US-2 + US-9

> **Date**: 2025-12-17
> **Status**: ✅ Complete

### Requirements Covered

From `docs/ux-review.md`:
- [x] US-2: `security.mcp.additional_allowlist` allows internal MCP servers
- [x] US-2: `calvin doctor -v` shows MCP server match details
- [x] US-9: YAML frontmatter syntax errors include line number + hint
- [x] US-9: Unknown config keys surface warning + "did you mean…" suggestion

### Tests Written

- Security: MCP allowlist tests (1)
- Config: MCP config parsing, unknown key warning tests (2)
- CLI: global flag positioning tests (2)
- Parser: frontmatter error formatting, directory fail-fast tests (2)

### Implementation Decisions

1. **MCP allowlist**: Added `[security.mcp] allowlist` and `additional_allowlist`
2. **Verbose output**: Added `details: Vec<String>` to `SecurityCheck`
3. **Config warnings**: Implemented `Config::load_with_warnings()` with Levenshtein suggestions
4. **Parser strictness**: `parse_directory()` now fails fast on invalid files

---

## v0.2.0 Phase 0: CLI Refactor

> **Date**: 2025-12-17
> **Status**: ✅ Complete

### Requirements Covered

- Split `src/main.rs` "god object" into modules (~1151 → ~123 lines)
- Introduced unified commands: `deploy`, `check`, `explain`
- Backward compatibility: legacy commands hidden but functional with warnings
- Added missing Cargo feature declaration (`claude` feature)

### New Module Structure

- `src/cli.rs`: Clap CLI definition + parsing tests
- `src/commands/deploy.rs`: Unified deploy command
- `src/commands/check.rs`: Unified check command
- `src/commands/explain.rs`: AI self-description
- `src/commands/watch.rs`: Watch mode
- `src/commands/debug.rs`: Debug utilities
- `src/ui/menu.rs`: Interactive menu helpers
- `src/ui/output.rs`: Output formatting

---

## v0.2.0 Phase 1: Interactive Entry

> **Date**: 2025-12-17
> **Status**: ✅ Complete

### Requirements Covered

- `calvin` with no subcommand enters interactive mode
- Project state detection: `NoPromptPack`, `EmptyPromptPack`, `Configured(total)`
- Interactive menus: first-run and existing-project variants
- Setup wizard: writes `.promptpack/config.toml` and templates
- Non-interactive safe path: `--json` outputs structured state JSON

### Implementation

- `src/state.rs`: `detect_state()` via filesystem scan
- `src/commands/interactive.rs`: Menus and wizard
- `src/main.rs`: `None` subcommand routes to interactive mode

---

## v0.2.0 Phase 3: Error Enhancements

> **Date**: 2025-12-17
> **Status**: ✅ Complete

### Requirements Covered

- Standardized fatal error format: `[ERROR] <title>` + location + body + `FIX:` suggestion
- "Press Enter to open in editor" flow for file-based errors (TTY only)
- `--json` mode remains non-interactive with structured error output

### Implementation

- `src/ui/error.rs`: `format_calvin_error()`, `print_error()`, `offer_open_in_editor()`
- Open-in-editor uses heuristics for popular editors (`code`, `cursor`, `vim` family)

---

## v0.2.0 Phase 4: Sync Module Refactor

> **Date**: 2025-12-17
> **Status**: ✅ Complete

### Goals

- Split `src/sync/mod.rs` (~900 LOC) into smaller modules
- Keep behavior identical and all tests green

### Implementation

- `src/sync/compile.rs`: `compile_assets()` (97 lines)
- `src/sync/conflict.rs`: `ConflictReason`, `ConflictChoice`, `SyncPrompter` (85 lines)
- `src/sync/tests.rs`: Unit tests (461 lines)
- `src/sync/mod.rs`: Reduced to ~300 LOC

---

## v0.2.0 Phase 5: Docs Cleanup + `--yes` Semantics

> **Date**: 2025-12-17
> **Status**: ✅ Complete

### Goals

- Update docs to v0.2.0 command set (`deploy`/`check`)
- Fix `calvin deploy --yes` to be truly non-interactive

### Tests Written

- `tests/cli_yes.rs`: Verifies `--yes` auto-confirms overwrites

### Implementation

- `--yes` treated as "assume yes" for conflict resolution
- Disabled interactive prompting when `--json`, `--yes`, or stdin is not TTY
- Updated target selection helper to suppress prompts with `--yes`

---

## v0.2.0 Polish (P1–P4)

> **Date**: 2025-12-17
> **Status**: ✅ Complete

### Requirements Covered

- P1: `calvin version` reports `v0.2.0`
- P2: `calvin --help` mentions interactive mode
- P3: `calvin check` includes Codex section
- P4: `calvin explain -v` shows richer examples

### Tests Written

- `tests/cli_help.rs`: Help output includes interactive mode hint
- `tests/cli_check.rs`: Check output includes `Codex`
- `tests/cli_explain.rs`: Verbose examples include targets + `--yes`

### Implementation

- Version bump to `0.2.0` in `Cargo.toml`
- Added `after_help` text to Clap CLI definition
- Added Codex checks to doctor pipeline
- Expanded verbose examples in `explain.rs`
