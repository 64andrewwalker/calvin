# TDD Session: Scope Isolation (SC-7 Revision)

> **Date**: 2025-12-19  
> **Feature**: Multi-scope coexistence and scope-isolated orphan detection  
> **Status**: ✅ Complete (design validated)

---

## Background

During code review of SC-7, we identified a critical design decision:

**Original assumption**: Scope switching means migration (old scope files should be cleaned up)

**Actual user scenario**: Multiple scopes coexist:
```
.promptpack/
├── commands/
│   ├── global-cmd.md      # scope: user  → ~/.claude/commands/
│   └── project-cmd.md     # scope: project → .claude/commands/
```

Users run both:
- `calvin deploy` → deploys `scope: project` assets
- `calvin deploy --home` → deploys `scope: user` assets

**Corrected design**: Each deploy operation only manages orphans within its own scope.

---

## Test Scenarios

### Scenario 1: Scope Isolation (Core Requirement) ✅

**Given**: Lockfile contains entries from both scopes
```toml
[files."home:~/.claude/commands/home.md"]
hash = "sha256:abc"

[files."project:.claude/commands/project.md"]
hash = "sha256:def"
```

**When**: Running `detect_orphans` with `LockfileNamespace::Home` and empty outputs

**Then**: Only `home:*` entries are detected as orphans
- `home:~/.claude/commands/home.md` → orphan ✅
- `project:.claude/commands/project.md` → ignored (different scope) ✅

**Test**: `test_detect_orphans_filters_by_namespace` (existing)

---

### Scenario 2: Multi-Scope Deploy Workflow

**Given**: User has a promptpack with both scopes
```yaml
# .promptpack/commands/global.md
scope: user
---
Global command content

# .promptpack/commands/local.md  
scope: project
---
Local command content
```

**When**: User runs deployments in sequence:
1. `calvin deploy` (project scope)
2. `calvin deploy --home` (home scope)

**Then**:
- After step 1: Lockfile has `project:.claude/commands/local.md`
- After step 2: Lockfile adds `home:~/.claude/commands/global.md`
- **Both entries coexist**, no orphans detected

**Test**: `test_multi_scope_deploy_no_cross_contamination` (new)

---

### Scenario 3: Asset Scope Change (SC-7 Original Intent)

**Given**: Asset previously deployed with `scope: project`
```toml
[files."project:.claude/commands/test.md"]
hash = "sha256:old"
```

**When**: User changes asset scope to `user` and runs `calvin deploy` (project)

