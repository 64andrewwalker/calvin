# Binary Skill Assets PRD

> **Status**: Implemented  
> **Author**: Code Review (AI Assistant)  
> **Created**: 2025-12-30  
> **Implemented**: 2025-12-30  
> **Target Version**: 0.8.0  
> **Related**: [Skills Support PRD](skills-support-prd.md)

---

## Implementation Progress

### Current vs Target State

| Component | Location | Current | Target |
|-----------|----------|---------|--------|
| `Asset.binary_supplementals` | `domain/entities/asset.rs` | ✅ Done | ✅ Done |
| `Asset.warnings()` | `domain/entities/asset.rs` | ✅ Done | ✅ Done |
| `BinaryOutputFile` entity | `domain/entities/output_file.rs` | ✅ Done | ✅ Done |
| `FileSystem::write_binary()` | `domain/ports/file_system.rs` | ✅ Done | ✅ Done |
| `LocalFs::write_binary()` | `infrastructure/fs/local.rs` | ✅ Done | ✅ Done |
| `RemoteFs::write_binary()` | `infrastructure/fs/remote.rs` | ✅ Done | ✅ Done |
| `SkillCompileResult.binary_outputs` | `infrastructure/adapters/skills.rs` | ✅ Done | ✅ Done |
| `TargetAdapter::compile_binary()` | `domain/ports/target_adapter.rs` | ✅ Done | ✅ Done |
| Adapter integration | `adapters/*.rs` | ✅ Done | ✅ Done |
| Deploy writer | `application/deploy/use_case.rs` | ✅ Done | ✅ Done |
| Lockfile binary tracking | `domain/entities/lockfile.rs` | ✅ Done | ✅ Done |
| Orphan detection | `domain/services/orphan_detector.rs` | ✅ Done | ✅ Done |

**Legend**: ✅ Complete | ⚠️ Partial | 🔴 Not started | ⏳ Deferred

### TODO Tracking

#### Phase 0: Preparation ✅ Complete

- [x] Fix dead code warning (`#[allow(dead_code)]` with documentation)
- [x] Fix misleading warning message ("copying" → "included")
- [x] Add `BinaryOutputFile` tests (7 tests added)
- [x] Create this PRD

#### Phase 1: Adapter Integration ✅ Complete

- [x] Add `compile_binary()` to `TargetAdapter` trait
  - File: `src/domain/ports/target_adapter.rs`
  
- [x] Implement `ClaudeCodeAdapter::compile_binary()`
  - File: `src/infrastructure/adapters/claude_code.rs`
  
- [x] Implement `CodexAdapter::compile_binary()`
  - File: `src/infrastructure/adapters/codex.rs`
  
- [x] Implement `CursorAdapter::compile_binary()`
  - File: `src/infrastructure/adapters/cursor.rs`

- [x] Write tests: `test_compile_binary_returns_skill_binaries` (3 adapters)
- [x] Write tests: `test_compile_binary_empty_for_non_skills` (3 adapters)
- [x] Remove `#[allow(dead_code)]` from `binary_outputs`

#### Phase 2: Deploy Writer ✅ Complete

- [x] Update `compile_assets()` to collect binary outputs
  - File: `src/application/deploy/use_case.rs`
  
- [x] Add binary writing step using `write_binary()`
  - File: `src/application/deploy/use_case.rs`

- [x] Write tests: `test_deploy_writes_binary_outputs`
- [x] Write tests: `test_deploy_dry_run_includes_binary_outputs_in_written_list`

#### Phase 3: Lockfile Tracking ✅ Complete

- [x] Add `is_binary` field to `LockfileEntry`
  - File: `src/domain/entities/lockfile.rs`
  
- [x] Create lockfile entries for binary outputs
  - File: `src/application/deploy/use_case.rs`

- [x] Write tests: `test_deploy_tracks_binary_files_in_lockfile_with_is_binary_flag`

#### Phase 4: Orphan Detection ✅ Complete

- [x] Update `OrphanDetector` for binary file paths
  - File: `src/domain/services/orphan_detector.rs`
  - Added `detect_with_binaries()` method

- [x] Write tests: `test_detect_with_binaries_includes_binary_outputs`
- [x] Write tests: `test_detect_with_binaries_finds_removed_binary_orphan`

#### Phase 5: Cleanup ✅ Complete

