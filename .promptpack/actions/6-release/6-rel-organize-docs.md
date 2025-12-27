---
description: Documentation Organization
---

# [6-REL] Documentation Organization

> **Stage**: Release | **Category**: Documentation | **Frequency**: Periodic

Audit and restructure project documentation. Track progress in `TODO.md`.

## Naming Conventions

- UPPERCASE: `README.md`, `LICENSE`, `CHANGELOG.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`
- lowercase-dash: All other docs (e.g., `api-reference.md`, `setup-guide.md`)

## Tasks

1. **Inventory**: List all documentation including inline long comments, `/docs`, wiki, and docstrings
2. **Audit**: Identify duplicates, outdated content, orphaned files, and gaps
3. **Consolidate**: Merge short docs (<100 lines) into relevant parent documents
4. **Rename**: Use `mv` command for renaming; update all references and indexes
5. **Restructure**: Organize into logical hierarchy:

   ```text
   docs/
   ├── getting-started.md
   ├── architecture.md
   ├── api/
   ├── guides/
   └── contributing/
   ```

6. **Index Update**: Ensure `README.md` and any `index.md` reflect current structure

## Output

- Updated documentation tree
- Report of changes in `docs/reorganization-log.md`
