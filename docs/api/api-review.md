# API Design Review

> **Date**: 2025-12-26
> **Reviewer**: AI Assistant
> **Scope**: CLI API, JSON output, flag naming

## Executive Summary

Overall the CLI API is well-designed with good consistency in most areas. A few inconsistencies were found in JSON output format that should be addressed.

## 1. CLI Command Naming ✅

| Command | Purpose | Status |
|---------|---------|--------|
| `deploy` | Deploy assets | ✅ Clear verb |
| `check` | Validate config/security | ✅ Clear verb |
| `watch` | Watch for changes | ✅ Clear verb |
| `diff` | Preview changes | ✅ Clear verb |
| `clean` | Remove deployed files | ✅ Clear verb |
| `init` | Initialize directory | ✅ Standard convention |
| `explain` | Show usage | ✅ Clear purpose |
| `version` | Show version | ✅ Standard |
| `layers` | Show layer stack | ✅ Clear noun |
| `projects` | List projects | ✅ Clear noun |
| `provenance` | Show file sources | ✅ Domain term |
| `migrate` | Migrate formats | ✅ Hidden, appropriate |
| `parse` | Debug parsing | ✅ Hidden, appropriate |

**Verdict**: All command names are clear, concise, and follow Unix conventions.

## 2. Flag Naming Conventions ✅

### Global Flags
| Flag | Short | Purpose | Consistency |
|------|-------|---------|-------------|
| `--json` | - | CI output | ✅ Standard |
| `--color` | - | Color mode | ✅ Standard |
| `--verbose` | `-v` | Verbosity | ✅ Standard |
| `--no-animation` | - | Disable animations | ✅ Uses `no-` prefix |

### Common Flags Across Commands
| Flag | Commands | Consistency |
|------|----------|-------------|
| `--source` / `-s` | deploy, watch, diff, clean | ✅ Consistent |
| `--home` | deploy, watch, diff, clean | ✅ Consistent |
| `--dry-run` | deploy, clean, migrate | ✅ Standard |
| `--yes` / `-y` | deploy, clean | ✅ Consistent |
| `--force` / `-f` | deploy, clean, init | ✅ Consistent |

**Verdict**: Flag naming is consistent and follows standard conventions.

## 3. JSON Output Format ⚠️ Issues Found

### Issue 3.1: Inconsistent Event/Type Naming

**Current State**:
```
# explain.rs, interactive/mod.rs
{"event": "start", "command": "explain" ...}
{"event": "complete", "command": "explain" ...}

# clean.rs
{"type": "clean_start", "scope": "..." ...}
{"type": "file_deleted", "path": "..." ...}
{"type": "clean_complete", "deleted": ... ...}

# projects.rs
{"type": "projects", "count": ... ...}
```

**Problem**: Mix of `"event"` and `"type"` field names, and inconsistent naming patterns.

**Recommendation**: Standardize on `"event"` field with format:
```json
{"event": "command_start", "command": "clean", ...}
{"event": "file_deleted", "command": "clean", ...}
{"event": "command_complete", "command": "clean", ...}
```

### Issue 3.2: Inconsistent JSON Construction

**Current State**:
- `clean.rs`: Manual string interpolation (error-prone)
- `explain.rs`, `interactive/mod.rs`: `serde_json::json!` macro
- `layers.rs`: `serde_json::to_string()`

**Problem**: Manual string interpolation in `clean.rs` is fragile and may produce invalid JSON.

**Recommendation**: Use `serde_json::json!` or derive `Serialize` consistently.

### Issue 3.3: Missing Event Envelope

**Current State**: Some commands output bare data without event wrapper.

**Recommendation**: All JSON output should follow NDJSON pattern:
```json
{"event": "command_start", "command": "CMD", "timestamp": "...", ...}
{"event": "data", "command": "CMD", ...actual_data...}
{"event": "command_complete", "command": "CMD", "success": true, ...}
```

## 4. Error Message Format ✅

Error messages consistently include:
- What went wrong
- Why it happened (when knowable)
- How to fix it
- Documentation links

Example:
```
error: registry file corrupted: ~/.calvin/registry.toml
  → Fix: Delete and rebuild registry
  → Run: rm ~/.calvin/registry.toml && calvin deploy
  → See: https://calvin.dev/guides/configuration
```

**Verdict**: Error messages follow best practices.

## 5. Default Values ✅

| Flag | Default | Rationale |
|------|---------|-----------|
| `--source` | `.promptpack` | Most common case |
| `--template` (init) | `standard` | Safe middle ground |
| `--mode` (check) | `balanced` | Safe middle ground |

**Verdict**: Defaults are sensible and documented.

## 6. Mutual Exclusivity ✅

Conflicting flags are properly marked:
- `deploy`: `--home`, `--project`, `--remote` are mutually exclusive
- `clean`: `--home`, `--project`, `--all` are mutually exclusive

**Verdict**: Conflicts are properly handled.

## 7. Recommendations Summary

### High Priority (Breaking Change Risk: Low)

1. **Standardize JSON output format** - Create shared event types
2. **Replace manual JSON string building in `clean.rs`** - Use serde

### Medium Priority

3. **Add `--format` flag** - Allow `json`, `text`, `table` output formats
4. **Document JSON schema** - Add OpenAPI/JSON Schema for CI integration

### Low Priority

5. **Add command aliases** - e.g., `calvin d` for `deploy`
6. **Add shell completion** - Generate completions for bash/zsh/fish

## Action Items

- [ ] Create `src/ui/json/events.rs` with standardized event types
- [ ] Refactor `clean.rs` to use serde for JSON output
- [ ] Add JSON schema documentation to `docs/api/`
- [ ] Add deprecation warnings for `"type"` field (for v1.0 migration)

## Appendix: File Locations

| Concern | Location |
|---------|----------|
| CLI definition | `src/presentation/cli.rs` |
| JSON utilities | `src/ui/json.rs` |
| Command handlers | `src/commands/*.rs` |
| Output formatting | `src/presentation/output.rs` |