- [x] Binary files are now deployed and tracked correctly
- [x] `binary_outputs` field is actively used in deploy flow
- [x] Warning message updated to "binary file included"

#### Phase 6: Polish (Remaining)

- [x] Update warning message to "will be deployed" (accurate now that deploy works)
  - File: `src/infrastructure/repositories/asset.rs`
  
- [x] Add CHANGELOG entry for binary skill assets feature
  - File: `CHANGELOG.md`
  
- [x] Update user documentation for skills with binary assets
  - File: `docs/skills.md`

---

## Self-Critique: Issues in Original Implementation

| Issue | Problem | Resolution |
|-------|---------|------------|
| **Dead code warning** | `binary_outputs` field never read | ✅ Fixed: Now consumed by `compile_binary()` |
| **Misleading warning** | "copying binary file" when nothing copied | ✅ Fixed: Changed to "binary file included" |
| **Missing tests** | No tests for `BinaryOutputFile.hash()` | ✅ Fixed: Added 7 tests |
| **No clear roadmap** | Unclear what work remained | ✅ Fixed: Created this PRD with TODO checklist |
| **Binary discarded** | Adapters return `result.outputs`, discard `.binary_outputs` | ✅ Fixed: Adapters now return binary outputs via `compile_binary()` |
| **No lockfile tracking** | Binary files not tracked | ✅ Fixed: Added `is_binary` field to `LockfileEntry` |

---

## Executive Summary

This PRD proposes completing **binary file support** for skills in Calvin. Currently, binary files (images, PDFs, etc.) in skill directories are loaded into memory but **not written to disk** during deploy.

**Current State**: Binary files are detected → loaded to `binary_supplementals` → compiled to `BinaryOutputFile` → **discarded by adapters**.

**Target State**: Binary files are deployed alongside text files, tracked in lockfile, and cleaned up properly.

---

## 1. Problem Statement

### Current Behavior

When a skill directory contains binary files:

```
.promptpack/skills/my-skill/
├── SKILL.md
├── reference.md
└── assets/
    ├── logo.png        # Binary file
    └── diagram.pdf     # Binary file
```

Calvin shows:
```
⚠ 2 warnings:
- Skill 'my-skill': binary file 'assets/logo.png' included (45.2 KB)
- Skill 'my-skill': binary file 'assets/diagram.pdf' included (123.4 KB)
✓ Deploy successful
```

But the **actual output**:
```
.claude/skills/my-skill/
├── SKILL.md            ✓ Written
├── reference.md        ✓ Written
└── assets/             ✗ MISSING!
    ├── logo.png        ✗ NOT written
    └── diagram.pdf     ✗ NOT written
```

### Pain Points

| Issue | Impact | User Experience |
|-------|--------|-----------------|
| Binary files not deployed | Skills with images/PDFs incomplete | Confusion: "where are my files?" |
| Warning says "included" | Misleading | User thinks files are deployed |
| No lockfile tracking | Binary files not cleaned | Orphan files accumulate |
| Incomplete skills | AI can't see referenced assets | Skill functionality broken |

### Root Cause

In adapters (e.g., `src/infrastructure/adapters/claude_code.rs`):

```rust
let result = skills::compile_skill_outputs(...)?;
// For now, return only text outputs. Binary outputs will be handled separately.
return Ok(result.outputs);  // <-- binary_outputs DISCARDED!
```

---

## 2. Goals

### Primary Goals

1. **Deploy binary files** to disk during `calvin deploy`
2. **Track binary files** in lockfile with content hashes
3. **Clean up binary files** via `calvin clean` and `--cleanup`
4. **Detect orphaned binaries** when skills are modified

### Non-Goals

- **NOT** implementing binary support for remote destinations (SSH/rsync) — deferred
- **NOT** implementing file size limits — user responsibility
- **NOT** implementing binary type validation — allow any binary
- **NOT** implementing image optimization/transformation

### Success Criteria

1. Binary files appear in output skill directories
2. `calvin.lock` contains entries for binary files
3. `calvin clean` removes binary files tracked in lockfile
4. Orphan detection removes stale binary files after skill modification
5. Warning message accurately reflects deployment status

---

## 3. Design Philosophy

### Principle 1: Minimal Trait Change

Add `compile_binary()` as a **separate method** with default empty implementation:

```rust
pub trait TargetAdapter {
    fn compile(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError>;
    
    // NEW: Default returns empty
    fn compile_binary(&self, asset: &Asset) -> Result<Vec<BinaryOutputFile>, AdapterError> {
        Ok(vec![])
    }
}
```

