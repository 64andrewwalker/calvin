---
description: Worktree Review
---

# [X-UTIL] Multi-Agent Worktree Review & Merge

> **Stage**: Cross-Stage Utility | **Category**: Multi-Agent | **Frequency**: After multi-agent sessions

Review parallel agent worktrees, cherry-pick best implementations, consolidate. Track in `TODO.md`.

## Context

When using Cursor multi-agent mode, each agent works in isolated worktrees. This command provides intelligent review and consolidation.

## Discovery Phase

1. **List All Worktrees**:

   ```bash
   git worktree list
   ```

2. **For Each Worktree, Collect**:
   - Branch name and purpose
   - Commit history since branch point
   - Files modified
   - Test results (if CI ran)
   - Agent notes/comments (if any)

## Comparative Analysis

1. **Diff Matrix Generation**:

   ```text
   | File | main | agent-1 | agent-2 | agent-3 |
   |------|------|---------|---------|---------|
   | auth.ts | base | +OAuth | +OAuth+MFA | +JWT |
   | db.ts | base | unchanged | +caching | +pooling |
   ```

2. **Quality Scoring Per Change**:
   - Code complexity delta (cyclomatic)
   - Test coverage change
   - Type safety (if TS/typed language)
   - Performance implications
   - Security considerations
   - Consistency with project patterns
3. **Conflict Detection**:
   - Semantic conflicts (same function, different logic)
   - Structural conflicts (file reorganization)
   - Dependency conflicts (different package versions)

## Decision Framework

For each modified file/feature:

```text
1. If one clear winner → Select that version
2. If complementary changes → Merge both (e.g., caching + pooling)
3. If conflicting approaches → Analyze trade-offs, recommend one
4. If all poor quality → Reject all, note improvement needs
```

## Cherry-Pick Strategy

```text
## Recommended Merge Plan
### Phase 1: Non-Conflicting Wins
| Source | Files | Reason |
|--------|-------|--------|
| agent-2/feature | auth.ts, auth.test.ts | Best test coverage, includes MFA |
| agent-3/feature | db.ts | Connection pooling improves perf |
### Phase 2: Manual Merge Required
| File | agent-1 | agent-2 | Recommendation |
|------|---------|---------|----------------|
| api/routes.ts | REST style | GraphQL | Discuss with team |
### Phase 3: Reject
| Source | Files | Reason |
|--------|-------|--------|
| agent-1/feature | utils.ts | Introduces circular dependency |
```

## Execution Commands

```bash
# Cherry-pick selected commits
git cherry-pick <commit-hash>
# Or cherry-pick with custom merge
git checkout main
git checkout agent-2/feature -- path/to/file.ts
# After consolidation, cleanup worktrees
git worktree remove agent-1/feature
git worktree remove agent-2/feature
git worktree prune
# Delete remote branches if pushed
git push origin --delete agent-1/feature
```

## Output

- `docs/worktree-review-<date>.md` - Decision log with rationale
- Consolidated branch ready for PR
- Cleanup script for worktree removal
- List of rejected changes with improvement notes for future reference

## Post-Merge Verification

1. Run full test suite on consolidated branch
2. Verify no regressions from rejected changes
3. Check for accidentally dropped features
4. Performance benchmark if applicable
