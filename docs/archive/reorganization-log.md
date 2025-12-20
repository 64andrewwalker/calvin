# Documentation Reorganization Log

> **Date**: 2025-12-18
> **Status**: ✅ Complete
> **Workflow**: 6-REL: Documentation Organization

---

## 1. Inventory

### Root-level Documentation (3 files)
| File | Size | Purpose |
|------|------|---------|
| `README.md` | 4.8KB | Project overview, installation, quick start |
| `TODO.md` | 9.7KB | Implementation checklist (all phases complete) |
| `spec.md` | 19.9KB | Original project specification (Chinese) |

### docs/ Directory (27 files + 4 subdirectories)

#### Core Documentation
| File | Size | Purpose | Status |
|------|------|---------|--------|
| `architecture.md` | 14.8KB | System design overview | ✅ Keep |
| `command-reference.md` | 3.8KB | CLI command documentation | ✅ Keep |
| `configuration.md` | 5.5KB | Config file, env vars, CLI args | ✅ Keep |
| `target-platforms.md` | 6.1KB | IDE versions & output formats | ✅ Keep |

#### API & Policy Documentation
| File | Size | Purpose | Status |
|------|------|---------|--------|
| `api-changelog.md` | 2.6KB | Format/adapter version history | ✅ Keep |
| `api-versioning-policy.md` | 3.9KB | Versioning strategy | ✅ Keep |
| `pitfall-mitigations.md` | 11.5KB | Known issues and mitigations | ✅ Keep |
| `tech-decisions.md` | 14.6KB | Technology choices rationale | ✅ Keep |

#### Audit & Review Reports
| File | Size | Purpose | Status |
|------|------|---------|--------|
| `security-audit-report.md` | 7.6KB | Security audit findings | ✅ Keep |
| `sync-audit-report.md` | 3.8KB | Sync engine audit | ✅ Keep |
| `ux-review.md` | 24.6KB | UX review with user stories | ✅ Keep |
| `review-3f9b0c8.md` | 13.7KB | Specific commit review | ⚠️ Archive candidate |

#### Implementation Documentation
| File | Size | Purpose | Status |
|------|------|---------|--------|
| `implementation-plan.md` | 9KB | Phased development roadmap | ✅ Keep |
| `cli-state-machine.md` | 52.5KB | CLI state documentation | ✅ Keep |
| `REFACTOR_SCOPE_POLICY.md` | 11.3KB | Refactoring scope rules | ⚠️ Rename to lowercase |
| `test-variants-rationale.md` | 3.7KB | Test variant explanations (Chinese) | ⚠️ Translate |

#### TDD Session Logs (10 files - Consolidation Candidates)
| File | Size | Lines | Status |
|------|------|-------|--------|
| `tdd-session-phase1-parser.md` | 2.2KB | 89 | 🔄 Consolidate |
| `tdd-session-sprint1-us1-us4.md` | 3.3KB | 69 | 🔄 Consolidate |
| `tdd-session-sprint2-us2-us9.md` | 2.3KB | 56 | 🔄 Consolidate |
| `tdd-session-sync-refactor.md` | 5.2KB | 143 | 🔄 Consolidate |
| `tdd-session-cli-animation.md` | 12.3KB | 220 | ✅ Keep (substantial) |
| `tdd-session-v0.2.0-phase0.md` | 2.5KB | 61 | 🔄 Consolidate |
| `tdd-session-v0.2.0-phase1.md` | 1.8KB | 48 | 🔄 Consolidate |
| `tdd-session-v0.2.0-phase3.md` | 1.4KB | 37 | 🔄 Consolidate |
| `tdd-session-v0.2.0-phase4.md` | 1.2KB | 30 | 🔄 Consolidate |
| `tdd-session-v0.2.0-phase5.md` | 1.4KB | 36 | 🔄 Consolidate |
| `tdd-session-v0.2.0-polish.md` | 1.4KB | 35 | 🔄 Consolidate |

### Subdirectories

#### docs/design/ (8 files)
| File | Size | Purpose | Status |
|------|------|---------|--------|
| `deploy-refactor-plan.md` | 4.5KB | Deploy command refactor plan | 🔄 Archive |
| `deploy-target-design.md` | 4.7KB | Deploy target system design | 🔄 Archive |
| `sync-engine-consolidation.md` | 6.5KB | SyncEngine consolidation (Chinese) | 🔄 Archive |
| `sync-engine-consolidation-todo.md` | 4.7KB | Related TODO | 🔄 Archive |
| `sync-engine-refactor.md` | 9.2KB | Sync engine refactor design | 🔄 Archive |
| `sync-unification-proposal.md` | 11.1KB | Sync unification (Chinese) | 🔄 Archive |
| `sync-unification-detailed.md` | 15.4KB | Detailed sync design (Chinese) | 🔄 Archive |
| `sync-unification-todo.md` | 5KB | Related TODO | 🔄 Archive |

**Recommendation**: These are historical design documents for completed work. Move to `docs/archive/design/`.

#### docs/calvin/ (3 files)
| File | Size | Purpose | Status |
|------|------|---------|--------|
| `TODO.md` | 8.5KB | v0.2.0 refactor TODO (Chinese) | ⚠️ Duplicate of root |
| `design-principles.md` | 6KB | Design principles | ✅ Move to parent |
| `product-reflection.md` | 19.3KB | Product design reflection | ✅ Move to parent |

#### docs/cli-animation/ (4 files)
| File | Size | Purpose | Status |
|------|------|---------|--------|
| `TODO.md` | 14.6KB | CLI animation TODO | ⚠️ Historical |
| `design-principles.md` | 18.1KB | Animation design principles | ✅ Consolidate |
| `product-reflection.md` | 25.3KB | Animation design reflection | ✅ Consolidate |
| `ui-components-spec.md` | 22.3KB | UI component specifications | ✅ Move to parent |

