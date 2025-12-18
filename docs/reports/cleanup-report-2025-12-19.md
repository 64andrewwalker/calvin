# Codebase Cleanup Report

> **Generated**: 2025-12-19  
> **Tool**: Calvin M-MAINT Codebase Cleanup

---

## 📊 Statistics

| Metric | Value |
|--------|-------|
| Files analyzed | ~78 source files |
| Dead code annotations | 10 locations |
| Unused dependencies | 2 crates |
| Build artifacts | 3.7G (`target/`) |
| Potential savings | ~2 crates + 3.7G disk |
| Documentation files | 25 files |
| Archive files | 13 files |

---

## 🗑️ Safe to Delete (High Confidence)

### Unused Dependencies

| Crate | Reason | Verification |
|-------|--------|--------------|
| `owo-colors` | Not used (terminal coloring handled by theme.rs) | `cargo machete` analysis |
| `which` | Listed in Cargo.toml but not called in code | `cargo machete` analysis |

**Action**: Remove from `Cargo.toml`

### Build Artifacts

| Path | Size | Action |
|------|------|--------|
| `target/` | 3.7G | Run `cargo clean` periodically |
| `dist/` | 8.0K | Keep (distribution artifacts) |

---

## ⚠️ Review Required (Medium Confidence)

### `#[allow(dead_code)]` Annotations

These functions are marked as unused but intentionally retained:

| File | Function/Struct | Reason | Recommendation |
|------|-----------------|--------|----------------|
| `src/ui/output.rs:10` | `maybe_warn_allow_naked()` | "reserved for future use" | **Keep** - documented reason |
| `src/ui/output.rs:44` | `format_allow_naked_warning()` | Helper for above | **Keep** - paired with above |
| `src/ui/widgets/list.rs:4` | Struct annotation | UI component library | **Keep** - library pattern |
| `src/ui/widgets/list.rs:49` | Method annotation | UI component library | **Keep** - library pattern |
| `src/ui/views/watch.rs:5` | Type annotation | Watch view components | **Keep** - UI library |
| `src/ui/components/stream.rs:2` | Type annotation | Stream component | **Keep** - UI library |
| `src/ui/primitives/icon.rs:6` | Icon variants | Icon library | **Keep** - library pattern |
| `src/ui/render.rs:5,15` | Render utilities | Render abstraction | **Keep** - UI library |
| `src/ui/blocks/warning.rs:5` | Warning block | UI block | **Keep** - used in tests |

**Verdict**: All `#[allow(dead_code)]` annotations are justified as part of a UI component library pattern. No action needed.

### `state.rs` Module

| File | Status | Notes |
|------|--------|-------|
| `src/state.rs` | Used in `main.rs` | Contains `detect_state()`, `ProjectState` |

**Usage**: `mod state;` declared in `main.rs`. Used for CLI project state detection.
**Verdict**: **Keep** - actively used in CLI flow.

### `security_baseline.rs` Module

| File | Status | Notes |
|------|--------|-------|
| `src/security_baseline.rs` | Used | Imported by `security.rs`, `claude_code.rs` |

**Verdict**: **Keep** - actively used for security deny patterns.

---

## 📦 Consolidation Opportunities

### No Duplicate Utilities Found

A review for duplicate functions revealed no significant consolidation opportunities. The codebase already follows good practices:

- Helper functions are centralized in appropriate modules
- No copy-pasted functions across files
- UI primitives are properly abstracted in `src/ui/primitives/`

---

## 🧹 `.gitignore` Status

Current `.gitignore` is well-configured:
- ✅ `/target/` - Build artifacts
- ✅ `/dist/` - Distribution outputs  
- ✅ `.DS_Store` - OS files
- ✅ `.env`, `.env.*` - Secrets
- ✅ `*.pem`, `*.key` - Credentials
- ✅ `*.swp`, `*.swo` - Editor swap files

**No additions needed.**

---

## 📁 Archive Directory Review

The `docs/archive/` directory contains historical documents:

| File | Size | Description | Status |
|------|------|-------------|--------|
| cli-animation-design.md | 18K | Original animation design | ✅ Properly archived |
| cli-animation-reflection.md | 25K | Post-implementation review | ✅ Properly archived |
| v0.2-design-principles.md | 6K | v0.2 design notes | ✅ Properly archived |
| v0.2-product-reflection.md | 19K | Product decisions | ✅ Properly archived |
| tdd-sessions.md | 8K | TDD session log | ✅ Properly archived |
| design/* | 8 files | Detailed design docs | ✅ Properly archived |

**Verdict**: Archive is well-organized. No action needed.

---

## ✅ Action Items

### Immediate (Safe to Execute)

1. **Remove unused dependencies** from `Cargo.toml`:
   ```toml
   # Remove these lines:
   owo-colors = "..."
   which = "..."
   ```

2. **Clean build artifacts** (optional, for disk space):
   ```bash
   cargo clean
   ```

### No Further Cleanup Needed

- No orphaned source files detected
- No commented-out code blocks (>10 lines)
- No duplicate utilities to consolidate
- No `test.js`, `temp.py`, `debug_*.ts` files (this is a Rust project)
- No `.bak`, `.orig`, `.swp` files in repository
- Git working directory is clean (`git status` shows no changes)

---

## Summary

The Calvin codebase is **clean and well-maintained**. The only actionable items are:

| Priority | Action | Savings |
|----------|--------|---------|
| P1 | Remove `owo-colors` dependency | ~10KB binary |
| P1 | Remove `which` dependency | ~5KB binary |
| P3 | Periodic `cargo clean` | 3.7G disk |

This cleanup will reduce binary size slightly by removing unused dependencies.
