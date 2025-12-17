# TDD Session: v0.2.0 Phase 4 (sync module refactor)

## Goals

- Split `src/sync/mod.rs` (was ~900 LOC) into smaller modules.
- Keep behavior identical and keep all tests green throughout.

## RED phase (tests added)

- Added “module boundary” tests in sync tests:
  - `super::compile::compile_assets(...)` compiles and returns outputs.
  - `super::conflict::unified_diff(...)` is available and includes expected headers.

These tests intentionally failed before the modules existed.

## GREEN phase (implementation)

- Extracted compilation logic to `src/sync/compile.rs`:
  - Moved `compile_assets()` including Cursor “commands when Claude isn’t selected” behavior.
  - Re-exported `compile_assets` from `src/sync/mod.rs` to preserve public API (`calvin::sync::compile_assets`).
- Extracted interactive-conflict helpers to `src/sync/conflict.rs`:
  - `ConflictReason`, `ConflictChoice`, `SyncPrompter`, `StdioPrompter`, `unified_diff()`.
- Reduced `src/sync/mod.rs` size by moving unit tests into `src/sync/tests.rs` and keeping `#[cfg(test)] mod tests;` in `src/sync/mod.rs`.

## Result

- `src/sync/mod.rs` is now < 400 LOC (currently ~300 LOC).
- `cargo test` and `cargo clippy` both pass.

