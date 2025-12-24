# Multi-Layer PromptPack Implementation Review

> **Date**: 2024-12-24  
> **Purpose**: Self-critical review before implementation

---

## 1. Alignment with Project Principles (CLAUDE.md)

### ✅ Aligned

| Principle | How We Align |
|-----------|--------------|
| **TDD always** | todo.md 中每个任务都要求先写测试，每个 plan 都有 Tests 部分 |
| **No god objects** | design.md 中明确限制每个文件 <400 行，有预估行数表 |
| **Domain purity** | 使用 Ports 模式，domain 层只定义 traits，不依赖 infrastructure |
| **Update docs** | Phase 4 包含文档更新任务 |
| **Minimal changes** | 每个 Phase 只做一件事，不超范围 |
| **Security by default** | plan-00 包含路径验证、符号链接检测 |
| **Never fail silently** | design.md E2 部分明确定义了每种情况的行为 |

### ⚠️ Potential Gaps

| Principle | Gap | Fix |
|-----------|-----|-----|
| **Clean Architecture** | todo.md 2.1 写 `Registry` 在 `infrastructure/repositories/`，但 Entity 应在 `domain/entities/` | 已在 plan-02 修复，明确 Entity 在 domain 层 |
| **Suggest refactoring** | 未评估现有 `DeployUseCase` 的复杂度增加 | 添加到 Open Questions |

---

## 2. PRD vs Plan Consistency Check

### Phase Mapping

| PRD Section | Plan Document | Status |
|-------------|---------------|--------|
| §9 Lockfile Architecture | plan-00-lockfile-migration.md | ✅ Covered |
| §5.1 Layer Resolution | plan-01-core-layer-system.md | ✅ Covered |
| §5.2 Asset Merge Algorithm | plan-01 Task 1.3 | ✅ Covered |
| §11 Global Registry | plan-02-global-registry.md | ✅ Covered |
| §4.3 Configuration Schema | plan-03-config-cli.md | ✅ Covered |
| §10 Output Provenance | plan-04 Task 4.2 | ✅ Covered |
| §12 Interactive UI | plan-02, plan-04 UI sections | ✅ Covered |
| §13 Error Handling | design.md + todo.md Error Handling | ✅ Covered |

### ⚠️ Inconsistencies Found

| Issue | PRD Says | Plan Says | Resolution |
|-------|----------|-----------|------------|
| **Task numbering in plan-02** | N/A | Task 2.3 → 2.4, 2.4 → 2.5 | Renumber correctly |
| **LayerLoader trait location** | N/A | plan-01 says `domain/ports/layer_loader.rs` but todo.md says `infrastructure/` | todo.md needs update |

---

## 3. Missing Items

### 3.1 Not Covered in Plans

| Item | PRD Section | Impact | Action |
|------|-------------|--------|--------|
| **MCP 配置合并** | §15 Q1 | Medium | Add as Open Question, defer to Phase 5 |
| **Remote deploy layer behavior** | §14.3 | Medium | Add note to plan-03 |
| **Watch mode with multi-layer** | §14.4 | Low | Add to Phase 4 or defer |
| **Windows path normalization** | §14.8 | High | Add to plan-00 (lockfile paths) |
| **`calvin check --all`** | §11.4 | Low | Listed in PRD but not in plans |

### 3.2 Missing Tests

| Missing Test | Plan | Action |
|--------------|------|--------|
| Windows path handling | plan-00 | Add to Task 0.5 |
| Layer with symlink path | plan-01 | Add to Task 1.2 |
| Config priority (env vars > config file) | plan-03 | Add to Task 3.1 |

---

## 4. Potential Issues

### 4.1 Architectural Risks

| Risk | Mitigation |
|------|------------|
| `DeployUseCase` already 811 lines, adding layer logic may push it further | Extract layer handling to separate `LayerUseCase`, inject into `DeployUseCase` |
| Registry file lock contention | Already addressed in PRD §14.6 with file locks |
| Lockfile migration during concurrent deploys | Handle in migration logic: check for race conditions |

### 4.2 Edge Cases Not Addressed

| Edge Case | Expected Behavior | Document Location |
|-----------|-------------------|-------------------|
| User layer is a broken symlink | Skip with warning | Add to design.md E2 |
| Project layer config says `ignore_user_layer = true` but CLI says `--layer user` | CLI wins | Add to plan-03 |
| `--source` and `--layer` both specified | `--source` replaces project layer, `--layer` adds additional | Already in PRD §4.4 |

---

## 5. Version/Format Consistency

### Lockfile Format

| Document | Version Format | Consistent? |
|----------|----------------|-------------|
| PRD §9.3 | `version = 1` | ✅ |
| plan-00 | `version = 1` | ✅ |
| design.md D1 | References lockfile migration | ✅ |

### Configuration Format

| Document | Path | Consistent? |
|----------|------|-------------|
| PRD §4.3 | `~/.config/calvin/config.toml` | ✅ |
| PRD §4.3 | User layer: `~/.calvin/.promptpack` | ✅ |
| plan-01 | `~/.calvin/.promptpack` | ✅ |
| plan-03 | `~/.config/calvin/config.toml` | ✅ |

---

## 6. Recommendations

### 6.1 Immediate Fixes (Before Implementation)

1. **Update todo.md Task 2.1**: Change Registry location from `infrastructure/` to `domain/entities/`
2. **Add Windows path handling**: Add explicit task to plan-00
3. **Add `calvin check --all`**: Add to plan-04 or explicitly defer
4. **Renumber plan-02 tasks**: Fix sequential numbering after insertions

### 6.2 Consider Adding

1. **Phase 5**: MCP config merge, remote layer handling
2. **Performance section**: Large layer handling, caching strategy
3. **Rollback section**: What if user wants to go back to single-layer?

### 6.3 Documentation Gaps

| Gap | Action |
|-----|--------|
| Migration guide doesn't cover config migration | Add section |
| No troubleshooting section | Add common issues and fixes |

---

## 7. Checklist for Implementation Start

- [ ] Fix todo.md Registry location
- [ ] Add Windows path handling to plan-00
- [ ] Renumber plan-02 tasks
- [ ] Confirm PRD version is "Draft" → update to "Ready for Implementation"
- [ ] Create `tests/multi_layer/` directory structure plan
- [ ] Identify first test to write (TDD)

---

## 8. Conclusion

**Overall Assessment**: ✅ **Ready to Implement** with minor fixes

The multi-layer design is well-documented and aligns with Calvin's architectural principles. Key strengths:
- Clear layer separation with dependency inversion
- Comprehensive error handling design
- Industry-standard approaches (lockfile location, merge semantics)

Key areas requiring attention:
- Ensure DeployUseCase doesn't become a god object
- Windows path handling needs explicit attention
- todo.md has minor inconsistencies with plan documents

**Recommended First Step**: Start with plan-00 (Lockfile Migration) as it has the fewest dependencies and establishes the foundation for provenance tracking.

