# Refactoring Plan

> **Stage**: Development | **Category**: Code Quality | **Created**: 2025-12-18
>
> Follow `docs/REFACTOR_SCOPE_POLICY.md`: preserve behavior, keep changes atomic (one refactor per commit), and run tests after each change.

## Pre-conditions

- Refactoring plan existed: **No** → this document created.
- Tests exist: **Yes** (`cargo test` passes).

## Baseline Signals (current)

- **Tests**: `cargo test` ✅
- **Clippy**: `cargo clippy --all-targets --all-features` ⚠️ (4 warnings)
  - `clippy::derivable_impls` in `src/commands/deploy/targets.rs` (manual `Default`)
  - `clippy::cmp_owned` in `src/commands/deploy/targets.rs` tests (PathBuf comparisons)
- **Rustfmt**: `cargo fmt --check` ❌ (formatting drift across multiple files)
- **Large modules (lines)**:
  - `src/sync/engine.rs` ~1391
  - `src/security.rs` ~1029
  - `src/config.rs` ~788
  - `src/watcher.rs` ~619

## Prioritized Refactoring Tasks

### P0 — Low-risk quick wins (small diffs)

1. **Fix clippy warnings in deploy target types**
   - Scope: `src/commands/deploy/targets.rs`
   - Changes: derive `Default` for `ScopePolicy`; adjust tests to avoid `PathBuf::from(...)` comparisons.
   - Verify: `cargo test`, `cargo clippy --all-targets --all-features`.

2. **Remove trivial duplication in parser ID derivation**
   - Scope: `src/parser.rs`
   - Changes: use `derive_id()` inside `parse_file()` (no behavior change), or remove/rename `derive_id()` if truly redundant.
   - Verify: `cargo test`.

3. **Deduplicate doctor sink “add_*” construction**
   - Scope: `src/security.rs`
   - Changes: centralize `SecurityCheck` construction so `DoctorReport` and the callback sink don’t duplicate `add_pass/add_warning/add_error` logic.
   - Verify: `cargo test`.

### P1 — Medium refactors (shared behavior paths, reduce duplication)

4. **Consolidate conflict-resolution logic**
   - Scope: `src/sync/engine.rs`, `src/sync/plan.rs`, `src/commands/deploy/runner.rs`
   - Goal: remove the double implementation of interactive conflict resolution (`SyncEngine::resolve_conflicts_with_resolver` vs `resolve_conflicts_interactive`).
   - Likely approach: make deploy code path use `SyncEngine` (or the same conflict-resolver abstraction) so conflict handling is single-sourced.
   - Verify: `cargo test` (includes CLI + golden tests).

5. **Split platform security checks into modules**
   - Scope: `src/security.rs` → `src/security/*` (module split)
   - Goal: reduce file size and make per-platform checks easier to maintain (`claude_code`, `cursor`, `vscode`, `antigravity`, `codex`).
   - Verify: `cargo test`, then `cargo test --doc` (if/when doc tests are added).

6. **Tighten watch-mode incremental parsing**
   - Scope: `src/watcher.rs`
   - Targets:
     - Avoid reading file content twice when hashing + parsing.
     - Reduce repeated `canonicalize()` calls when possible.
     - Centralize “README.md skip” + “*.md only” decisions (currently duplicated between full and incremental paths).
   - Verify: `cargo test` (watcher tests already exist).

### P2 — Mechanical but high-churn (do last)

7. **Apply `cargo fmt` across the workspace**
   - Scope: multiple files (large diff)
   - Goal: establish rustfmt-consistent formatting; optionally add a CI check later.
   - Verify: `cargo fmt --check`, `cargo test`.

## Suggested Execution Order (one refactor per commit)

1. P0.1 Fix clippy warnings (deploy targets)
2. P0.2 Parser ID derivation cleanup
3. P0.3 Doctor sink dedup
4. P1.4 Conflict-resolution consolidation
5. P1.5 Security module split
6. P1.6 Watch incremental parsing improvements
7. P2.7 Workspace rustfmt pass

## Open Questions / Approval Needed

- Should `cargo fmt` be enforced in CI (and if so, where)?
- OK to remove or integrate currently-unwired code in `src/commands/deploy_v2/` as part of cleanup?
- Any areas you want explicitly excluded from refactoring (e.g., adapters, sync, UI)?

