# SC-6 Implementation Plan: Orphan Cleanup in Deploy

> **Created**: 2025-12-19  
> **Status**: Complete ✅  
> **Blocked By**: None

---

## Overview

Implement orphan file cleanup functionality in the `deploy` command. When files are no longer part of the PromptPack output (e.g., asset removed, scope changed), Calvin should:

1. Detect these "orphan" files
2. Display them to the user with appropriate styling
3. Offer deletion (interactive) or auto-delete (`--cleanup` flag)

---

## User Experience Design

### Scenario 1: Interactive Mode (default)

```
📦 Calvin Deploy
Source: .promptpack
Target: ~/

● Scanning...
● Writing files...
╭────────────────────────────────╮
│ ✓ Deploy Complete              │
│ 36 assets → 5 targets          │
│ 147 files written              │
╰────────────────────────────────╯

⚠ 3 orphan files detected:
  ✓ ~/.claude/commands/old-cmd.md        [safe]
  ✓ ~/.cursor/rules/removed-rule.md      [safe]
  ⚠ ~/.gemini/workflows/unknown.md       [no signature]

Delete 2 files with Calvin signature? [y/N] y
🗑 Deleted 2 orphan files.
```

### Scenario 2: Auto Mode (`--yes`)

```
📦 Calvin Deploy
...
╭────────────────────────────────╮
│ ✓ Deploy Complete              │
│ 147 files written              │
╰────────────────────────────────╯

⚠ 3 orphan files detected. Run with --cleanup to remove.
```

### Scenario 3: Cleanup Mode (`--cleanup`)

```
📦 Calvin Deploy
...
╭────────────────────────────────╮
│ ✓ Deploy Complete              │
│ 147 files written              │
│ 2 orphans deleted              │
╰────────────────────────────────╯

⚠ 1 file skipped (no Calvin signature): ~/.gemini/workflows/unknown.md
```

### Scenario 4: Cleanup + Force (`--cleanup --force`)

```
📦 Calvin Deploy
...
╭────────────────────────────────╮
│ ✓ Deploy Complete              │
│ 147 files written              │
│ 3 orphans deleted              │
╰────────────────────────────────╯
```

---

## Implementation Steps

### Step 1: Add Orphan Detection to DeployRunner

**File**: `src/commands/deploy/runner.rs`

```rust
impl DeployRunner {
    /// Detect orphan files after deploy
    pub fn detect_orphans(&self, outputs: &[OutputFile]) -> OrphanDetectionResult {
        let lockfile = self.load_lockfile(&self.get_lockfile_path());
        let namespace = match self.target {
            DeployTarget::Home => LockfileNamespace::Home,
            _ => LockfileNamespace::Project,
        };
        detect_orphans(&lockfile, outputs, namespace)
    }
}
```

### Step 2: Add Orphan Renderer

**File**: `src/ui/views/orphan.rs` (NEW)

```rust
pub fn render_orphan_list(
    orphans: &[OrphanFile],
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    // Render list with icons:
    // ✓ = safe to delete (has Calvin signature)
    // ⚠ = unsafe (no signature)
}

pub fn render_orphan_summary(
    deleted: usize,
    skipped: usize,
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    // Render deletion summary
}
```

### Step 3: Add Cleanup Logic to cmd_deploy

**File**: `src/commands/deploy/cmd.rs`

After `runner.run()` completes successfully:

```rust
// After deploy completes
let result = runner.run_with_callback(...)?;

// Detect orphans
let orphan_result = runner.detect_orphans(&outputs);

if !orphan_result.orphans.is_empty() {
    // Check each orphan for signature
    let fs = LocalFileSystem;
    for orphan in &mut orphan_result.orphans {
        check_orphan_status(orphan, &fs);
    }
    
    let safe_count = orphan_result.orphans.iter().filter(|o| o.is_safe_to_delete()).count();
    let unsafe_count = orphan_result.orphans.len() - safe_count;
    
    if cleanup {
        // Auto-delete mode
        delete_orphans(&orphan_result.orphans, force, &fs)?;
    } else if interactive {
        // Interactive mode - ask user
        if safe_count > 0 {
            eprintln!("{}", render_orphan_list(&orphan_result.orphans, ui.color, ui.unicode));
            let confirm = Confirm::new()
                .with_prompt(format!("Delete {} files with Calvin signature?", safe_count))
                .default(false)
                .interact()?;
            if confirm {
                delete_orphans(&orphan_result.orphans, false, &fs)?;
            }
        }
    } else {
        // Non-interactive, non-cleanup: just warn
        eprintln!("⚠ {} orphan files detected. Run with --cleanup to remove.", orphan_result.orphans.len());
    }
}
```

