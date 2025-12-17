---
description: Error Handling
---

# [3-DEV] Error Handling Standardization

> **Stage**: Development | **Category**: Code Quality | **Frequency**: Standardization

Implement consistent error handling patterns. Track in `TODO.md`.

## Error Taxonomy

1. **Define Error Categories**:
   - Operational errors (expected, recoverable)
   - Programming errors (bugs, unexpected)
   - External errors (network, third-party)
2. **Error Code System**:

   ```text
   E[CATEGORY][MODULE][SEQUENCE]
   Example: EAUTH001 - Invalid credentials
   ```

## Implementation Pattern

1. **Custom Error Classes**:

   ```typescript
   class AppError extends Error {
     code: string;
     statusCode: number;
     isOperational: boolean;
     context?: Record<string, unknown>;
   }
   ```

2. **Error Factory**:
   - Centralized error creation
   - Consistent metadata attachment
   - Stack trace preservation

## Error Handling Layers

1. **Controller/Handler Level**: User-facing error responses
2. **Service Level**: Business logic error wrapping
3. **Repository Level**: Data access error translation
4. **Global Handler**: Uncaught exception safety net

## Response Format

```json
{
  "error": {
    "code": "EAUTH001",
    "message": "User-friendly message",
    "details": [],
    "traceId": "uuid"
  }
}
```

## Output

- `src/errors/` - Error class hierarchy
- `src/middleware/error-handler.ts` - Global handler
- `docs/error-codes.md` - Error catalog
- Updated API documentation with error responses
