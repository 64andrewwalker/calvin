# Open Items Summary

> **Date**: 2025-12-24  
> **Updated**: 2025-12-24 (config-silent-failures verified as implemented)  
> **Purpose**: Consolidated list of remaining open items from docs/proposals and docs/reports audits

---

## Summary

After reviewing all documents in `docs/proposals/` and `docs/reports/`, the following status was determined:

| Category | Total | Archived (Implemented) | Remaining |
|----------|-------|----------------------|-----------|
| Proposals | 7 | 7 | 0 |
| Reports | 11 | 9 | 2 |

**All proposals have been fully implemented and archived.**

---

## Archived Documents (Fully Implemented)

### Proposals → `docs/proposals/archive/`

| Document | Status | Evidence |
|----------|--------|----------|
| `clean-command-implementation-plan.md` | ✅ Complete | `src/application/clean/` exists |
| `clean-command-prd.md` | ✅ Complete | Phase 1 & 2 implemented |
| `clean-command-prd-checklist.md` | ✅ Complete | All checklist items verified |
| `clean-command-todo.md` | ✅ Complete | Phase 1 & 2 marked done |
| `selection-icons-refactor.md` | ✅ Complete | `CalvinTheme` in `src/ui/theme.rs` |
| `tree-menu-refactor-plan.md` | ✅ Complete | `src/ui/widgets/tree_menu/` exists |
| `tree-menu-refactor-todo.md` | ✅ Complete | All phases marked complete |

### Reports → `docs/reports/archive/`

| Document | Status | Notes |
|----------|--------|-------|
| `api-review-clean-command.md` | ✅ Complete | All improvements implemented |
| `sync-audit-2025-12-23.md` | ✅ Fixed | ~95% aligned |
| `sync-audit-report.md` | ✅ Pass | No action needed |
| `api-review-2025-12-19.md` | ✅ Pass | Minor P3 items remain (acceptable) |
| `cleanup-report-2025-12-20.md` | ✅ Clean | No cleanup needed |
| `dependency-report-2025-12-20.md` | ✅ Healthy | Deps up to date |
| `performance-report-2025-12-19.md` | ✅ Excellent | No optimization needed |
| `cleanup-report-2025-12-19.md` | ✅ Complete | `owo-colors` and `which` removed |
| `config-silent-failures-audit-2025-12-24.md` | ✅ Implemented | `EnvVarValidator` provides warnings |

---

## Remaining Reports (With Open Items)

The following reports remain in `docs/reports/` with some pending items:

### 1. `product-issues-2025-12-21.md`

**Status**: Mixed - Some items implemented, some remain open

| Issue | Status | Notes |
|-------|--------|-------|
| P0: `calvin init` command | ✅ IMPLEMENTED | `Commands::Init` exists in `cli.rs` |
| P0: Installation docs | ⚠️ Verify | Check if Homebrew/Scoop publish is complete |
| P1: Error message UX | ❌ Open | Error messages could include fix suggestions |
| P2: MCP support completeness | ⚠️ Verify | Needs functional testing |
| P2: `--home` vs `scope: user` docs | ❌ Open | Documentation clarity needed |

**Actions Needed**:
1. Verify package manager publishing status (Homebrew, Scoop, crates.io)
2. Consider improving error messages with fix suggestions
3. Document the distinction between `--home` flag and `scope: user`

---

### 2. `security-audit-report.md`

**Status**: PASS with 1 advisory to monitor

| Issue | Severity | Status |
|-------|----------|--------|
| SEC-009: `serde_yml` / `libyml` advisory | ⚠️ Medium | Monitor for fixes |
| All other findings | ✅ | Mitigated or acceptable |

**Recommendation**: 
- Monitor `serde_yml` for RUSTSEC-2025-0067 and RUSTSEC-2025-0068 fixes
- Consider migrating to `serde_yaml` if advisory is not resolved

---

## Action Items Summary

| Priority | Action | Source | Status |
|----------|--------|--------|--------|
| P2 | Verify package manager publishing | product-issues | Open |
| P2 | Monitor `serde_yml` security advisory | security-audit-report | Open |
| ~~P3~~ | ~~Improve error messages with suggestions~~ | ~~product-issues~~ | ✅ Implemented |
| P3 | Document `--home` vs `scope: user` clearly | product-issues | Open |

---

## Recently Implemented (2025-12-24)

### Error Message Improvements

Added helpful suggestions to all major error types in `src/error.rs`:

| Error Type | Improvement |
|------------|-------------|
| `MissingField` | Shows fix suggestion + docs link |
| `NoFrontmatter` | Shows example frontmatter format |
| `UnclosedFrontmatter` | Suggests adding closing `---` |
| `DirectoryNotFound` | Suggests `mkdir -p` command |
| `InvalidAssetKind` | **Typo suggestions** using Levenshtein distance |
| `PathEscape` | Security warning about path traversal |

Example:
```
invalid asset kind 'polcy' in test.md
  → Did you mean 'policy'?
  → Valid kinds: policy, action, agent
  → Docs: https://calvin.dev/docs/frontmatter#kind
```

**Files changed**: `src/error.rs` (7 new tests added)

---

## Conclusion

**Overall Health**: ✅ Excellent

- All 7 proposals have been fully implemented and archived
- 9 of 11 reports have been addressed and archived
- Remaining 2 reports have minor open items (P2/P3 priority)
- No critical blockers identified

The codebase is in excellent shape. The remaining items are quality-of-life improvements that can be addressed in future iterations.

