---
description: Test Variants
---

# [4-TEST] Test Variant Generation

> **Stage**: Testing | **Category**: Test Creation | **Frequency**: Edge case coverage

Generate edge case variants for existing tests. Track in `TODO.md`.

## For Each Existing Test, Generate Variants For

### Input Boundaries

- Empty/null/undefined inputs
- Minimum and maximum valid values
- Off-by-one boundaries
- Type coercion edge cases

### State Variations

- Uninitialized state
- Concurrent access scenarios
- Resource exhaustion conditions
- Timeout and retry scenarios

### Error Injection

- Network failures (if applicable)
- Malformed data
- Permission denied scenarios
- Partial failures

## Naming Convention

```text
<original_test_name>__<variant_type>
Example: should_save_user__with_empty_name
```

## Anti-Overfitting Measures

- Randomized valid inputs where appropriate
- Property-based test candidates identification
- Mutation testing recommendations

## Output

- Variant tests alongside originals
- `docs/test-variants-rationale.md` explaining coverage improvements
