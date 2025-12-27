---
description: Changelog Generation
---

# [6-REL] Changelog Generation

> **Stage**: Release | **Category**: Documentation | **Frequency**: Pre-release

Generate accurate changelog from commits and code changes. Track in `TODO.md`.

## Why Separate Command

Prevents the common issue of AI generating changelogs that describe intended rather than actual changes.

## Process

1. **Commit Analysis**:
   - Parse commits since last release tag
   - Categorize by conventional commit type
   - Extract scope and description
2. **Code Diff Verification**:

   ```text
   For each commit claiming a feature/fix:
     → Verify code change exists
     → Confirm tests added/updated
     → Check documentation updated
     → Mark as VERIFIED or UNVERIFIED
   ```

3. **Breaking Change Detection**:
   - API signature changes
   - Removed public methods
   - Changed default behaviors
   - Database schema changes

## Output Format (Keep a Changelog)

```text
# Changelog
## [Unreleased]
### Added
- OAuth2 authentication support (#123) ✓ verified
### Changed
- Improved database connection pooling (#125) ✓ verified
### Deprecated
- Legacy auth endpoint `/api/v1/login` (#124)
### Removed
- Unused analytics module (#126) ✓ verified
### Fixed
- Race condition in queue processor (#127) ✓ verified
### Security
- Updated dependencies with known CVEs (#128)
---
### Unverified Claims (Review Required)
- #129 claims "Added caching" but no cache implementation found
```

## Automation Hook

- Pre-release CI step
- Block release if unverified claims exist
