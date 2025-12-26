# Windows Home Directory Unification Plan

> **Created**: 2025-12-26
> **Status**: ✅ Completed
> **Goal**: Eliminate Windows CI failures caused by `dirs::home_dir()` using system API instead of environment variables

## Problem Statement

On Windows, `dirs::home_dir()` uses the Windows system API (`SHGetKnownFolderPath`) rather than environment variables like `HOME` or `USERPROFILE`. This means setting these environment variables in tests has no effect on Windows, causing test isolation to fail.

Current workaround: Adding individual environment variable overrides for each path that depends on home directory. This leads to:
- Variable proliferation (already 4+ variables)
- Easy to miss new callsites
- Inconsistent override mechanisms
- Maintenance burden in `WindowsCompatExt`

## Solution: Unified `calvin_home_dir()` Function

Create a single `calvin_home_dir()` function that:
1. First checks `CALVIN_TEST_HOME` environment variable
2. Falls back to `dirs::home_dir()` if not set

All Calvin-internal code that needs the home directory for functional paths (lockfile, registry, user layer, config) will use this function.

**Exceptions** (continue using `dirs::home_dir()` directly):
- Display functions (`display_with_tilde`) - cosmetic only
- Security checks - need to check real user directories
- Tilde expansion in user-provided paths - external paths should use real home

## Implementation Checklist

### Phase 1: Create Infrastructure

- [x] **1.1** Create `src/infrastructure/fs/home.rs` with `calvin_home_dir()` function
- [x] **1.2** Export from `src/infrastructure/fs/mod.rs`
- [x] **1.3** Export from `src/infrastructure/mod.rs`
- [x] **1.4** Add unit tests for the new function

### Phase 2: Migrate Callsites

Replace `dirs::home_dir()` with `calvin_home_dir()` in:

- [x] **2.1** `src/application/lockfile_migration.rs:19` - `global_lockfile_path()`
- [x] **2.2** `src/config/types.rs:207` - `default_user_layer_path()`
- [x] **2.3** `src/config/loader.rs:98` - legacy user config path
- [x] **2.4** `src/infrastructure/repositories/registry.rs:158` - registry path fallback
- [x] **2.5** `src/application/deploy/use_case.rs:1114` - private `default_user_layer_path()`
- [x] **2.6** `src/commands/debug.rs:292` - compare root for user scope
- [x] **2.7** `src/infrastructure/sync/local.rs` - `LocalHomeDestination::expand_home()`
- [x] **2.8** `src/infrastructure/fs/local.rs` - public `expand_home()` function
- [x] **2.9** `src/domain/services/layer_resolver.rs` - `expand_home()` (direct env var check, domain can't depend on infrastructure)

### Phase 3: Update Test Infrastructure

- [x] **3.1** Add `CALVIN_TEST_HOME` as the primary variable in `WindowsCompatExt`
- [x] **3.2** Keep existing specific overrides for backwards compatibility
- [x] **3.3** No `TestEnv` changes needed (already uses `with_test_home`)

### Phase 4: Verification

- [x] **4.1** Run `cargo test` locally (macOS/Linux) - all 263+ tests passing
- [ ] **4.2** Verify CI passes on Windows (pending push)
- [ ] **4.3** Verify CI passes on all platforms (pending push)

## Files to Modify

| File | Change |
|------|--------|
| `src/infrastructure/fs/home.rs` | **NEW** - Create `calvin_home_dir()` |
| `src/infrastructure/fs/mod.rs` | Export new module |
| `src/infrastructure/mod.rs` | Export new function |
| `src/application/lockfile_migration.rs` | Use `calvin_home_dir()` |
| `src/config/types.rs` | Use `calvin_home_dir()` |
| `src/config/loader.rs` | Use `calvin_home_dir()` |
| `src/infrastructure/repositories/registry.rs` | Use `calvin_home_dir()` |
| `src/application/deploy/use_case.rs` | Use `calvin_home_dir()` |
| `src/commands/debug.rs` | Use `calvin_home_dir()` |
| `tests/common/windows.rs` | Add `CALVIN_TEST_HOME` |

## Files NOT to Modify (Exceptions)

| File | Reason |
|------|--------|
| `src/ui/primitives/text.rs` | `display_with_tilde()` is cosmetic |
| `src/security/report.rs` | Security checks need real home |
| `src/security/checks.rs` | Security checks need real home |
| `src/domain/services/layer_resolver.rs` | Tilde expansion in user paths |
| `src/infrastructure/sync/local.rs` | Tilde expansion in user paths |
| `src/infrastructure/fs/local.rs` | Tilde expansion in user paths |

## Success Criteria

1. All tests pass on Windows CI
2. No new environment variables needed for future home-dependent features
3. Single point of control for test isolation

## Rollback Plan

If issues arise, can revert to the per-path environment variable approach by:
1. Adding `CALVIN_GLOBAL_LOCKFILE_PATH` override to `global_lockfile_path()`
2. Updating `WindowsCompatExt` with the new variable

This is less elegant but provides the same functionality.

