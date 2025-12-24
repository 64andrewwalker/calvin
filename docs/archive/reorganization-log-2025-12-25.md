# Documentation Reorganization Log

> **Date**: 2025-12-25
> **Reason**: Multi-layer feature completed, consolidate archives

## Changes Made

### 1. Multi-Layer Implementation Docs → Archive

The multi-layer promptpack feature is now complete. All implementation plans and design
documents have been moved to `docs/archive/multi-layer-implementation/`:

**Moved files:**

- `plan-00-lockfile-migration.md`
- `plan-01-core-layer-system.md`
- `plan-02-global-registry.md`
- `plan-03-config-cli.md`
- `plan-04-visibility-tooling.md`
- `design.md`
- `todo.md`
- `HANDOFF.md`
- `review-audit.md`
- `review-deep-check.md`
- `multi-layer-promptpack-prd.md`
- `multi-layer-migration-guide.md`

### 2. Proposals Archive Consolidated

Moved `docs/proposals/archive/` to `docs/archive/proposals-archive/`

### 3. Reports Archived

Moved `docs/reports/product-issues-2025-12-21.md` to `docs/reports/archive/`

### 4. Empty Directories Removed

- `docs/proposals/multi-layer/` (now empty)
- `docs/proposals/` (now empty)

## Current Structure

```
docs/
├── architecture/       # Architecture documentation
├── archive/            # Historical documents
│   ├── design/         # Old design documents
│   ├── multi-layer-implementation/  # Multi-layer feature docs
│   └── proposals-archive/           # Old proposals
├── reports/            # Audit and review reports
│   └── archive/        # Historical reports
├── analysis-report.md
├── architecture.md
├── cli-state-machine.md
├── command-reference.md
├── configuration.md
├── implementation-plan.md
├── target-platforms.md
├── tech-decisions.md
├── ui-components-spec.md
└── ux-review.md
```

## User-Facing Documentation

User documentation is in `docs-content/` for the Fumadocs site:

```
docs-content/docs/
├── api/
│   ├── changelog.mdx
│   ├── clean.mdx
│   ├── frontmatter.mdx
│   ├── index.mdx
│   ├── library.mdx
│   └── versioning.mdx
└── guides/
    ├── configuration.mdx
    ├── multi-layer.mdx       # NEW
    ├── clean-command.mdx
    ├── scope-guide.mdx
    └── ...
```

## Notes

- Multi-layer feature is now fully documented in user-facing `multi-layer.mdx`
- Internal implementation docs preserved in archive for historical reference
- All proposals completed and archived
