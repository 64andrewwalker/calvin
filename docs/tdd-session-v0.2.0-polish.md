# TDD Session: v0.2.0 Polish (P1–P4)

## Requirements covered

- P1: `calvin version` reports `v0.2.0` (bump crate version).
- P2: `calvin --help` mentions running without args for interactive mode.
- P3: `calvin check` includes a Codex section.
- P4: `calvin explain -v` shows richer examples.

## RED phase (tests added)

- `tests/cli_help.rs`: expects help output to include `Run 'calvin' without arguments for interactive setup.`
- `tests/cli_check.rs`: expects `calvin check` output to include `Codex`.
- `tests/cli_explain.rs`: expects `calvin explain -v` to include extra examples (targets + `--yes`).

## GREEN phase (implementation)

- Version bump:
  - Updated `Cargo.toml` crate version to `0.2.0` (and updated Insta snapshots accordingly).
- Help UX:
  - Added `after_help` text to Clap CLI definition (`src/cli.rs`) so `--help` advertises interactive mode.
- Check output:
  - Added Codex checks to the doctor/check pipeline (`src/security.rs`) and included both project-scope and user-scope prompt locations.
- Explain UX:
  - Expanded verbose examples in `src/commands/explain.rs` for common workflows (`deploy --targets`, `deploy --yes`, `deploy --home`, `deploy --remote`, `diff`).

## REFACTOR / docs sync

- Updated docs snippets that referenced `v0.1.0` to `v0.2.0` where they represent current CLI output/examples.

## Result

- New tests pass and `cargo test` / `cargo clippy -D warnings` are green.

