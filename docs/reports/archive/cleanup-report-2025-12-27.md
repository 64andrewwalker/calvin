# Cleanup Report (2025-12-27)

## 📊 Statistics

- Files analyzed: 501 tracked files (`git ls-files`)
- Dead code files: 2 (soft-deprecated)
- Redundant scripts: 0 identified
- Potential savings: ~3 KB / 2 files (after one release cycle)

## 🗑️ Safe to Delete (High Confidence)

These were removed from the compilation graph and moved to `_deprecated/` (soft delete). After one release cycle, they can be permanently deleted.

| File | Reason | Last Modified | Verification |
|------|--------|---------------|--------------|
| `_deprecated/src/ui/render.rs` | Unused UI render abstraction (`TerminalState`) | 2025-12-18 | Only referenced from `src/ui/mod.rs` module list; no runtime usage |
| `_deprecated/src/ui/blocks/warning.rs` | Unused test-only component (`WarningBlock`) | 2025-12-24 | No references in `src/` or `tests/` |

## ⚠️ Review Required (Medium Confidence)

| File / Area | Reason | Concern |
|------------|--------|---------|
| `src/application/check.rs` (`CheckUseCase::config`) | Field is currently unused | Might be intended for future custom checks; remove only if roadmap confirms |
| `src/ui/json/events.rs` | Many event types are `#[allow(dead_code)]` | Used as a “future migration” surface; pruning could break downstream JSON tooling assumptions |

## 📦 Consolidation Opportunities

| Duplicates | Locations | Suggested Target |
|------------|-----------|------------------|
| Dangerous tool allow-list warnings | `src/infrastructure/adapters/claude_code.rs`, `src/infrastructure/adapters/codex.rs`, `src/infrastructure/adapters/cursor.rs`, `src/security/checks.rs` | Extract a shared helper (domain/policy or security module) and reference from adapters |
| Cursor-only command decision logic | `src/domain/services/compiler_service.rs` (static + instance variants) | Keep one implementation (static) and delegate instance method to it |

## 🧹 Recommended .gitignore Additions

None identified in this pass (common OS/editor/build artifacts are already covered).

## Execution Artifacts

- Deletion script (not executed): `scripts/cleanup/delete-deprecated-2025-12-27.sh`

