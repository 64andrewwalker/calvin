---
description: Test-Driven Development
---

# [4-TEST] Test-Driven Development
>
> **Stage**: Testing | **Category**: TDD Workflow | **Frequency**: Feature development
Test-driven development mode. Input: feature spec or failing test.

## Process

1. **RED Phase**:
   - If no test: Write minimal failing test for feature
   - Test should describe desired behavior clearly
   - Run test (must fail - RED)
2. **GREEN Phase**:
   - Write minimal code to pass test
   - No premature optimization
   - Focus on making test pass, not perfect code
3. **REFACTOR Phase**:
   - Clean up code while keeping tests green
   - Improve structure, naming, performance
   - Ensure all tests still pass
4. **Repeat**:
   - Move to next requirement
   - Write next failing test
   - Continue RED-GREEN-REFACTOR cycle

## TDD Principles

- Write tests first (specification)
- One assertion per test (logical grouping allowed)
- Test behavior, not implementation
- Keep tests fast and isolated

## Output

- Passing tests + implementation
- `docs/tdd-session-<feature>.md` log with:
  - Requirements covered
  - Tests written
  - Implementation decisions
  - Refactoring notes
