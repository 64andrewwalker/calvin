# TDD Session: v0.2.0 Phase 1 (Interactive entry)

## Requirements covered

- `calvin` with no subcommand enters interactive mode (Phase 1).
- Added project state detection:
  - `NoPromptPack` when `.promptpack/` is missing
  - `EmptyPromptPack` when `.promptpack/` exists but has no prompt markdown files
  - `Configured(total)` when prompt markdown files exist
- Implemented interactive menus:
  - First-run menu (no `.promptpack/`)
  - Existing-project menu (with `.promptpack/`)
  - Setup wizard writes `.promptpack/config.toml` and optional example templates
- Non-interactive safe path:
  - `calvin --json` with no subcommand outputs structured state JSON and exits.

## RED phase (tests added)

- CLI parsing: allow no subcommand (`Cli.command: Option<Commands>`) and test `Cli::try_parse_from(["calvin"])`.
- Project state detection tests in `src/state.rs`.
- Setup output tests in `src/commands/interactive.rs`:
  - Creates config + templates
  - Empty template selection creates no action files
  - Does not overwrite existing `config.toml`
  - Minimal security sets `allow_naked = true`

## GREEN phase (implementation)

- `src/state.rs` implemented `detect_state()` via filesystem scan.
- `src/commands/interactive.rs` added:
  - `cmd_interactive()` entrypoint
  - First-run / existing-project menus
  - 3-step setup wizard that creates `.promptpack/config.toml` and example actions
- `src/main.rs` dispatch updated:
  - `None` subcommand routes to interactive mode
  - Existing subcommands unchanged

## REFACTOR notes

- Kept interactive mode JSON-safe: no TTY required when `--json` is used.
- Reused existing deploy/check/diff/watch/explain implementations from menus (no duplicated business logic).

## Validation

- `cargo test` passes.
- `cargo clippy` is clean.

