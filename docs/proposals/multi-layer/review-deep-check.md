# Deep Cross-Reference Check: PRD vs Plans

> Second-pass self-criticism to ensure nothing is missed.

## PRD Section-by-Section Verification

### §1-3: Problem, Goals, Philosophy
✅ Covered in design.md

### §4: Design Overview

| Subsection | Covered In | Status |
|------------|------------|--------|
| §4.1 Layer Hierarchy | plan-01 | ✅ |
| §4.2 Merge Semantics | plan-01 Task 1.3 | ✅ |
| §4.3 Configuration Schema | plan-03 Task 3.1 | ✅ |
| §4.4 CLI Interface | plan-03 Tasks 3.2-3.5 | ✅ |

### §5: Detailed Design

| Subsection | Covered In | Status | Notes |
|------------|------------|--------|-------|
| §5.1 Layer Resolution | plan-01 Task 1.2 | ✅ | |
| §5.2 Asset Merge | plan-01 Task 1.3 | ✅ | |
| §5.3 Directory Structure | Implicit | ✅ | |
| §5.4 Error Handling | todo.md Error section | ✅ | |
| §5.5 Asset Layer Migration | ? | ⚠️ **MISSING** | 需要添加到 plan-01 |
| §5.6 Symlink Handling | plan-01? | ⚠️ **PARTIAL** | 需要显式添加测试 |

### §6: Migration & Compatibility
✅ Covered in multi-layer-migration-guide.md

### §7: Implementation Plan
✅ Covered in all plan-XX documents

### §8: Security Considerations

| Item | Covered In | Status |
|------|------------|--------|
| 路径验证 | Implicit | ⚠️ Need explicit task |
| 符号链接循环检测 | plan-01? | ⚠️ Need explicit task |
| 权限检查 | Implicit | ⚠️ Need explicit task |
| 配置来源限制 | Not covered | ⚠️ **MISSING** |

### §9: Lockfile Architecture
✅ Covered in plan-00

### §10: Output Provenance

| Subsection | Covered In | Status |
|------------|------------|--------|
| §10.1-10.2 Lockfile tracking | plan-00 | ✅ |
| §10.3 Provenance command | plan-04 Task 4.2 | ✅ |
| §10.4 Verbose deploy output | plan-01 Task 1.5 | ✅ |
| §10.5 Data Structure | plan-00 Task 0.1 | ✅ |

### §11: Global Registry
✅ Covered in plan-02

### §12: Interactive UI

| Subsection | Covered In | Status |
|------------|------------|--------|
| §12.1 Layer Selection View | plan-01? | ⚠️ Not explicit |
| §12.2 Override Confirmation | plan-01 verbose mode | ✅ |
| §12.3 Provenance Report View | plan-04 Task 4.2 | ✅ |
| §12.4 Projects List View | plan-02 Task 2.6 | ✅ |
| §12.5 Layer Initialization View | plan-03 Task 3.5 | ✅ |

### §13: Error Handling
✅ Covered in todo.md Error Handling section

### §14: Edge Cases

| Subsection | Covered In | Status | Notes |
|------------|------------|--------|-------|
| §14.1 Version Compatibility | plan-00 | ✅ | |
| §14.2 Config Merge Rules | design.md | ✅ | |
| §14.3 Remote Deploy | NOT COVERED | ❌ **MISSING** | Need explicit decision |
| §14.4 Watch Mode | NOT COVERED | ❌ **MISSING** | Need explicit decision |
| §14.5 Env Var Priority | plan-03? | ⚠️ Implicit | |
| §14.6 Registry Concurrency | plan-02 | ✅ | File locks |
| §14.7 Git Integration | NOT COVERED | ⚠️ | Optional, can defer |
| §14.8 Windows Path | plan-00 Task 0.5 | ✅ | Added in review |

### §15: Open Questions
These are intentionally deferred - OK

### §16: Success Metrics
These are acceptance criteria - implicitly covered in verification sections

---

## Missing Items Summary

### Critical (Must Add)

1. **§5.5 Asset Layer Migration Detection**
   - When asset moves from one layer to another
   - Need to add to plan-01

2. **§14.3 Remote Deploy Behavior**
   - PRD says: "只使用项目层，忽略本地用户层"
   - Need explicit note in plan-01 or plan-03

3. **§14.4 Watch Mode with Multi-Layer**
   - PRD says: "默认只监听项目层"
   - Need to add to plan-01 or create plan-05

4. **§8 Security: Config Source Restriction**
   - "不允许 project layer 添加任意外部路径"
   - Need to add validation to plan-03

### Medium (Should Add)

5. **§5.6 Symlink Handling Tests**
   - Need explicit test cases in plan-01

6. **§12.1 Layer Selection View in Interactive Mode**
   - Currently implicit in verbose output
   - Consider adding to plan-01 or plan-04

### Low (Nice to Have)

7. **§14.7 Git Integration / .gitignore**
   - Recommendations for what to commit
   - Can add to documentation phase

---

## Recommended Fixes

### Fix 1: Add Asset Layer Migration to plan-01

```markdown
### Task 1.6: Handle Asset Layer Migration

When an asset moves from one layer to another between deploys:

**Scenario**:
1. First deploy: asset "review" from project layer
2. User deletes project's review.md
3. Second deploy: asset "review" now from user layer

**Behavior**:
- Same output path → update lockfile source_layer, no orphan
- Different output path → old becomes orphan, new is fresh

**Tests**:
- [ ] Asset migrates from project to user layer
- [ ] Asset migrates from user to project layer
- [ ] Lockfile correctly updates source_layer
```

### Fix 2: Add Remote Deploy Note to plan-03

```markdown
### Note: Remote Deploy Behavior

When using `--remote`, multi-layer is simplified:
- Only project layer (`.promptpack/`) is used
- User layer and additional layers are ignored
- Rationale: Remote machine's layer structure may differ
```

### Fix 3: Add Watch Mode Decision to plan-01 or design.md

```markdown
### Watch Mode with Multi-Layer

**Default Behavior**: Only watch project layer

**Rationale**:
- User layer changes are rare
- Watching multiple directories has performance cost
- User can manually re-run `calvin deploy` for global changes

**Future Option**: `--watch-all-layers` flag
```

### Fix 4: Add Security Validation to plan-03

```markdown
### Task 3.6: Security Validation

**Constraint**: Project config cannot ADD external paths

```rust
fn validate_project_config(config: &ProjectConfig) -> Result<(), ConfigError> {
    if !config.sources.additional_layers.is_empty() {
        return Err(ConfigError::SecurityViolation {
            message: "Project config cannot add external layer paths. Use user config instead.",
        });
    }
    Ok(())
}
```

Only allowed in project config:
- `ignore_user_layer = true`
- `ignore_additional_layers = true`
```

---

## Action Items

- [ ] Add Task 1.6 to plan-01 (Asset Layer Migration)
- [ ] Add Remote Deploy note to plan-03
- [ ] Add Watch Mode decision to design.md
- [ ] Add Security Validation Task 3.6 to plan-03
- [ ] Add explicit symlink test cases to plan-01 Task 1.2