**Rationale**: Backward compatible. Existing adapters don't need changes. Only skill-supporting adapters implement it.

### Principle 2: Reuse Existing Infrastructure

Binary writing uses existing `FileSystem::write_binary()` trait method (already implemented for `LocalFs`).

```rust
// Already exists:
fn write_binary(&self, path: &Path, content: &[u8]) -> FsResult<()>;
```

No new dependencies required.

### Principle 3: Consistent Lockfile Format

Binary files use same lockfile entry format as text files:

```toml
[files."project:.claude/skills/my-skill/assets/logo.png"]
hash = "sha256:89504e470d0a1a0a..."
source_layer = "project"
source_asset = "my-skill"
is_binary = true  # Optional marker
```

**Rationale**: Orphan detection, clean, and provenance work unchanged.

### Principle 4: Fail Loudly for Remote

Remote destinations (`RemoteFs`) should return explicit error, not silently skip:

```rust
fn write_binary(&self, path: &Path, _content: &[u8]) -> FsResult<()> {
    Err(FsError::Other(format!(
        "Binary file write not yet supported for remote destinations: {}",
        path.display()
    )))
}
```

**Rationale**: User knows their skill is incomplete. Don't pretend it worked.

---

## 4. Detailed Design

### 4.1 TargetAdapter Extension

```rust
// src/domain/ports/target_adapter.rs

pub trait TargetAdapter: Send + Sync {
    // ... existing methods ...
    
    /// Compile binary outputs for an asset (skills only).
    /// 
    /// Default implementation returns empty vec for non-skill assets.
    fn compile_binary(&self, asset: &Asset) -> Result<Vec<BinaryOutputFile>, AdapterError> {
        Ok(vec![])
    }
}
```

### 4.2 Adapter Implementation (ClaudeCode Example)

```rust
// src/infrastructure/adapters/claude_code.rs

impl TargetAdapter for ClaudeCodeAdapter {
    fn compile_binary(&self, asset: &Asset) -> Result<Vec<BinaryOutputFile>, AdapterError> {
        if asset.kind() != AssetKind::Skill {
            return Ok(vec![]);
        }
        
        let footer = self.footer(&asset.source_path_normalized());
        let result = skills::compile_skill_outputs(
            asset,
            self.skills_dir(asset.scope()),
            Target::ClaudeCode,
            &footer,
        )?;
        
        Ok(result.binary_outputs)
    }
}
```

### 4.3 Deploy Writer Integration

```rust
// src/application/deploy/use_case.rs

fn compile_assets(...) -> Result<(Vec<OutputFile>, Vec<BinaryOutputFile>, ...), Error> {
    let mut text_outputs = Vec::new();
    let mut binary_outputs = Vec::new();
    
    for asset in assets {
        for adapter in &active_adapters {
            // Existing: text outputs
            text_outputs.extend(adapter.compile(asset)?);
            
            // NEW: binary outputs
            binary_outputs.extend(adapter.compile_binary(asset)?);
        }
    }
    
    Ok((text_outputs, binary_outputs, provenance))
}

fn write_outputs(...) -> Result<(), Error> {
    // Existing: write text files
    for output in &text_outputs {
        fs.write(output.path(), output.content())?;
    }
    
    // NEW: write binary files
    for output in &binary_outputs {
        fs.write_binary(output.path(), output.content())?;
    }
    
    Ok(())
}
```

### 4.4 Lockfile Entry for Binaries

```rust
// src/domain/entities/lockfile.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockfileEntry {
    pub hash: String,
    pub source_layer: String,
    pub source_layer_path: PathBuf,
    pub source_asset: String,
    pub source_file: PathBuf,
    
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_binary: bool,  // NEW: Optional marker
}
```

### 4.5 Path Handling

Binary outputs use same path resolution as text outputs:

| Scope | Asset | Binary Path | Output Path |
|-------|-------|-------------|-------------|
| Project | `my-skill` | `assets/logo.png` | `.claude/skills/my-skill/assets/logo.png` |
| User | `my-skill` | `assets/logo.png` | `~/.claude/skills/my-skill/assets/logo.png` |

---

## 5. Edge Cases

### 5.1 Large Binary Files

**Behavior**: No size limit. User responsibility.

**Rationale**: Skills with large assets are rare. Adding limits complicates UX without clear benefit.