**Then**:
- `project:.claude/commands/test.md` is detected as orphan ✅
- (It's no longer in project outputs because scope changed)

**Test**: `test_asset_scope_change_creates_orphan` (new)

---

### Scenario 4: Scope Switch Should NOT Cross-Contaminate

**Given**: User has been using project scope only
```toml
[files."project:.claude/commands/cmd.md"]
hash = "sha256:abc"
```

**When**: User runs `calvin deploy --home` for the first time (with same assets)

**Then**:
- `project:*` entries are NOT detected as orphans
- Only `home:*` orphans (if any) would be detected
- **No prompt to delete project files**

**Test**: `test_scope_switch_no_cross_contamination` (new)

---

### Scenario 5: Same Relative Path, Different Scopes

**Given**: Same file path deployed to both scopes
```toml
[files."home:~/.claude/commands/shared.md"]
hash = "sha256:aaa"

[files."project:.claude/commands/shared.md"]
hash = "sha256:bbb"
```

**When**: Running orphan detection for home scope with empty outputs

**Then**:
- `home:~/.claude/commands/shared.md` → orphan
- `project:.claude/commands/shared.md` → ignored

**Test**: `test_same_path_different_scopes_independent` (new)

---

## Unit Tests to Add

```rust
// src/sync/orphan.rs

#[test]
fn test_multi_scope_deploy_no_cross_contamination() {
    let mut lockfile = Lockfile::new();
    // Simulate: project deploy created this
    lockfile.set_hash("project:.claude/commands/local.md", "sha256:abc");
    // Simulate: home deploy created this
    lockfile.set_hash("home:~/.claude/commands/global.md", "sha256:def");

    // Now deploy home again - only home assets in outputs
    let outputs = vec![
        OutputFile::new(PathBuf::from("~/.claude/commands/global.md"), "content"),
    ];

    let result = detect_orphans(&lockfile, &outputs, LockfileNamespace::Home);
    
    // home file is retained, not orphaned
    assert_eq!(result.retained.len(), 1);
    assert!(result.retained[0].starts_with("home:"));
    
    // project file is NOT detected (scope isolation)
    assert_eq!(result.orphans.len(), 0);
}

#[test]
fn test_scope_switch_no_cross_contamination() {
    let mut lockfile = Lockfile::new();
    // User previously only used project scope
    lockfile.set_hash("project:.claude/commands/cmd.md", "sha256:abc");
    lockfile.set_hash("project:.cursor/rules/rule.md", "sha256:def");

    // Now user switches to home for the first time
    let outputs = vec![
        OutputFile::new(PathBuf::from("~/.claude/commands/cmd.md"), "new content"),
    ];

    let result = detect_orphans(&lockfile, &outputs, LockfileNamespace::Home);
    
    // No orphans - project files should NOT be detected
    assert_eq!(result.orphans.len(), 0);
    // New home file is tracked
    assert_eq!(result.retained.len(), 1);
}

#[test]
fn test_same_path_different_scopes_independent() {
    let mut lockfile = Lockfile::new();
    // Same relative path, but different scopes
    lockfile.set_hash("home:~/.claude/commands/shared.md", "sha256:home");
    lockfile.set_hash("project:.claude/commands/shared.md", "sha256:proj");

    // Home deploy with no outputs (simulating file removed)
    let outputs: Vec<OutputFile> = vec![];

    let result = detect_orphans(&lockfile, &outputs, LockfileNamespace::Home);
    
    // Only home version is orphaned
    assert_eq!(result.orphans.len(), 1);
    assert_eq!(result.orphans[0].key, "home:~/.claude/commands/shared.md");
    
    // Project version is untouched
    let project_result = detect_orphans(&lockfile, &outputs, LockfileNamespace::Project);
    assert_eq!(project_result.orphans.len(), 1);
    assert_eq!(project_result.orphans[0].key, "project:.claude/commands/shared.md");
}
```

---

## Integration Test Scenarios

### CLI Integration Test: `tests/cli_deploy_cleanup.rs`

```rust
#[test]
fn deploy_home_does_not_delete_project_files() {
    // 1. Setup: Deploy to project scope
    // 2. Deploy to home scope with --cleanup
    // 3. Assert: Project files are NOT deleted
}

#[test]
fn deploy_cleanup_only_affects_current_scope() {
    // 1. Setup: Deploy to both scopes
    // 2. Remove an asset, deploy to home with --cleanup
    // 3. Assert: Only home orphans deleted, project files intact
}
```

---

## Files to Modify

| File | Changes |
|------|---------|
| `src/sync/orphan.rs` | **No changes needed** - existing code already correct |
| `src/sync/orphan.rs` (tests) | Add 3 new unit tests |
| `tests/cli_deploy_cleanup.rs` | Add 2 integration tests |
| `docs/impl-plan-sc7-scope.md` | Updated with scope isolation design |

---

## Acceptance Criteria

- [x] `detect_orphans` only returns orphans matching current namespace
- [x] `calvin deploy` does not detect `home:*` entries as orphans
- [x] `calvin deploy --home` does not detect `project:*` entries as orphans
- [x] Multi-scope lockfile entries coexist peacefully
- [ ] Unit tests covering scope isolation scenarios (3 tests)
- [ ] Integration tests for cleanup scope isolation (2 tests)

---

## Implementation Status

| Task | Status |
|------|--------|
| Design decision documented | ✅ Complete |
| `detect_orphans` logic verified | ✅ Correct (no change needed) |
| Unit tests added | 🔲 Pending |
| Integration tests added | 🔲 Pending |
| Code review approved | 🔲 Pending |

