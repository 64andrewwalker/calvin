---
description: Bug Fix Testing
---

# [4-TEST] Test Fix & Verification
>
> **Stage**: Testing | **Category**: Test Execution | **Frequency**: After code changes
Run all tests, analyze failures, apply fixes. Track in `TODO.md`.

## Process

1. **Run Full Suite**:
   - Execute all tests
   - Collect failures with stack traces
   - Record execution time and coverage
2. **Categorize Each Failure**:
   - **CODE_BUG**: Implementation doesn't match expected behavior
   - **TEST_BUG**: Test assertion incorrect or outdated
   - **FLAKY**: Non-deterministic, timing/order dependent
   - **ENV**: Missing dependencies, config, or setup issue
3. **Fix Priority Order**:
   - CODE_BUG > TEST_BUG > ENV > FLAKY
   - If >10 failures: prioritize by dependency order (fix foundational first)
4. **Fix Strategy**:
   - **For CODE_BUG**: Fix implementation, preserve test as spec
   - **For TEST_BUG**: Update test to match intended behavior (confirm with docs/specs first)
   - **For FLAKY**: Add retries, fix timing, or isolate properly
   - **For ENV**: Fix configuration, add missing setup, document requirements
5. **Verification**:
   - Re-run after each fix to verify
   - Catch cascading failures early
   - Run full suite after all fixes

## Output

- Test run report with failure categorization
- Fixes applied (code changes + test updates)
- `TODO.md` for remaining issues
- Updated test coverage metrics
