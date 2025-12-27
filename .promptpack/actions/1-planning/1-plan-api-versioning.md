---
description: API Versioning Strategy
---

# [1-PLAN] API Versioning Strategy

> **Stage**: Planning | **Category**: API Design | **Frequency**: API evolution

Implement or restructure API versioning. Track in `TODO.md`.

## Current State Analysis

1. **Breaking Changes History**: Document past breaking changes
2. **Consumer Inventory**: Known API consumers and their versions
3. **Deprecation Debt**: Currently deprecated but not removed

## Versioning Strategy Selection

| Strategy | Use Case |
|----------|----------|
| URL Path (`/v1/`) | Public APIs, clear separation |
| Header (`Accept-Version`) | Internal APIs, cleaner URLs |
| Query Param (`?version=1`) | Simple, easy debugging |

## Implementation Plan

1. **Version Structure**:

   ```text
   src/api/
   ├── v1/
   │   ├── routes/
   │   ├── controllers/
   │   └── schemas/
   ├── v2/
   └── shared/  # Cross-version utilities
   ```

2. **Deprecation Policy**:
   - Sunset header implementation
   - Deprecation timeline (e.g., N-2 versions supported)
   - Migration guide per version bump
3. **Documentation**:
   - Per-version OpenAPI specs
   - Changelog with breaking changes highlighted
   - Migration guides

## Output

- Restructured API with versioning
- `docs/api-versioning-policy.md`
- `docs/api-changelog.md`
- Deprecation warning middleware
- Consumer notification templates
