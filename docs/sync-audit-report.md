# Documentation-Code Synchronization Audit Report

> **Date**: 2025-12-27  
> **Auditor**: AI Assistant  
> **Scope**: User-facing docs + core architecture docs vs current implementation

---

## Sync Status: ~95% Aligned (User-Facing Docs)

After applying fixes in this audit, the primary user-facing docs (README, target-platforms, tech-decisions, root TODO) match the current implementation well. Remaining drift is mostly limited to **historical design/review docs** that reference older milestone versions (e.g., `v0.3.0`).

---

## ✅ Issues Fixed This Audit (2025-12-27)

| Issue | Location | Fix Applied |
|---|---|---|
| README version drift (`v0.3.0` vs crate `0.6.0`) | `README.md:215`, `README_zh-CN.md:215`, `Cargo.toml:3` | Updated README + Chinese README to `v0.6.0` |
| Skills support not surfaced in README | `README.md:42`, `README.md:74`, `README_zh-CN.md:42`, `README_zh-CN.md:74` | Added Skills to the `.promptpack/` example + Features list |
| MCP docs overstated generation support | `README.md:47`, `README_zh-CN.md:47`, `docs/target-platforms.md:88-94` | Clarified MCP as “planned/validated” (not generated) |
| Lockfile strategy docs out-of-date | `docs/tech-decisions.md:408-429` | Updated to `calvin.lock` (project) / `~/.calvin/calvin.lock` (home) |
| Root TODO status/test counts drift | `TODO.md:3-7` | Updated top-level status + test inventory snapshot and marked CI/cross-compile as complete |

---

## 🔴 Remaining Critical Mismatches

None found in primary user-facing docs after the fixes above.

---

## 🟡 Partial Implementations

| Feature | Status | Notes |
|---|---|---|
| MCP support | **PARTIAL** | `calvin check` validates Cursor `.cursor/mcp.json` against an allowlist, but Calvin does not generate MCP configs yet. |

---

## 🟢 Verified Complete (Spot Checks)

| Feature | Evidence |
|---|---|
| Cross-platform CI | `.github/workflows/ci.yml` |
| Coverage gating + badge update | `.github/workflows/ci.yml`, README coverage badge endpoint |
| Skills support (directory-based `SKILL.md`) | `docs/skills.md`, `docs/proposals/skills-support-prd.md`, tests under `tests/cli_skills.rs` |
| Lockfile migration to project root | `src/application/lockfile_migration.rs` |

---

## 🔧 Code Reality Scan (2025-12-27)

- `TODO/FIXME/HACK/XXX`: 1 actionable TODO found in tests (`tests/contracts/layers.rs:265`). Production `TODO` strings are part of VS Code adapter output validation (`src/infrastructure/adapters/vscode.rs:92`).
- `todo!()` / `unimplemented!()`: none found under `src/` and `tests/` in this audit.
- Stub/mock code: mocks exist in tests and port-contract examples; no production placeholder implementations were found.

---

## 📝 Documentation Updates Applied (This Audit)

- Updated README + Chinese README to reflect `v0.6.0`, include Skills, and clarify MCP as “planned/validated (not generated)”.
- Updated `docs/target-platforms.md` to match `calvin version` output and clarify MCP as validation-only.
- Updated `docs/tech-decisions.md` lockfile strategy to match the current `calvin.lock` (project) and `~/.calvin/calvin.lock` (home) behavior.
- Updated root `TODO.md` high-level status + CI/cross-compile checkboxes to match repository reality.

---

## Prevention (Recommended)

1. Add a lightweight “docs drift” check in CI (e.g., assert `README` version matches `Cargo.toml`).
2. Prefer “at least” phrasing for volatile counts (tests/coverage) unless generated automatically.

---

## Notes (Historical Docs)

The following files still reference version milestones like `v0.3.0`, but appear to be **historical planning/review artifacts** rather than current-status claims:

- `docs/ui-components-spec.md`
- `docs/ux-review.md`
- `docs/architecture/TODO.md`
- `docs/architecture/review-senior.md`
