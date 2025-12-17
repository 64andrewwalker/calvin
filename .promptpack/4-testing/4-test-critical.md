---
description: Critical Path Testing
---

# [4-TEST] Critical Tests Generation

> **Stage**: Testing | **Category**: Test Creation | **Frequency**: Core features

Design unit tests for core functionality only. Track in `TODO.md`.

## Scope Definition

1. Identify core modules from:
   - Project documentation
   - Entry points and main workflows
   - Business-critical paths
2. Include tests for PLANNED but unimplemented features (from specs)

## Test Design Principles

- **Arrange-Act-Assert** pattern
- **One assertion focus** per test (logical grouping allowed)
- **Descriptive names**: `should_<expected>_when_<condition>`
- **No edge cases**: Happy path and primary failure modes only
- **Isolated**: No external dependencies; mock aggressively

## Output Structure

```text
tests/
├── critical/
│   ├── <module>_test.<ext>
│   └── ...
└── fixtures/
```

## Test Quality Requirements

- Clear failure messages
- Fast execution (<100ms per test)
- Deterministic (no flaky tests)
- Self-documenting

## Purpose

These tests serve as:

1. Specification for core behavior
2. Template for full test suite
3. Development guidance (TDD anchor)
