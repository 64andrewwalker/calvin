---
description: Full Test Suite
---

# [4-TEST] Full Test Suite Generation

> **Stage**: Testing | **Category**: Test Creation | **Frequency**: Comprehensive coverage

Generate comprehensive tests based on critical tests template. Track in `TODO.md`.

## Pre-condition

- [ ] Critical tests exist? → If NO: STOP. Output "Run `critical-tests` first."

## Coverage Targets

- Line coverage: >80%
- Branch coverage: >75%
- Critical paths: 100%

## Test Categories

1. **Unit Tests**: All public methods and functions
2. **Edge Cases**: Boundary values, empty inputs, max limits
3. **Error Handling**: Invalid inputs, null/undefined, type errors
4. **Integration Points**: Module boundaries, API contracts
5. **Security Tests**:
   - Input validation (injection, XSS vectors)
   - Authentication/authorization boundaries
   - Sensitive data handling

## Quality Standards

- Mirror critical tests' style and structure
- Group by feature/module
- Include setup/teardown where needed
- Document non-obvious test rationale

## Output

- `tests/unit/` - Unit tests
- `tests/integration/` - Integration tests
- `tests/security/` - Security-focused tests
- Coverage report generation command in `README.md`
