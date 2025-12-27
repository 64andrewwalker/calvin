---
description: Observability Setup
---

# [2-INIT] Logging & Observability Setup

> **Stage**: Initialize | **Category**: Monitoring | **Frequency**: Project setup

Implement comprehensive observability stack. Track in `TODO.md`.

## Logging Standards

1. **Log Levels**: Define usage for DEBUG, INFO, WARN, ERROR, FATAL
2. **Structured Logging**: JSON format with consistent fields

   ```json
   {
     "timestamp": "ISO8601",
     "level": "INFO",
     "service": "api",
     "traceId": "uuid",
     "message": "...",
     "context": {}
   }
   ```

3. **Sensitive Data**: PII redaction, credential masking
4. **Correlation**: Request ID propagation across services

## Metrics Implementation

1. **Application Metrics**:
   - Request rate, latency, error rate (RED)
   - Saturation indicators (queue depth, connection pool)
   - Business metrics (signups, transactions)
2. **System Metrics**:
   - CPU, memory, disk, network
   - GC statistics
   - Event loop lag (Node.js)

## Tracing Setup

1. **Distributed Tracing**:
   - Span creation for key operations
   - Context propagation headers
   - Sampling strategy
2. **Integration Points**:
   - HTTP clients/servers
   - Database queries
   - External API calls
   - Message queue operations

## Output

- `src/lib/logger.ts` - Configured logger
- `src/lib/metrics.ts` - Metrics utilities
- `src/middleware/tracing.ts` - Tracing middleware
- `docs/observability.md` - Standards and usage guide
- Dashboard templates (Grafana JSON if applicable)
