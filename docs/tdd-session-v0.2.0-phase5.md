# TDD Session: v0.2.0 Phase 5 (docs cleanup + `--yes` semantics)

## Goals

- Finish Phase 5 documentation cleanup: update docs to the v0.2.0 command set (`deploy`/`check`) and current config surface.
- Fix `calvin deploy --yes` to be truly non-interactive and to auto-confirm overwrites (as documented).

## RED phase (failing test)

- Added integration test `tests/cli_yes.rs`:
  - Creates a PromptPack, runs `calvin deploy` once to create the lockfile/output.
  - Modifies the generated file.
  - Runs `calvin deploy --yes` and asserts the conflict is overwritten.
- Test failed because `--yes` disabled prompts but *skipped* conflicted files (no overwrite).

## GREEN phase (implementation)

- Updated CLI dispatch in `src/main.rs`:
  - Treat `--yes` as “assume yes” for conflict resolution (`force || yes`).
  - Disable interactive prompting when `--json`, `--yes`, or stdin is not a TTY.
- Updated target selection helper `src/ui/menu.rs` so `--yes` also suppresses the platform selection prompt.

## REFACTOR phase (documentation sync)

- Updated docs to reflect the v0.2.0 command set and current CLI/config:
  - `docs/command-reference.md`
  - `docs/configuration.md`
  - `docs/cli-state-machine.md`
  - plus targeted fixes in related docs (changelog, platforms, pitfalls, decisions).

## Result

- `tests/cli_yes.rs` passes and `--yes` now behaves as documented.
- Documentation matches the v0.2.0 CLI surface more closely.

