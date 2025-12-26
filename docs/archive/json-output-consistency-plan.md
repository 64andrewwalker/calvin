# JSON Output Consistency Plan

> **Created**: 2025-12-26
> **Branch**: `fix/json-output-consistency`
> **Status**: Phase 3 Complete (Documentation)

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

---

## TODO Tracker

### Phase 1: Infrastructure (No Breaking Changes)

| ID | Task | Status | File(s) | Acceptance Criteria |
|----|------|--------|---------|---------------------|
| 1.1 | Create `events.rs` with shared event types | ✅ Done | `src/ui/json/events.rs` | `StartEvent`, `CompleteEvent`, `ErrorEvent`, `ProgressEvent`, `DataEvent` structs with `#[derive(Serialize)]` |
| 1.2 | Reorganize `json.rs` into module | ✅ Done | `src/ui/json/mod.rs` | Existing `emit()` function works unchanged |
| 1.3 | Add `emit_event()` helper | ✅ Done | `src/ui/json/mod.rs` | Generic function that serializes any `Serialize` and prints with newline |
| 1.4 | Add unit tests for events | ✅ Done | `src/ui/json/events.rs`, `src/ui/json/mod.rs` | 11 tests covering all event types and helpers |

### Phase 2: Migrate Commands (Backward Compatible)

| ID | Task | Status | File | Current Issue | Change |
|----|------|--------|------|---------------|--------|
| 2.1 | Fix `clean.rs` manual JSON | ✅ Done | `src/commands/clean.rs` | Was using `println!(r#"..."#)` | Replaced with `emit_event()` and serde structs |
| 2.2 | Standardize `projects.rs` | ✅ Done | `src/commands/projects.rs` | Was using `"type": "projects"` | Added `"event": "data"`, `"command": "projects"` alongside `"type"` |
| 2.3 | Wrap `layers.rs` output | ✅ Done | `src/commands/layers.rs` | Was direct `serde_json::to_string()` | Wrapped in `{"event": "data", "command": "layers", ...}` |
| 2.4 | Wrap `provenance.rs` output | ✅ Done | `src/application/provenance.rs` | Was using `"type": "provenance"` | Added `"event": "data"`, `"command": "provenance"` |
| 2.5 | Review `deploy/cmd.rs` | ✅ Done | `src/infrastructure/events/json.rs` | Already uses correct format | No changes needed (uses JsonEventSink) |
| 2.6 | Review `check/engine.rs` | ✅ Done | `src/commands/check/engine.rs` | Already uses correct format | No changes needed |
| 2.7 | Standardize `interactive/mod.rs` | ✅ Done | `src/commands/interactive/mod.rs` | Was using `"event": "interactive"` | Changed to `"event": "data"`, `"command": "interactive"` |
| 2.8 | Verify `explain.rs` | ✅ Done | `src/commands/explain.rs` | Already uses correct format | No changes needed |
| 2.9 | Standardize `init.rs` | ✅ Done | `src/commands/init.rs` | Was missing `"command"` | Added `"command": "init"` to all events |
| 2.10 | Standardize `watch` events | ✅ Done | `src/application/watch/event.rs` | Was missing `"command"` | Added `"command": "watch"` via `to_json()` |

### Phase 3: Documentation

| ID | Task | Status | File | Content |
|----|------|--------|------|---------|
| 3.1 | Document JSON schema | ✅ Done | `docs/json-output.md` | Event types, field descriptions, examples |
| 3.2 | Migration guide | ✅ Done | `docs/json-output.md` | Included in "Backward Compatibility" section |
| 3.3 | Update command reference | ✅ Done | `docs/command-reference.md` | Updated all JSON examples with new format |

### Phase 4: Deprecation (v0.6+)

