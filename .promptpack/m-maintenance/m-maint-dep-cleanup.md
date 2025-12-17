---
description: Dependency Cleanup
---

# [M-MAINT] Dependency Cleanup

> **Stage**: Maintenance | **Category**: Cleanup | **Frequency**: Periodic

Remove unused and redundant dependencies. Track in `TODO.md`.

## Detection

1. **Unused Dependencies**:
   - Packages in manifest but never imported
   - DevDependencies used in production code
   - Phantom dependencies (used but not declared)
2. **Redundant Dependencies**:
   - Duplicate functionality (lodash + underscore)
   - Superseded packages (moment → dayjs)
   - Framework-included utilities imported separately
3. **Version Issues**:
   - Multiple versions of same package (npm ls --all)
   - Pinned to vulnerable versions
   - Major version behind with security implications

## Tools

```bash
# JavaScript/TypeScript
npx depcheck
npx npm-check
# Python
pip-autoremove
pipreqs --diff
# Go
go mod tidy
go mod why <package>
# Rust
cargo machete
cargo udeps
```

## Output

```text
## Dependency Cleanup Report
### 🗑️ Unused (Safe to Remove)
| Package | Type | Last Used | Size Impact |
|---------|------|-----------|-------------|
| lodash | prod | never imported | -72KB |
### 🔄 Replaceable
| Current | Recommended | Reason |
|---------|-------------|--------|
| moment | dayjs | 97% smaller, maintained |
### ⚠️ Phantom Dependencies
| Package | Used In | Action |
|---------|---------|--------|
| uuid | src/utils.ts | Add to dependencies |
### 📦 Deduplication Opportunities
| Package | Versions | Resolution |
|---------|----------|------------|
| lodash | 4.17.21, 4.17.15 | Upgrade transitive deps |
```
