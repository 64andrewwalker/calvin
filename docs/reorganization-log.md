# Documentation Reorganization Log

> **Started**: 2025-12-30

## 1. Inventory Summary

### Root Level (8 files, 2047 lines total)
| File | Lines | Status |
|------|-------|--------|
| `README.md` | 246 | ✅ Keep (UPPERCASE convention) |
| `README_zh-CN.md` | 246 | ✅ Keep (localized README) |
| `CHANGELOG.md` | 299 | ✅ Keep (UPPERCASE convention) |
| `TODO.md` | 444 | ⚠️ Review - should this be in docs/? |
| `spec.md` | 125 | ✅ Keep |
| `AGENTS.md` | 229 | ⚠️ Near-duplicate of CLAUDE.md |
| `CLAUDE.md` | 229 | ⚠️ Near-duplicate of AGENTS.md |
| `GEMINI.md` | 229 | ⚠️ Different content from CLAUDE.md |

### docs/ Directory Structure
| Subdirectory | Files | Purpose |
|--------------|-------|---------|
| `architecture/` | 13 | Architecture documentation |
| `archive/` | 68 | Historical/deprecated docs |
| `guides/` | 2 | How-to guides |
| `proposals/` | 3 | Feature PRDs |
| `reports/` | 13 | Audit reports (10 archived) |
| `testing/` | 5 | Testing documentation |

### Short Docs (<100 lines) - Candidates for Consolidation
| File | Lines | Recommendation |
|------|-------|----------------|
| `architecture/overview.md` | 37 | Merge into `architecture.md` |
| `architecture/README.md` | 39 | Merge into `architecture.md` |
| `reports/cleanup-report-2025-12-27.md` | 40 | Archive |
| `architecture/migration.md` | 64 | Keep (active migration guide) |
| `test-variants-rationale.md` | 77 | Merge into `testing/testing-philosophy.md` |
| `sync-audit-report.md` | 83 | Archive to `reports/archive/` |
| `skills.md` | 96 | Keep (user-facing docs) |

### Potential Duplicates
| Files | Action |
|-------|--------|
| `architecture.md` (253 lines) vs `architecture/overview.md` (37) | Merge overview into architecture.md |
| `architecture-redesign.md` (318) | Check if outdated, consider archiving |
| `implementation-plan.md` (328) vs `testing/implementation-plan.md` (599) | Different content, rename for clarity |

### Naming Convention Violations
| File | Issue | Action |
|------|-------|--------|
| `docs/architecture/README.md` | UPPERCASE inside docs/ | Rename to `index.md` |
| `docs/architecture/TODO.md` | UPPERCASE inside docs/ | Rename to `todo.md` or merge |

## 2. Proposed Structure

```text
docs/
├── getting-started.md          # NEW: Combine quickstart content
├── architecture.md             # Existing + merged overview
├── command-reference.md        # Existing
├── configuration.md            # Existing
├── skills.md                   # Existing
├── target-platforms.md         # Existing
├── json-output.md              # Existing
├── api/
│   └── frontmatter.md          # Move from architecture/?
├── architecture/
│   ├── index.md                # Rename from README.md
│   ├── layers.md
│   ├── directory.md
│   ├── ports.md
│   ├── platform.md
│   └── decisions/              # NEW: Move tech-decisions.md here
│       └── tech-decisions.md
├── guides/
│   └── ... (existing)
├── proposals/
│   └── ... (existing PRDs)
├── testing/
│   ├── philosophy.md           # Rename from testing-philosophy.md
│   ├── contract-registry.md
│   └── ...
├── archive/
│   └── ... (old content)
└── reports/
    └── archive/                # Completed reports
```

## 3. Action Plan

### Phase 1: Quick Wins
- [ ] Rename `architecture/README.md` → `architecture/index.md`
- [ ] Rename `architecture/TODO.md` → `architecture/todo.md`
- [ ] Archive `sync-audit-report.md` to `reports/archive/`
- [ ] Archive `reports/cleanup-report-2025-12-27.md` to `reports/archive/`

### Phase 2: Consolidation
- [ ] Merge `architecture/overview.md` content into `architecture.md`
- [ ] Merge `test-variants-rationale.md` into `testing/testing-philosophy.md`
- [ ] Review `architecture-redesign.md` for archival

### Phase 3: Restructure
- [ ] Create `docs/api/` directory
- [ ] Move API-related docs
- [ ] Update all internal references

### Phase 4: Index Updates
- [ ] Update root `README.md` documentation section
- [ ] Create/update `docs/index.md` if needed

## 4. Change Log

| Date | Change | Files Affected |
|------|--------|----------------|
| 2025-01-05 | **Archived completed plan docs** | archive/ |
| 2025-01-05 | Moved implementation-plan.md → archive/implementation-plan-v1.md | archive/ |
| 2025-01-05 | Moved architecture/todo.md → archive/architecture-todo-completed-2025-12.md | archive/ |
| 2025-01-05 | Moved architecture-redesign.md → archive/architecture-redesign-skills.md | archive/ |
| 2025-01-05 | Moved analysis-report.md → archive/analysis-report-v0.2.md | archive/ |
| 2025-01-05 | Updated version refs 0.5.1 → 0.6.0 | command-reference.md, api/json-output.md |
| 2025-01-05 | Created docs/index.md navigation page | index.md |
| 2025-01-05 | Improved configuration.md formatting | configuration.md |
| 2025-01-05 | Updated README roadmap links | README.md, README_zh-CN.md |
| 2025-12-30 | Created reorganization plan | This file |
| 2025-12-30 | Phase 1: Renamed architecture/README.md → index.md | architecture/index.md |
| 2025-12-30 | Phase 1: Renamed architecture/TODO.md → todo.md | architecture/todo.md |
| 2025-12-30 | Phase 1: Archived sync-audit-report.md | reports/archive/ |
| 2025-12-30 | Phase 1: Archived cleanup-report-2025-12-27.md | reports/archive/ |
| 2025-12-30 | Phase 2: Archived architecture/overview.md (duplicate) | archive/ |
| 2025-12-30 | Updated references in README, AGENTS, CLAUDE, GEMINI | Root files |
| 2025-12-30 | Phase 3: Created docs/api/ directory | api/ |
| 2025-12-30 | Phase 3: Moved test-variants-rationale.md → testing/ | testing/ |
| 2025-12-30 | Phase 3: Moved api-review.md → api/ | api/ |
| 2025-12-30 | Phase 3: Moved json-output.md → api/ | api/ |
| 2025-12-30 | Updated references in TODO.md, command-reference.md | docs/ |
