---
description: Performance Profiling
---

# [5-QA] Performance Profiling

> **Stage**: Quality Assurance | **Category**: Performance | **Frequency**: Performance issues

Identify and resolve performance bottlenecks. Track in `TODO.md`.

## Analysis

1. **Hotspot Detection**: CPU-intensive functions
2. **Memory Profiling**: Leaks, excessive allocation
3. **I/O Analysis**: Database queries, network calls, file operations
4. **Async Audit**: Blocking operations, concurrency issues

## Tools

- Language-specific profilers
- Flame graphs generation
- Query analyzers (EXPLAIN)

## Output

- `docs/performance-report.md` with metrics
- Optimization recommendations with expected impact
- Before/after benchmarks for implemented fixes