| ID | Task | Status | Release | Content |
|----|------|--------|---------|---------|
| 4.1 | Add deprecation warning | ⬜ Pending | v0.6.0 | Log warning when `"type"` field is used by consumers |
| 4.2 | Remove `"type"` field | ⬜ Pending | v1.0.0 | Breaking change, remove backward compat fields |

### Success Criteria

| ID | Criterion | Status | Notes |
|----|-----------|--------|-------|
| S1 | All JSON uses serde (no manual strings) | ✅ Done | All `println!` JSON replaced with serde |
| S2 | All JSON has `"event"` and `"command"` fields | ✅ Done | clean, projects, layers, provenance, interactive, init, watch all updated |
| S3 | jq can filter by event type reliably | ✅ Done | `jq 'select(.event == "complete" and .command == "clean")'` works |
| S4 | Existing CI pipelines work (backward compat) | ✅ Done | `type` field preserved for clean, projects, provenance |
| S5 | JSON schema documented | ✅ Done | `docs/json-output.md` created |

---

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
pub struct StartEvent<'a> {
    pub event: &'static str, // "start"
    pub command: &'a str,
    pub version: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

#[derive(Serialize)]
pub struct CompleteEvent<'a> {
    pub event: &'static str, // "complete"
    pub command: &'a str,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

#[derive(Serialize)]
pub struct ErrorEvent<'a> {
    pub event: &'static str, // "error"
    pub command: &'a str,
    pub code: &'a str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

#[derive(Serialize)]
pub struct DataEvent<'a, T: Serialize> {
    pub event: &'static str, // "data"
    pub command: &'a str,
    #[serde(flatten)]
    pub data: T,
}
```

---

## Migration Strategy

### For `clean.rs` (Highest Priority)

**Current** (lines 654-684):
```rust
println!(r#"{{"type":"clean_start","scope":"{}","file_count":{}}}"#, scope, count);
```

**After**:
```rust
#[derive(Serialize)]
struct CleanStartEvent {
    event: &'static str,
    command: &'static str,
    #[serde(rename = "type")] // backward compat
    type_compat: &'static str,
    scope: String,
    file_count: usize,
}

emit_event(&CleanStartEvent {
    event: "start",
    command: "clean",
    type_compat: "clean_start", // deprecated
    scope: scope.to_string(),
    file_count: count,
})?;
```

### Backward Compatibility

During transition, emit both fields:
```json
{
  "event": "start",
  "type": "clean_start",  // deprecated, removed in v1.0
  "command": "clean",
  ...
}
```

---

## Development Approach: TDD

All implementation follows Test-Driven Development:

1. **Write failing test first** - Define expected behavior
2. **Implement minimum code** - Make test pass
3. **Refactor** - Improve while keeping tests green

### Testing Strategy

1. **Unit tests**: Each event type serializes correctly
2. **Integration tests**: JSON output can be parsed by jq
3. **Snapshot tests**: JSON output format is stable
4. **Property tests**: No invalid JSON is ever produced

### TDD Workflow Example

```bash
# 1. Write failing test
cargo test json_events -- --nocapture
# Expected: FAIL (module doesn't exist)

# 2. Implement minimum code
# ...edit code...

# 3. Run test again
cargo test json_events
# Expected: PASS

# 4. Refactor if needed, keep tests green
```

---

## Rollout Plan

1. Merge Phase 1 (infrastructure) - No user impact
2. Merge Phase 2 (migration) - Backward compatible
3. Release v0.6.0 with deprecation warnings
4. Remove `"type"` field in v1.0

---

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

---

## Risks

| Risk | Mitigation |
|------|------------|
| Breaking existing consumers | Emit both `"event"` and `"type"` during transition |
| Performance regression from serde | Benchmark before/after |
| Scope creep | Strict phase boundaries |

---

## Timeline Estimate

- Phase 1: 1-2 hours
- Phase 2: 2-3 hours
- Phase 3: 1 hour
- Phase 4: Future release

**Total**: ~5 hours for v0.6.0 compatible changes