### Step 4: Add Delete Function

**File**: `src/sync/orphan.rs`

```rust
/// Delete orphan files
pub fn delete_orphans<FS: FileSystem + ?Sized>(
    orphans: &[OrphanFile],
    force: bool,  // If true, delete even without signature
    fs: &FS,
) -> CalvinResult<DeleteResult> {
    let mut deleted = Vec::new();
    let mut skipped = Vec::new();
    
    for orphan in orphans {
        if !orphan.exists {
            continue;  // Already gone
        }
        
        if force || orphan.is_safe_to_delete() {
            let path = expand_home_dir(&PathBuf::from(&extract_path_from_key(&orphan.key)));
            fs.remove_file(&path)?;
            deleted.push(orphan.key.clone());
        } else {
            skipped.push(orphan.key.clone());
        }
    }
    
    Ok(DeleteResult { deleted, skipped })
}
```

### Step 5: Update Lockfile After Deletion

After deleting orphans, remove them from lockfile:

```rust
for key in &delete_result.deleted {
    lockfile.remove(key);
}
lockfile.save(&lockfile_path, &fs)?;
```

### Step 6: Add Icons and Colors

**File**: `src/ui/theme.rs`

```rust
// Add new icon
pub const TRASH: &str = "🗑";
```

**File**: `src/ui/primitives/icon.rs`

```rust
pub enum Icon {
    // ... existing
    Trash,
}
```

---

## Color Scheme

| Element | Color | Description |
|---------|-------|-------------|
| Safe orphan (has signature) | `SUCCESS` (Green) | ✓ Safe to delete |
| Unsafe orphan (no signature) | `WARNING` (Yellow) | ⚠ Manual review needed |
| Deleted count | `SUCCESS` (Green) | Deletion successful |
| Skipped count | `WARNING` (Yellow) | Not deleted |
| Error | `ERROR` (Red) | Deletion failed |

---

## Test Cases (TDD)

### Unit Tests

1. **`test_delete_orphans_safe_only`**: Only deletes files with signature
2. **`test_delete_orphans_force_all`**: Deletes all with --force
3. **`test_delete_orphans_nonexistent_skipped`**: Skips files that don't exist
4. **`test_orphan_list_render_colors`**: Correct colors for safe/unsafe

### Integration Tests

1. **`test_deploy_cleanup_removes_orphans`**: E2E test with --cleanup flag
2. **`test_deploy_no_cleanup_warns_only`**: Without --cleanup, only warns

---

## Files to Modify

| File | Changes |
|------|---------|
| `src/commands/deploy/cmd.rs` | Add cleanup logic after deploy |
| `src/commands/deploy/runner.rs` | Add `detect_orphans()` method |
| `src/sync/orphan.rs` | Add `delete_orphans()` function |
| `src/ui/views/orphan.rs` | **NEW** - Orphan list renderer |
| `src/ui/views/mod.rs` | Add orphan module |
| `src/ui/theme.rs` | Add TRASH icon |
| `src/ui/primitives/icon.rs` | Add Trash variant |

---

## Edge Cases

1. **Orphan file already deleted**: Skip gracefully
2. **Permission denied**: Report error but continue with others
3. **Dry run mode**: Show what would be deleted, don't delete
4. **Remote target**: Skip orphan cleanup (SSH deletion complex)
5. **Empty orphan list**: No output, no prompt

---

## Acceptance Criteria

- [ ] `calvin deploy` shows orphan count after completion
- [ ] `calvin deploy --cleanup` deletes files with Calvin signature
- [ ] `calvin deploy --cleanup --force` deletes all orphans
- [ ] Interactive mode prompts for confirmation
- [ ] `--yes` mode only warns, doesn't auto-delete
- [ ] Files without Calvin signature are listed but not deleted
- [ ] Lockfile is updated after deletion
- [ ] Colors and icons match design spec

---

## Estimated Effort

| Step | Time |
|------|------|
| Step 1: Runner integration | 10 min |
| Step 2: Orphan renderer | 15 min |
| Step 3: cmd_deploy logic | 20 min |
| Step 4: Delete function | 10 min |
| Step 5: Lockfile update | 5 min |
| Step 6: Icons/colors | 5 min |
| Testing | 15 min |
| **Total** | **~80 min** |
