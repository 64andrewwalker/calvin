---
description: Code Refactoring
---

# [3-DEV] Refactoring Implementation

> **Stage**: Development | **Category**: Code Quality | **Frequency**: As needed

Execute code refactoring systematically. Track in `TODO.md`.

## Pre-conditions Check

- [ ] Refactoring plan exists? → If NO: Run analysis first (see below)
- [ ] Tests exist? → If NO: Warn user, proceed with caution

## If No Plan Provided

1. Analyze codebase for:
   - Code smells (duplication, long methods, feature envy)
   - Performance anti-patterns
   - Security vulnerabilities
   - Maintainability issues
2. Generate `docs/refactoring-plan.md` with prioritized tasks
3. Await user approval before proceeding

## Execution Protocol

1. **Atomic Changes**: One refactoring per commit
2. **Test After Each Change**: Run existing tests; fix if broken
3. **Preserve Behavior**: No functional changes unless explicitly required
4. **Document**: Update inline comments and docs as needed

## Best Practices

- Extract before refactoring complex methods
- Prefer composition over inheritance
- Apply early returns to reduce nesting
- Use meaningful names reflecting intent
- Consider performance implications

## Commit Format

```text
refactor(<scope>): <description>
- What changed
- Why it's better
```
