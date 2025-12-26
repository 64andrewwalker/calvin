# JSON Output Consistency Plan

> **Created**: 2025-12-26
> **Branch**: `fix/json-output-consistency`
> **Status**: Planning

## Problem Statement

The CLI's JSON output format is inconsistent across commands:

1. **Field naming**: Mix of `"event"` and `"type"` for event classification
2. **Construction method**: Mix of manual string interpolation and serde
3. **Event structure**: No standard envelope pattern

This makes it harder for CI/CD pipelines and tools to parse Calvin's output reliably.

## Goals

1. Standardize all JSON output to use consistent field names
2. Replace all manual JSON construction with serde
3. Define a clear NDJSON event protocol for all commands
4. Maintain backward compatibility where possible

## Non-Goals

- Adding new output formats (e.g., YAML, table)
- Changing the text output format
- Breaking existing JSON consumers (use deprecation period)

## Design

### Event Protocol

All JSON output should follow this NDJSON (newline-delimited JSON) pattern:

```json
{"event": "start", "command": "CMD", "version": "0.5.1", "timestamp": "..."}
{"event": "progress", "command": "CMD", ...progress_data...}
{"event": "data", "command": "CMD", ...actual_data...}
{"event": "complete", "command": "CMD", "success": true, "duration_ms": 123}
```

For error cases:
```json
{"event": "error", "command": "CMD", "code": "ERROR_CODE", "message": "...", "help": "..."}
```

### Event Types

| Event | Purpose |
|-------|---------|
| `start` | Command started |
| `progress` | Progress update (file written, etc.) |
| `data` | Main data payload |
| `warning` | Non-fatal issue |
| `error` | Fatal error |
| `complete` | Command finished |

### Shared Event Structures

Create `src/ui/json/events.rs`:

```rust
use serde::Serialize;

#[derive(Serialize)]
pub struct StartEvent {
    pub event: &'static str, // "start"
    pub command: &'static str,
    pub version: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

#[derive(Serialize)]
pub struct CompleteEvent {
    pub event: &'static str, // "complete"
    pub command: &'static str,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

#[derive(Serialize)]
pub struct ErrorEvent {
    pub event: &'static str, // "error"
    pub command: &'static str,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}
```

## Implementation Plan

### Phase 1: Infrastructure (No Breaking Changes)

- [ ] **1.1** Create `src/ui/json/events.rs` with shared event types
- [ ] **1.2** Create `src/ui/json/mod.rs` to organize JSON utilities
- [ ] **1.3** Add `emit_event()` helper function
- [ ] **1.4** Add tests for event serialization

### Phase 2: Migrate Commands (Backward Compatible)

For each command, add new `"event"` field while keeping `"type"` for compatibility:

- [ ] **2.1** `clean.rs` - Replace manual string construction with serde
- [ ] **2.2** `projects.rs` - Add `"event"` alongside `"type"`
- [ ] **2.3** `layers.rs` - Wrap output in event envelope
- [ ] **2.4** `provenance.rs` - Wrap output in event envelope
- [ ] **2.5** `deploy/cmd.rs` - Review and standardize
- [ ] **2.6** `check/engine.rs` - Review and standardize
- [ ] **2.7** `interactive/mod.rs` - Already uses `"event"`, verify consistency
- [ ] **2.8** `explain.rs` - Already uses `"event"`, verify consistency

### Phase 3: Documentation

- [ ] **3.1** Add JSON output schema to `docs/api/json-output.md`
- [ ] **3.2** Add migration guide for existing consumers
- [ ] **3.3** Update command-reference.md with JSON examples

### Phase 4: Deprecation (v0.6+)

- [ ] **4.1** Add deprecation warning for `"type"` field
- [ ] **4.2** Plan removal of `"type"` field for v1.0

## Migration Strategy

### For `clean.rs` (Highest Priority)

Current:
```rust
println!(r#"{{"type":"clean_start","scope":"{}","file_count":{}}}"#, scope, count);
```

After:
```rust
emit_event(&CleanStartEvent {
    event: "start",
    command: "clean",
    scope: scope.to_string(),
    file_count: count,
})?;
```

### Backward Compatibility

During transition, emit both fields:
```json
{
  "event": "start",
  "type": "clean_start",  // deprecated
  "command": "clean",
  ...
}
```

## Testing Strategy

1. **Unit tests**: Each event type serializes correctly
2. **Integration tests**: JSON output can be parsed by jq
3. **Snapshot tests**: JSON output format is stable
4. **Property tests**: No invalid JSON is ever produced

## Rollout Plan

1. Merge Phase 1 (infrastructure) - No user impact
2. Merge Phase 2 (migration) - Backward compatible
3. Release v0.6.0 with deprecation warnings
4. Remove `"type"` field in v1.0

## Files to Modify

| File | Changes |
|------|---------|
| `src/ui/json.rs` | Restructure into module |
| `src/ui/json/events.rs` | New: shared event types |
| `src/ui/json/mod.rs` | New: module organization |
| `src/commands/clean.rs` | Replace manual JSON |
| `src/commands/projects.rs` | Add event wrapper |
| `src/commands/layers.rs` | Add event wrapper |
| `src/commands/provenance.rs` | Add event wrapper |
| `docs/api/json-output.md` | New: JSON schema docs |

## Success Criteria

- [ ] All JSON output uses serde (no manual string construction)
- [ ] All JSON output has consistent `"event"` and `"command"` fields
- [ ] jq can reliably filter events by type
- [ ] Existing CI pipelines continue to work (backward compatibility)
- [ ] JSON schema is documented for integrators

## Risks

| Risk | Mitigation |
|------|------------|
| Breaking existing consumers | Emit both `"event"` and `"type"` during transition |
| Performance regression from serde | Benchmark before/after |
| Scope creep | Strict phase boundaries |

## Timeline Estimate

- Phase 1: 1-2 hours
- Phase 2: 2-3 hours
- Phase 3: 1 hour
- Phase 4: Future release

Total: ~5 hours for v0.6.0 compatible changes

