---
description: Database Optimization
---

# [1-PLAN] Database Optimization

> **Stage**: Planning | **Category**: Performance | **Frequency**: Performance issues

Comprehensive database performance analysis and optimization. Track in `TODO.md`.

## Analysis Phase

1. **Schema Review**:
   - Normalization level assessment (under/over-normalized)
   - Data type appropriateness (e.g., VARCHAR vs TEXT, INT vs BIGINT)
   - Nullable columns audit
   - Default values and constraints
2. **Index Analysis**:
   - Missing indexes on frequently queried columns
   - Unused indexes consuming write overhead
   - Composite index ordering optimization
   - Covering index opportunities
3. **Query Audit**:
   - Collect slow queries from logs/monitoring
   - Run EXPLAIN/EXPLAIN ANALYZE on critical queries
   - Identify N+1 query patterns
   - Full table scans detection
4. **Relationship Review**:
   - Foreign key integrity
   - Cascade behavior appropriateness
   - Orphaned records detection

## Optimization Actions

1. **Index Optimization**:
   - Create missing indexes with impact estimation
   - Drop unused indexes
   - Reorder composite indexes
2. **Query Rewriting**:
   - Convert subqueries to JOINs where beneficial
   - Add pagination to unbounded queries
   - Implement query result caching strategy
3. **Schema Adjustments**:
   - Denormalization for read-heavy tables
   - Partitioning strategy for large tables
   - Archive strategy for historical data

## Tools

- Database-specific analyzers (pg_stat_statements, MySQL slow query log)
- EXPLAIN visualization tools
- Index usage statistics

## Output

Generate `docs/database-optimization-report.md`:

- Current performance baseline metrics
- Schema diagram (Mermaid ERD)
- Identified issues with severity ranking
- Optimization recommendations with expected impact
- Migration scripts in `migrations/` directory
- Rollback procedures for each change

## Safety Protocol

- Backup verification before schema changes
- Test migrations on staging first
- Monitor performance after each change
- Keep rollback scripts ready
