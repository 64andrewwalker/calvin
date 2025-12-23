# API Review: Clean Command

> **Date**: 2024-12-23  
> **Reviewer**: AI Assistant  
> **Scope**: `calvin clean` CLI and `CleanUseCase` public API

## Executive Summary

The Clean command API is well-designed with a clear separation between CLI layer and application layer. The builder pattern for `CleanOptions` follows Rust conventions. A few minor improvements are suggested for API consistency.

---

## 1. CLI Interface Review

### Current Options

| Option | Short | Type | Description |
|--------|-------|------|-------------|
| `--source` | `-s` | Path | Path to .promptpack directory |
| `--home` | - | Flag | Clean only home directory deployments |
| `--project` | - | Flag | Clean only project directory deployments |
| `--all` | - | Flag | Clean all deployments (home + project) |
| `--dry-run` | - | Flag | Preview without deletion |
| `--yes` | `-y` | Flag | Skip confirmation prompt |
| `--force` | `-f` | Flag | Force delete even if modified |

### Consistency Checks ✅

| Check | Status | Notes |
|-------|--------|-------|
| Follows existing CLI patterns | ✅ | Mirrors `deploy` command flags |
| Short flags for common options | ✅ | `-y`, `-f`, `-s` |
| Mutually exclusive options defined | ✅ | `--home`/`--project`/`--all` |
| Help text clarity | ✅ | Each option has clear description |

### Suggestions

1. **Consider `--scope` unified option** (low priority)
   - Alternative: `--scope home,project` instead of separate flags
   - Pro: More flexible for future scope types
   - Con: Current design is simpler and matches PRD

2. **`--target` placeholder** (Phase 3)
   - Add hidden `--target` option that shows "not yet implemented" message
   - Helps users discover future functionality

---

## 2. Application Layer API Review

### `CleanOptions` Structure

```rust
pub struct CleanOptions {
    pub scope: Option<Scope>,
    pub dry_run: bool,
    pub force: bool,
    pub selected_keys: Option<HashSet<String>>,
}
```

**Assessment**: ✅ Good

- Builder pattern: `CleanOptions::new().with_scope(...).with_force(...)` ✅
- Immutable by default, builder returns new instance ✅
- Field names are clear and self-documenting ✅

### `CleanResult` Structure

```rust
pub struct CleanResult {
    pub deleted: Vec<DeletedFile>,
    pub skipped: Vec<SkippedFile>,
    pub errors: Vec<String>,
}
```

**Assessment**: ✅ Good with minor suggestions

| Field | Assessment | Notes |
|-------|------------|-------|
| `deleted` | ✅ Good | Typed as `DeletedFile` with path + key |
| `skipped` | ✅ Good | Typed as `SkippedFile` with reason enum |
| `errors` | ⚠️ Could improve | Consider typed error enum |

**Suggestion**: Replace `errors: Vec<String>` with typed errors:

```rust
pub enum CleanError {
    IoError { path: PathBuf, message: String },
    LockfileError { message: String },
}

pub errors: Vec<CleanError>,
```

### `CleanUseCase` Public API

```rust
impl<LR, FS> CleanUseCase<LR, FS> {
    pub fn new(lockfile_repo: LR, fs: FS) -> Self;
    pub fn execute(&self, lockfile_path: &Path, options: &CleanOptions) -> CleanResult;
    pub fn execute_confirmed(&self, lockfile_path: &Path, options: &CleanOptions) -> CleanResult;
}
```

**Assessment**: ✅ Good

- Two-phase execution (`execute` for preview, `execute_confirmed` for actual) ✅
- Generic over dependencies (testable) ✅
- Lockfile path as parameter (not embedded state) ✅

**Suggestion**: Consider returning `Result<CleanResult, CleanError>` instead of embedding errors in `CleanResult`.

---

## 3. JSON Output Format Review

### Current Events

| Event Type | Fields | Example |
|------------|--------|---------|
| `clean_start` | `scope`, `file_count` | `{"type":"clean_start","scope":"all","file_count":5}` |
| `file_deleted` | `path` | `{"type":"file_deleted","path":"~/.claude/commands/test.md"}` |
| `file_skipped` | `path`, `reason` | `{"type":"file_skipped","path":"...","reason":"modified"}` |
| `clean_complete` | `deleted`, `skipped` | `{"type":"clean_complete","deleted":3,"skipped":2}` |

### Consistency with Other Commands

| Aspect | Clean | Deploy | Check | Consistent? |
|--------|-------|--------|-------|-------------|
| Event naming | snake_case | snake_case | snake_case | ✅ |
| Has start event | ✅ | ✅ | ✅ | ✅ |
| Has complete event | ✅ | ✅ | ✅ | ✅ |
| Per-file events | ✅ | ✅ | ✅ | ✅ |

### Suggestions

1. **Add `errors` to `clean_complete`**:
   ```json
   {"type":"clean_complete","deleted":3,"skipped":2,"errors":0}
   ```

2. **Add `key` to file events** (for programmatic use):
   ```json
   {"type":"file_deleted","path":"~/.claude/commands/test.md","key":"home:~/.claude/commands/test.md"}
   ```

---

## 4. Exit Codes Review

| Exit Code | Meaning | Implemented? |
|-----------|---------|--------------|
| 0 | Success | ✅ |
| 1 | Partial failure / runtime error | ⚠️ Implicit |
| 2 | Invalid arguments | ✅ (clap) |

**Suggestion**: Explicitly return exit code 1 when `result.errors` is non-empty.

---

## 5. Error Handling Review

### Skip Reasons

```rust
pub enum SkipReason {
    Modified,
    Missing,
    NoSignature,
    PermissionDenied,
    Remote,
}
```

**Assessment**: ✅ Complete coverage of all skip scenarios

### Error Message Quality

| Scenario | Current Message | Assessment |
|----------|-----------------|------------|
| No lockfile | "No deployments found. Lockfile does not exist: ..." | ✅ Clear |
| Empty lockfile | "No files to clean. Lockfile is empty." | ✅ Clear |
| Modified file | "modified" (in skip reason) | ✅ Clear |

---

## 6. Security Review

| Security Measure | Status | Notes |
|------------------|--------|-------|
| Signature verification | ✅ | Only deletes files with Calvin signature |
| Hash verification | ✅ | Detects modified files |
| Confirmation prompt | ✅ | Requires explicit `--yes` for non-interactive |
| Force flag guarded | ✅ | `--force` bypasses checks with user consent |
| Path traversal protection | ✅ | Uses lockfile keys, not user input |

---

## Summary

### Strengths

1. Clean separation between CLI and application layers
2. Type-safe options with builder pattern
3. Two-phase execution (preview + confirm) follows best practices
4. Comprehensive skip reason coverage
5. Strong security measures (signature, hash, confirmation)

### Improvement Opportunities

| Priority | Item | Effort |
|----------|------|--------|
| Low | Typed error enum instead of `Vec<String>` | 1h |
| Low | Add `key` field to JSON file events | 15m |
| Low | Add `errors` count to `clean_complete` JSON | 5m |
| Low | Explicit exit code for partial failures | 15m |

### Overall Rating

**API Quality**: ⭐⭐⭐⭐ (4/5)

The API is well-designed and follows Rust and Calvin conventions. The suggested improvements are minor enhancements for completeness.

