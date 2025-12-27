---
description: Legacy Code Modernization
---

# [3-DEV] Legacy Code Modernization

> **Stage**: Development | **Category**: Modernization | **Frequency**: Legacy projects

Systematically modernize legacy codebase. Track in `TODO.md`.

## Assessment Phase

1. **Code Archaeology**:
   - Identify oldest/most outdated patterns
   - Map deprecated API usage
   - Document tribal knowledge
2. **Dependency Audit**:
   - EOL libraries
   - Security vulnerabilities
   - Upgrade path complexity
3. **Technical Debt Inventory**:
   - Code duplication hotspots
   - Missing tests coverage map
   - Documentation gaps

## Modernization Priorities

1. **Critical Path First**:
   - Most frequently modified files
   - Highest bug density areas
   - Performance bottlenecks
2. **Low-Risk Quick Wins**:
   - Syntax modernization (ES6+, modern Python, etc.)
   - Linter/formatter introduction
   - Type annotation addition

## Execution Strategy

```text
Phase 1: Stabilization
  - Add tests to critical paths (golden master tests)
  - Set up CI/CD if missing
  - Establish coding standards
Phase 2: Infrastructure
  - Update build tooling
  - Upgrade runtime version
  - Dependency updates (security first)
Phase 3: Code Modernization
  - Pattern by pattern replacement
  - API migration with deprecation warnings
  - Progressive type safety introduction
Phase 4: Architecture Evolution
  - Extract modules for future flexibility
  - Introduce abstraction layers
  - Prepare for potential rewrites
```

## Output

- `docs/modernization-roadmap.md`
- `docs/deprecated-patterns.md` (migration guides)
- Golden master test suite
- Automated migration scripts where possible
