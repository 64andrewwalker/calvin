---
description: Debug Exception
---

# [X-UTIL] Exception Analysis & Resolution

> **Stage**: Cross-Stage Utility | **Category**: Debugging | **Frequency**: Event-driven

Diagnose and fix provided error/bug. Track in `TODO.md`.

## Input Required

- Error message/stack trace
- Reproduction steps (if available)
- Environment context

## Diagnostic Process

1. **Stack Trace Analysis**: Map to source code locations
2. **Call Chain Tracing**: Identify data flow leading to exception
3. **Hypothesis Formation**: List probable causes ranked by likelihood
4. **Tool-Assisted Analysis**:
   - Use debugger breakpoints
   - Add diagnostic logging
   - Network inspection (curl, tcpdump) if relevant
   - Memory/resource profiling if suspected

## Resolution Protocol

1. **Root Cause Confirmation**: Verify hypothesis with minimal reproduction
2. **Fix Implementation**:
   - Smallest change that resolves issue
   - Add defensive checks where appropriate
   - No unrelated changes
3. **Regression Test**: Create test that would have caught this bug
4. **Verification**: Run full test suite

## Output

- `docs/incident-reports/<date>-<issue>.md`:
  - Symptoms
  - Root cause
  - Fix applied
  - Prevention measures
