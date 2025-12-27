---
description: Monorepo Setup
---

# [2-INIT] Monorepo Setup & Management

> **Stage**: Initialize | **Category**: Infrastructure | **Frequency**: Project setup

Configure or optimize monorepo structure. Track in `TODO.md`.

## Tool Selection

| Tool | Best For |
|------|----------|
| Turborepo | JS/TS, simple setup |
| Nx | Large teams, advanced caching |
| Lerna | npm publishing focus |
| pnpm workspaces | Disk efficient, strict |
| Bazel | Polyglot, massive scale |

## Structure Design

```text
/
├── apps/
│   ├── web/
│   ├── api/
│   └── mobile/
├── packages/
│   ├── ui/           # Shared components
│   ├── utils/        # Common utilities
│   ├── config/       # Shared configs
│   └── types/        # Shared TypeScript types
├── tools/            # Build scripts, generators
├── turbo.json        # Pipeline configuration
└── package.json      # Root workspace config
```

## Configuration

1. **Dependency Management**:
   - Shared dependencies hoisting
   - Per-package version overrides
   - Internal package linking
2. **Build Pipeline**:
   - Task dependencies graph
   - Caching strategy (local + remote)
   - Parallel execution configuration
3. **CI Optimization**:
   - Affected package detection
   - Incremental builds
   - Test splitting

## Output

- Configured monorepo tooling
- Package templates/generators
- `docs/monorepo-guide.md`
- CI/CD pipeline with caching
- Dependency graph visualization
