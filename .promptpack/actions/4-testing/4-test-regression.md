---
description: Regression Testing
---

# [4-TEST] Regression Testing
>
> **Stage**: Testing | **Category**: Test Execution | **Frequency**: After changes, pre-release
Run previously failing or affected tests only. Track in `TODO.md`.

## Process

1. **Identify Affected Tests**:
   - Changed files (git diff)
   - Explicit test list provided
   - Related tests (same module/feature)
   - Previously failing tests from history
2. **Run Targeted Subset**:
   - Execute only identified tests
   - Faster feedback loop
   - Focus on changed areas
3. **On Failure**:
   - Apply 4-test-fix process
   - Categorize failure type
   - Fix in priority order
4. **Expand Coverage**:
   - Run related tests (same module)
   - Catch ripple effects
   - Verify no new regressions
5. **Full Suite**:
   - Only after targeted fixes pass
   - Final verification
   - Ensure no unexpected breakage

## Use Cases

- After refactoring specific module
- Before merging feature branch
- After dependency updates
- Quick verification after small changes

## Output

- Regression report with:
  - Tests run
  - Failures found
  - Fixes applied
  - Related tests status
- Updated test status
