# SC-7 Implementation Plan: Lockfile Scope Tracking

> **Created**: 2025-12-19  
> **Updated**: 2025-12-19  
> **Status**: Complete ✅

---

## Problem Statement

When an asset's `scope` attribute changes (e.g., from `project` to `user`), files previously 
deployed under the old scope become orphans. However, the current lockfile only stores the 
file hash, not the deployment scope. This makes it impossible to:

1. Detect scope-based orphans accurately
2. Track which scope each file was deployed under
3. Clean up files when an asset's scope changes

---

## Design Decision: Scope Isolation

### Key Principle

**Each deploy operation only manages orphans within its own scope.**

| Command | Scope | Orphan Detection |
|---------|-------|------------------|
| `calvin deploy` | project | Only detects `project:*` orphans |
| `calvin deploy --home` | home | Only detects `home:*` orphans |

### Rationale

Users often maintain **both** scopes simultaneously:

```
.promptpack/
├── commands/
│   ├── global-cmd.md      # scope: user  → ~/.claude/commands/
│   └── project-cmd.md     # scope: project → .claude/commands/
```

In this scenario:
1. `calvin deploy` deploys `scope: project` assets to project directory
2. `calvin deploy --home` deploys `scope: user` assets to home directory
3. **Both scopes coexist**, they are not mutually exclusive

If we detected cross-scope orphans, running `deploy --home` would incorrectly mark 
all `project:*` files as orphans and suggest deleting them.

### What SC-7 Actually Solves

SC-7 handles the case when an **asset's scope attribute changes**:

1. User has `commands/test.md` with `scope: project`
2. User changes it to `scope: user`
3. Next `calvin deploy` (project scope):
   - `test.md` is no longer in project outputs
   - `project:.claude/commands/test.md` is detected as orphan ✅
4. Next `calvin deploy --home` (home scope):
   - `test.md` is now in home outputs
   - `home:~/.claude/commands/test.md` is deployed ✅

### Cross-Scope Cleanup (Future)

For users who want to **migrate** from one scope to another (e.g., permanently switching 
from project to home), a future command could be added:

```bash
# Potential future feature
calvin cleanup --all-scopes    # Clean orphans across all scopes
calvin cleanup --scope home    # Clean only home scope orphans
```

This requires explicit user action, preventing accidental deletion.

---

## Solution

Scope information is encoded in the **lockfile key prefix**, not as a separate field.

### Lockfile Structure

```toml
# Scope is part of the key prefix
[files."home:~/.claude/commands/test.md"]
hash = "sha256:abc123..."

[files."project:.claude/commands/project.md"]
hash = "sha256:def456..."
```

Key format:
- `home:~/<path>` - Files deployed to home directory
- `project:<path>` - Files deployed to project directory

---

## Implementation

### Scope Inference

Scope is inferred from the key prefix at runtime:

```rust
pub fn get_scope(&self, path: &str) -> Option<&'static str> {
    if self.files.contains_key(path) {
        if path.starts_with("home:") {
            Some("home")
        } else if path.starts_with("project:") {
            Some("project")
        } else {
            None
        }
    } else {
        None
    }
}
```

### detect_orphans

**Important**: Only detect orphans within the current namespace:

```rust
pub fn detect_orphans(
    lockfile: &Lockfile,
    outputs: &[OutputFile],
    namespace: LockfileNamespace,
) -> OrphanDetectionResult {
    let prefix = match namespace {
        LockfileNamespace::Project => "project:",
        LockfileNamespace::Home => "home:",
    };

    for key in lockfile.paths() {
        // ✅ Only check entries matching current namespace
        if !key.starts_with(prefix) {
            continue;  // Skip other scopes
        }
        // ... orphan detection logic
    }
}
```

---

## Backward Compatibility

- No breaking changes - lockfile format unchanged
- Scope is always inferred from key prefix
- Old lockfiles work without modification

---

## Test Cases

1. **Scope inference**: `get_scope("home:...")` returns `"home"`
2. **Old lockfiles load**: Lockfiles load correctly
3. **Scope-based orphan detection**: Files with changed scope are detected as orphans
4. **Scope isolation**: `deploy --home` does NOT detect `project:*` as orphans
5. **Multi-scope coexistence**: Both home and project files can coexist in lockfile

---

## Estimated Effort

| Step | Time |
|------|------|
| Implement scope inference | 5 min |
| Update orphan detection | 10 min |
| Testing | 10 min |
| **Total** | **~25 min** |
