# TDD Session: Sprint 1 (US-1 + US-4)

> **Started**: 2025-12-17  
> **Status**: ✅ Complete (`cargo test` passing)

## Requirements Covered

From `docs/ux-review.md`:
- [x] US-1: `security.deny.exclude` can remove minimum deny patterns that would block specific files (e.g. `.env.example`)
- [x] US-1: `security.allow_naked = true` disables hardcoded deny injection and shows a prominent warning
- [x] US-1: `calvin doctor` reflects custom deny rules
- [x] US-4: `calvin sync --interactive` prompts on conflicts with `o/s/d/a/A`
- [x] US-4: `diff` shows a unified diff and re-prompts
- [x] US-4: `All` applies the choice to subsequent conflicts

From `docs/cli-state-machine.md`:
- [x] Document `--interactive` in sync option matrices/state notes

## Tests Written

- `src/config.rs`
  - `test_config_parse_security_deny_table_with_exclude`
  - `test_config_parse_security_deny_array_compat`
- `src/security_baseline.rs`
  - `test_effective_claude_deny_patterns_default_includes_minimum`
  - `test_effective_claude_deny_patterns_allow_naked_removes_minimum`
  - `test_effective_claude_deny_patterns_exclude_removes_matching_pattern`
- `src/adapters/claude_code.rs`
  - `test_claude_security_baseline_respects_allow_naked`
  - `test_claude_security_baseline_applies_deny_exclude`
- `src/security.rs`
  - `test_deny_list_completeness_requires_env_dot_star_by_default`
  - `test_deny_list_exclude_can_remove_minimum_patterns`
  - `test_allow_naked_disables_minimum_deny_requirements_in_doctor`
- `src/sync/mod.rs`
  - `test_sync_interactive_overwrite_modified_file`
  - `test_sync_interactive_skip_modified_file`
  - `test_sync_interactive_diff_then_overwrite`
  - `test_sync_interactive_overwrite_all_applies_to_multiple_conflicts`
  - `test_sync_interactive_abort_returns_error`
- `src/main.rs`
  - `test_cli_parse_sync_interactive`
  - `test_cli_parse_sync_interactive_short_flag`

## Implementation Decisions

1. **Config compatibility**
   - Introduced `security.deny` as a structured config (`patterns` + `exclude`) while keeping legacy `deny = [...]` support via an untagged serde deserializer.

2. **Single source of truth for minimum deny**
   - Centralized the hardcoded deny list and the “effective deny patterns” calculation in `src/security_baseline.rs` so adapters and `doctor/audit` share identical logic.

3. **Exclude semantics**
   - Implemented `security.deny.exclude` as “exclude specific paths” by removing any deny pattern that would match an excluded path (e.g. `.env.*` matches `.env.example` → drop `.env.*`).

4. **Doctor reflects config**
   - `run_doctor()` now reads `.promptpack/config.toml` from the project root and validates `.claude/settings.json` against the effective deny patterns derived from config.

5. **Interactive sync design**
   - Added `SyncPrompter` abstraction and a default `StdioPrompter` implementation.
   - Implemented unified diff output via `similar::TextDiff`.
   - Abort is represented as `CalvinError::SyncAborted` and stops the sync loop (lockfile still persists prior writes).

## Refactoring Notes

- Extracted conflict-handling logic into `sync_with_fs_with_prompter()` to keep the core sync path testable.
- Added `maybe_warn_allow_naked()` in `src/main.rs` to keep the CLI warning consistent across `sync` and `install`.

