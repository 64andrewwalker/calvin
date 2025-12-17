# TDD Session: v0.2.0 Phase 0 (CLI refactor)

## Requirements covered

- Split `src/main.rs` “god object” into modules:
  - `src/main.rs` now only parses CLI + dispatches commands.
  - `src/cli.rs` contains the Clap CLI definition + CLI parsing tests.
  - `src/commands/*` contains command implementations.
  - `src/ui/*` contains terminal/UI helpers used by commands.
- Introduced new unified commands per `docs/calvin`:
  - `calvin deploy` (project / `--home` / `--remote`)
  - `calvin check` (replaces doctor + audit behavior; non-zero exit on violations)
  - `calvin explain` (human-readable + `--brief` + `--json`)
- Backward compatibility (DP-10):
  - Legacy commands (`sync`, `install`, `doctor`, `audit`, `migrate`, `parse`) remain available but are hidden from `--help`.
  - Running legacy commands prints deprecation warnings (suppressed in `--json` mode).
- Quality:
  - Added missing Cargo feature declaration to silence `unexpected_cfgs` warning (`claude` feature).
  - `cargo test` and `cargo clippy` both run clean.

## Tests written (RED → GREEN)

- **RED**
  - Added failing CLI tests to assert `calvin deploy`, `calvin check`, `calvin explain` are valid subcommands.
- **GREEN**
  - Implemented the new subcommands and wired them through `src/main.rs` dispatch.
  - Strengthened tests to assert parsed command variants/flags:
    - `deploy` defaults
    - `check` defaults
    - `explain` defaults + `--brief`

Tests live in `src/cli.rs` under `#[cfg(test)]`.

## Implementation decisions

- Keep `--json` and `-v/-vv/-vvv` as global flags; `explain` uses the global verbosity level to show more detail.
- Implement `deploy` as the single shared deploy path with:
  - Target selection (project/home/remote)
  - Optional scope policy for deprecated `install` compatibility
  - Shared compilation + sync pipeline
- Implement `check` as “doctor-style output + audit-style exit codes” to unify both use cases.

## Refactoring notes

- New modules created:
  - `src/commands/deploy.rs`
  - `src/commands/check.rs`
  - `src/commands/explain.rs`
  - `src/commands/watch.rs`
  - `src/commands/debug.rs`
  - `src/ui/menu.rs`
  - `src/ui/output.rs`
- `README.md` updated to use `deploy`/`check` instead of deprecated commands.

## Follow-ups (next phases)

- Phase 1: interactive no-args entry (`calvin` → menu), plus `state.rs`.
- Phase 4: split `src/sync/mod.rs` into smaller modules.
- Phase 5: finish docs/help cleanup and remove fully deprecated code when ready.

