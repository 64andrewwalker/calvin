---
description: Documentation-Code Synchronization Audit
---

# [0-DISC] Documentation-Code Synchronization Audit

> **Stage**: Discovery | **Category**: Verification | **Frequency**: Pre-release, periodic

Verify documentation accurately reflects implementation reality. Track in `TODO.md`.

## Critical Anti-Patterns to Detect

1. **Phantom Features**: Documented as complete but actually TODO/Mock/Stub
2. **Zombie Documentation**: Describes removed or refactored features
3. **Optimistic Status**: README claims "fully tested" but coverage is 30%
4. **Version Drift**: API docs describe v1 behavior, code is v2

## Audit Process

1. **Feature Inventory Cross-Check**:

   ```text
   For each documented feature:
     → Search codebase for implementation
     → Verify not just stub/mock/placeholder
     → Check test coverage exists
     → Mark status: IMPLEMENTED | PARTIAL | MOCK | MISSING
   ```

2. **Code Reality Scan**:
   - Find all TODO/FIXME/HACK/XXX comments
   - Identify mock implementations (`mock`, `fake`, `stub`, `dummy`)
   - Detect placeholder returns (`return null`, `return []`, `throw NotImplementedError`)
   - Find `console.log("not implemented")` patterns
3. **Status Markers Audit**:
   - README badges vs actual CI status
   - "Coming soon" features timeline check
   - Changelog entries without corresponding code
4. **API Documentation Verification**:
   - Compare OpenAPI/Swagger with actual endpoints
   - Verify request/response schemas match implementation
   - Check documented error codes exist in code

## Output: `docs/sync-audit-report.md`

```text
## Sync Status: X% Aligned
## 🔴 Critical Mismatches (Documented as Done, Actually Not)
| Feature | Doc Location | Code Status | Evidence |
|---------|--------------|-------------|----------|
| OAuth2 | README L45 | MOCK | `return mockToken` in auth.ts:23 |
## 🟡 Partial Implementations
| Feature | Documented | Implemented | Missing |
|---------|------------|-------------|---------|
## 🟢 Verified Complete
## 📝 Documentation Updates Required
- [ ] README.md: Change "Full OAuth2" → "OAuth2 (in progress)"
- [ ] API.md: Remove endpoint /api/v2/export (not implemented)
## 🗑️ Dead Documentation (No Corresponding Code)
## 💀 Undocumented Features (Code Exists, No Docs)
```

## Auto-Fix Actions

1. Update status badges to reflect reality
2. Add "WIP"/"Planned" labels to incomplete features
3. Generate stubs for undocumented features
4. Create tracking issues for mismatches

## Prevention

- Add CI check: doc-code sync verification
- Pre-commit hook for README status claims