**Future**: Could add `--max-binary-size` flag or config option.

### 5.2 Remote Destinations

**Behavior**: Explicit error. Binary deploy aborted.

```
Error: Binary file write not supported for remote: user@host:/path
  - Skill 'my-skill' contains binary files
  - Use local deploy or skip binary files
```

**Future**: Implement base64 encoding or stdin piping.

### 5.3 Empty Binary Files

**Behavior**: Zero-byte files are valid. Deploy as-is.

### 5.4 Binary File in Non-Skill

**Behavior**: Binary files only loaded for skills. Non-skill assets ignore binary_supplementals.

### 5.5 Orphaned Binary After Skill Rename

**Behavior**: Same as text orphan detection. Old binary files become orphans, removed on `--cleanup`.

---

## 6. Implementation Plan (TDD)

### Phase 1: Adapter Integration

| Contract | Test | Status |
|----------|------|--------|
| BINARY-001 | `TargetAdapter` has `compile_binary()` method | [ ] |
| BINARY-002 | `ClaudeCodeAdapter::compile_binary()` returns skill binaries | [ ] |
| BINARY-003 | `CodexAdapter::compile_binary()` returns skill binaries | [ ] |
| BINARY-004 | `CursorAdapter::compile_binary()` returns skill binaries | [ ] |
| BINARY-005 | `compile_binary()` empty for non-skill assets | [ ] |
| BINARY-006 | `compile_binary()` empty for skills without binary files | [ ] |

**Files**:
- `src/domain/ports/target_adapter.rs`
- `src/infrastructure/adapters/claude_code.rs`
- `src/infrastructure/adapters/codex.rs`
- `src/infrastructure/adapters/cursor.rs`
- `src/infrastructure/adapters/skills.rs`

**Estimated**: 1-2 days

### Phase 2: Deploy Writer

| Contract | Test | Status |
|----------|------|--------|
| BINARY-007 | `DeployUseCase` calls `compile_binary()` for each asset | [ ] |
| BINARY-008 | Binary outputs written using `write_binary()` | [ ] |
| BINARY-009 | Binary write failure reported as deploy error | [ ] |
| BINARY-010 | Binary file path respects scope (project vs user) | [ ] |

**Files**:
- `src/application/deploy/use_case.rs`

**Estimated**: 1-2 days

### Phase 3: Lockfile Tracking

| Contract | Test | Status |
|----------|------|--------|
| BINARY-011 | Lockfile tracks binary files with hash | [ ] |
| BINARY-012 | Binary lockfile entry has `is_binary: true` | [ ] |
| BINARY-013 | `BinaryOutputFile::hash()` used for content hash | [ ] |

**Files**:
- `src/domain/entities/lockfile.rs`
- `src/application/deploy/use_case.rs`

**Estimated**: 1 day

### Phase 4: Orphan Detection

| Contract | Test | Status |
|----------|------|--------|
| BINARY-014 | `OrphanDetector` finds removed binary files | [ ] |
| BINARY-015 | Orphaned binaries deleted on `--cleanup` | [ ] |
| BINARY-016 | Empty asset directories pruned after binary removal | [ ] |

**Files**:
- `src/domain/services/orphan_detector.rs`
- `src/application/deploy/use_case.rs`

**Estimated**: 1 day

### Phase 5: Warning Message & Cleanup

| Contract | Test | Status |
|----------|------|--------|
| BINARY-017 | Warning says "will be deployed" (accurate) | [ ] |
| BINARY-018 | Remove `#[allow(dead_code)]` from `binary_outputs` | [ ] |

**Files**:
- `src/infrastructure/repositories/asset.rs`
- `src/infrastructure/adapters/skills.rs`

**Estimated**: 0.5 days

### Phase 0: Preparation ✅ COMPLETE

| Task | Status |
|------|--------|
| Create review report | ✅ |
| Fix dead code warning | ✅ |
| Fix misleading warning | ✅ |
| Add `BinaryOutputFile` tests | ✅ |
| Create this PRD | ✅ |

---

## 7. Security Considerations

| Risk | Mitigation |
|------|------------|
| Path traversal (`../`) | Already validated in `compile_skill_outputs()` |
| Symlinks | Already rejected during skill loading |
| Large files DoS | User responsibility (no limit) |
| Binary content injection | No execution, just file copy |

---

## 8. Dependencies