#### docs/calvin-v0.3/ (EMPTY)
**Action**: Delete empty directory.

---

## 2. Audit Findings

### A. Naming Violations
1. ❌ `REFACTOR_SCOPE_POLICY.md` - Should be `refactor-scope-policy.md`

### B. Duplicate Content
1. ⚠️ `TODO.md` (root) vs `docs/calvin/TODO.md` - Both track project progress
2. ⚠️ Multiple `design-principles.md` files in subdirectories

### C. Outdated/Orphaned Content
1. 🗑️ `docs/calvin-v0.3/` - Empty directory
2. 📦 `docs/design/*.md` - Historical design docs for completed features

### D. Content Requiring Translation
1. `spec.md` - Chinese
2. `docs/test-variants-rationale.md` - Chinese
3. `docs/design/sync-*.md` - Mostly Chinese
4. `docs/calvin/TODO.md` - Chinese

### E. Small Files for Consolidation
9 TDD session files under 100 lines each should be consolidated into `docs/tdd-sessions.md`.

---

## 3. Reorganization Plan

### Phase 1: File Renames
```bash
# Fix naming convention violations
mv docs/REFACTOR_SCOPE_POLICY.md docs/refactor-scope-policy.md
```

### Phase 2: Remove Empty Directories
```bash
rmdir docs/calvin-v0.3
```

### Phase 3: Consolidate TDD Sessions
Create `docs/archive/tdd-sessions.md` by merging:
- tdd-session-phase1-parser.md
- tdd-session-sprint1-us1-us4.md
- tdd-session-sprint2-us2-us9.md
- tdd-session-v0.2.0-phase0.md
- tdd-session-v0.2.0-phase1.md
- tdd-session-v0.2.0-phase3.md
- tdd-session-v0.2.0-phase4.md
- tdd-session-v0.2.0-phase5.md
- tdd-session-v0.2.0-polish.md

Keep separate (substantial):
- tdd-session-sync-refactor.md (143 lines)
- tdd-session-cli-animation.md (220 lines)

### Phase 4: Archive Design Docs
```bash
mkdir -p docs/archive/design
mv docs/design/*.md docs/archive/design/
rmdir docs/design
```

### Phase 5: Reorganize Subdirectories
```bash
# Move valuable content from subdirectories
mv docs/calvin/design-principles.md docs/archive/v0.2-design-principles.md
mv docs/calvin/product-reflection.md docs/archive/v0.2-product-reflection.md
rm docs/calvin/TODO.md  # Duplicate, root TODO.md is canonical
rmdir docs/calvin

# CLI animation docs
mv docs/cli-animation/ui-components-spec.md docs/ui-components-spec.md
mv docs/cli-animation/design-principles.md docs/archive/cli-animation-design.md
mv docs/cli-animation/product-reflection.md docs/archive/cli-animation-reflection.md
rm docs/cli-animation/TODO.md  # Historical
rmdir docs/cli-animation
```

### Phase 6: Create Archive Directory Structure
```
docs/
├── archive/                    # Historical/completed work
│   ├── design/                 # Old design proposals
│   ├── tdd-sessions.md         # Consolidated TDD logs
│   ├── v0.2-design-principles.md
│   ├── v0.2-product-reflection.md
│   ├── cli-animation-design.md
│   └── cli-animation-reflection.md
├── api-changelog.md
├── api-versioning-policy.md
├── architecture.md
├── cli-state-machine.md
├── command-reference.md
├── configuration.md
├── implementation-plan.md
├── pitfall-mitigations.md
├── refactor-scope-policy.md    # Renamed from UPPERCASE
├── security-audit-report.md
├── sync-audit-report.md
├── target-platforms.md
├── tdd-session-cli-animation.md  # Kept (substantial)
├── tdd-session-sync-refactor.md  # Kept (substantial)
├── tech-decisions.md
├── test-variants-rationale.md
├── ui-components-spec.md       # Moved from cli-animation/
├── ux-review.md
└── review-3f9b0c8.md
```

---

## 4. Changes Log

| Action | Source | Destination | Status |
|--------|--------|-------------|--------|
| Rename | `REFACTOR_SCOPE_POLICY.md` | `refactor-scope-policy.md` | ✅ Done |
| Delete | `calvin-v0.3/` | - | ✅ Done |
| Create | - | `archive/` | ✅ Done |
| Move | `design/*.md` | `archive/design/` | ✅ Done |
| Consolidate | 9 TDD files | `archive/tdd-sessions.md` | ✅ Done |
| Move | `calvin/design-principles.md` | `archive/` | ✅ Done |
| Move | `calvin/product-reflection.md` | `archive/` | ✅ Done |
| Delete | `calvin/TODO.md` | - | ✅ Done |
| Move | `cli-animation/ui-components-spec.md` | `docs/` | ✅ Done |
| Move | `cli-animation/*.md` | `archive/` | ✅ Done |

---

## 5. README.md Index Update

After reorganization, update `README.md` documentation section:

```markdown
## Documentation

- **[Architecture](docs/architecture.md)**: System design and component overview
- **[Command Reference](docs/command-reference.md)**: CLI commands and options
- **[Configuration](docs/configuration.md)**: Config file, environment variables
- **[Target Platforms](docs/target-platforms.md)**: Supported IDEs and formats
- **[Tech Decisions](docs/tech-decisions.md)**: Technology choices and rationale
- **[Implementation Plan](docs/implementation-plan.md)**: Development roadmap
```

---

## 6. Notes

- Chinese documentation should be translated in a separate pass
- `spec.md` at root level is the original specification - consider archiving
- `review-3f9b0c8.md` is a specific commit review - consider archiving after confirming no active references
