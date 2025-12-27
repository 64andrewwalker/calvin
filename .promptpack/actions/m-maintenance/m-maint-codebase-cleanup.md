---
description: Codebase Cleanup
---

# [M-MAINT] Codebase Cleanup & Declutter

> **Stage**: Maintenance | **Category**: Cleanup | **Frequency**: Periodic

Remove dead code, redundant files, and accumulated cruft. Track in `TODO.md`.

## Detection Targets

### Dead Code

1. **Unreachable Code**:
   - Functions never called (check import graph)
   - Dead branches (always true/false conditions)
   - Commented-out code blocks (>10 lines)
2. **Orphaned Files**:
   - Source files not in any import chain
   - Tests for deleted features
   - Config files for removed tools

### Redundant Scripts

1. **One-Time Scripts**:
   - Migration scripts already applied
   - Data fix scripts (check git history for age)
   - Setup scripts superseded by automation
2. **Duplicate Utilities**:
   - Multiple implementations of same helper
   - Copy-pasted functions across files
   - Wrapper functions that just pass through

### Cruft Patterns

1. **Development Artifacts**:
   - `test.js`, `temp.py`, `debug_*.ts`
   - `old_`, `backup_`, `copy_of_` prefixed files
   - `.bak`, `.orig`, `.swp` files
2. **Build Leftovers**:
   - Committed build outputs
   - Cache directories (`.cache`, `__pycache__`)
   - Lock file conflicts
3. **Abandoned Experiments**:
   - Feature branches merged but code unused
   - A/B test variants never cleaned up
   - POC code that became production

## Analysis Tools

```bash
# Unreachable code detection
- knip (JS/TS)
- vulture (Python)
- unused (Go)
- dead_code (Rust)
# Duplicate detection
- jscpd (polyglot)
- PMD CPD (Java)
# Large/binary file detection
- git ls-files | xargs du -h | sort -rh
```

## Safety Protocol

1. **Before Deletion**:
   - Verify no dynamic imports/requires
   - Check reflection/metaprogramming usage
   - Search for string-based references
   - Review git blame for context
2. **Soft Delete First**:
   - Move to `_deprecated/` directory
   - Wait one release cycle
   - Then permanently remove

## Output

```text
## Cleanup Report
### 📊 Statistics
- Files analyzed: X
- Dead code files: Y
- Redundant scripts: Z
- Potential savings: X MB / Y files
### 🗑️ Safe to Delete (High Confidence)
| File | Reason | Last Modified | Verification |
|------|--------|---------------|--------------|
| scripts/migrate_v1.py | One-time migration, applied 2023-01 | 2023-01-15 | No imports |
### ⚠️ Review Required (Medium Confidence)
| File | Reason | Concern |
|------|--------|---------|
| utils/old_parser.ts | No static imports | Check dynamic require() |
### 📦 Consolidation Opportunities
| Duplicates | Locations | Suggested Target |
|------------|-----------|------------------|
| formatDate() | utils.ts, helpers.ts, lib.ts | utils/date.ts |
### 🧹 Recommended .gitignore Additions
```

## Execution

1. Generate deletion script (not auto-execute)
2. Create consolidation PR separately
3. Update import statements
4. Run full test suite
5. Monitor error rates post-deploy