No new dependencies required. Uses existing:

| Crate | Purpose |
|-------|---------|
| `sha2` | Content hashing (already used) |
| `serde` | Lockfile serialization (already used) |

---

## 9. Testing Strategy

### Unit Tests

```rust
#[test]
fn test_compile_binary_returns_skill_binaries() {
    let asset = create_skill_asset("my-skill")
        .with_binary_supplementals(hashmap! {
            PathBuf::from("logo.png") => vec![0x89, 0x50, 0x4E, 0x47]
        });
    
    let adapter = ClaudeCodeAdapter::new();
    let outputs = adapter.compile_binary(&asset).unwrap();
    
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].path(), &PathBuf::from(".claude/skills/my-skill/logo.png"));
    assert_eq!(outputs[0].content(), &[0x89, 0x50, 0x4E, 0x47]);
}

#[test]
fn test_compile_binary_empty_for_non_skill() {
    let asset = create_policy_asset("my-policy");
    
    let adapter = ClaudeCodeAdapter::new();
    let outputs = adapter.compile_binary(&asset).unwrap();
    
    assert!(outputs.is_empty());
}
```

### Integration Tests

```rust
#[test]
fn deploy_writes_binary_skill_files() {
    let env = TestEnv::new();
    env.create_skill_with_binary("my-skill", "logo.png", &[0x89, 0x50, 0x4E, 0x47]);
    
    let result = env.run(["calvin", "deploy", "--targets", "claude-code"]);
    assert!(result.success);
    
    let output_path = env.path(".claude/skills/my-skill/logo.png");
    assert!(output_path.exists());
    assert_eq!(fs::read(&output_path).unwrap(), vec![0x89, 0x50, 0x4E, 0x47]);
}

#[test]
fn lockfile_tracks_binary_files() {
    let env = TestEnv::new();
    env.create_skill_with_binary("my-skill", "logo.png", &[0x89, 0x50, 0x4E, 0x47]);
    
    env.run(["calvin", "deploy"]);
    
    let lockfile = env.read_lockfile();
    let entry = lockfile.files.get("project:.claude/skills/my-skill/logo.png");
    assert!(entry.is_some());
    assert!(entry.unwrap().is_binary);
}

#[test]
fn cleanup_removes_orphaned_binary() {
    let env = TestEnv::new();
    env.create_skill_with_binary("my-skill", "old.png", &[1, 2, 3]);
    env.run(["calvin", "deploy"]);
    
    // Remove binary from source
    env.remove(".promptpack/skills/my-skill/old.png");
    
    // Redeploy with cleanup
    env.run(["calvin", "deploy", "--cleanup"]);
    
    assert!(!env.path(".claude/skills/my-skill/old.png").exists());
}
```

---

## 10. Open Questions

### Resolved

| Question | Decision | Rationale |
|----------|----------|-----------|
| Separate method or combined return? | Separate `compile_binary()` | Backward compatible |
| Remote support? | Defer, return error | Explicit failure better than silent skip |
| Lockfile format? | Same as text with `is_binary` | Consistent, orphan detection works |
| File size limit? | No limit | User responsibility |

### Future Considerations

| Feature | Status |
|---------|--------|
| Remote binary deploy via base64 | Deferred to v0.9+ |
| Binary size warnings | Could add in v0.8.1 |
| Image optimization | Not planned |

---

## 11. Estimated Effort

| Phase | Days | Tests | Tasks |
|-------|------|-------|-------|
| 0. Preparation | 0.5 | 7 | ✅ Complete |
| 1. Adapter Integration | 1-2 | 6 | 5 |
| 2. Deploy Writer | 1-2 | 4 | 3 |
| 3. Lockfile Tracking | 1 | 3 | 2 |
| 4. Orphan Detection | 1 | 3 | 2 |
| 5. Warning & Cleanup | 0.5 | 2 | 2 |
| **Total** | **5-7** | **25** | **14** |

---

## 12. References

- [Skills Support PRD](skills-support-prd.md) — Original skills implementation
- [Testing Philosophy](../testing/TESTING_PHILOSOPHY.md) — TDD principles
- [Calvinignore PRD](calvinignore-prd.md) — Similar feature PRD structure

---

## Version History

| Date | Author | Change |
|------|--------|--------|
| 2025-12-30 | Code Review | Initial draft from review findings |
| 2025-12-30 | Code Review | Added implementation progress, self-critique, full structure |
