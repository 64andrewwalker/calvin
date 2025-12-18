# API Design Review

> **Generated**: 2025-12-19  
> **Workflow**: 5-QA API Design Review  
> **Version**: Calvin 0.2.0

---

## 📊 Executive Summary

Calvin's API design is **well-structured and consistent**. The CLI interface follows modern conventions, the library exports are minimal and stable, and JSON output is consistent across commands.

| Aspect | Status | Score |
|--------|--------|-------|
| CLI Consistency | ✅ Excellent | 9/10 |
| Library API | ✅ Excellent | 9/10 |
| JSON Output Format | ✅ Good | 8/10 |
| Error Handling | ✅ Good | 8/10 |
| Versioning Strategy | ✅ Excellent | 9/10 |
| Documentation | ✅ Good | 8/10 |

---

## 1. CLI API Review

### 1.1 Command Structure

```
calvin [GLOBAL_OPTIONS] <COMMAND> [COMMAND_OPTIONS]
```

**Commands** (v0.2.0):

| Command | Purpose | Status |
|---------|---------|--------|
| `deploy` | Deploy assets to targets | ✅ Primary |
| `check` | Security/config validation | ✅ Primary |
| `explain` | Usage explanation | ✅ Primary |
| `watch` | Continuous sync | ✅ Primary |
| `diff` | Preview changes | ✅ Primary |
| `version` | Show versions | ✅ Primary |
| `sync` | Legacy deploy | ⚠️ Hidden/deprecated |
| `install` | Legacy home deploy | ⚠️ Hidden/deprecated |
| `doctor` | Legacy check | ⚠️ Hidden/deprecated |
| `audit` | Legacy strict check | ⚠️ Hidden/deprecated |
| `migrate` | Version migration | ⚠️ Hidden (placeholder) |
| `parse` | Debug parsing | ⚠️ Hidden (debug) |

**Assessment**: Clean command hierarchy with sensible deprecation strategy. ✅

### 1.2 Global Options

| Option | Short | Type | Purpose |
|--------|-------|------|---------|
| `--json` | - | flag | Machine-readable output |
| `--color` | - | enum | Color mode (auto/always/never) |
| `--no-animation` | - | flag | Disable animations |
| `--verbose` | `-v` | count | Verbosity level |
| `--help` | `-h` | flag | Show help |
| `--version` | `-V` | flag | Show version |

**Assessment**: Standard CLI conventions followed. ✅

### 1.3 Flag Naming Consistency

| Pattern | Examples | Consistent |
|---------|----------|------------|
| Negation prefix | `--no-animation` | ✅ |
| Action flags | `--force`, `--yes`, `--dry-run` | ✅ |
| Path arguments | `--source`, `--remote` | ✅ |
| Mode selection | `--mode`, `--targets` | ✅ |
| Short flags | `-s`, `-t`, `-f`, `-y`, `-v` | ✅ |

**Finding**: All flag names follow kebab-case convention. ✅

### 1.4 Exit Codes

| Code | Meaning | Standard |
|------|---------|----------|
| `0` | Success | ✅ POSIX |
| `1` | Runtime error | ✅ POSIX |
| `2` | Invalid arguments | ✅ POSIX |

**Assessment**: Standard exit codes. ✅

---

## 2. Library API Review

### 2.1 Public Exports (`lib.rs`)

```rust
// Modules
pub mod adapters;
pub mod config;
pub mod error;
pub mod fs;
pub mod models;
pub mod parser;
pub mod security;
pub mod security_baseline;
pub mod sync;
pub mod watcher;

// Convenience re-exports
pub use adapters::{all_adapters, get_adapter, OutputFile, TargetAdapter};
pub use config::{Config, SecurityMode};
pub use error::{CalvinError, CalvinResult};
pub use models::{AssetKind, Frontmatter, PromptAsset, Scope, Target};
pub use parser::parse_frontmatter;
pub use security::{run_doctor, DoctorReport, DoctorSink};
pub use sync::{compile_assets, SyncEngine, SyncEngineOptions, SyncOptions, SyncResult};
pub use watcher::{parse_incremental, watch, IncrementalCache, WatchEvent, WatchOptions};
```

**Assessment**:
- ✅ Minimal surface area (only essential types exported)
- ✅ Clear module organization
- ✅ Result types with custom error enum
- ⚠️ `security_baseline` could be private (only used internally)

### 2.2 Error Types (`CalvinError`)

| Variant | When Used | Quality |
|---------|-----------|---------|
| `MissingField` | Required YAML field missing | ✅ Includes file:line |
| `InvalidFrontmatter` | YAML parse error | ✅ Includes message |
| `NoFrontmatter` | Missing `---` delimiter | ✅ Clear guidance |
| `UnclosedFrontmatter` | Missing closing `---` | ✅ Clear guidance |
| `Io` | Filesystem errors | ✅ From impl |
| `Yaml` | serde_yaml errors | ✅ From impl |
| `DirectoryNotFound` | Missing directory | ✅ Includes path |
| `InvalidAssetKind` | Unknown kind value | ✅ Includes file |
| `PathEscape` | Path traversal attempt | ✅ Security-aware |
| `SyncAborted` | User cancelled | ✅ Distinct from errors |

**Assessment**: Well-structured error enum with actionable messages. ✅

### 2.3 Core Data Models

| Type | Purpose | Serde Config |
|------|---------|--------------|
| `Target` | Platform enum | kebab-case, ValueEnum |
| `AssetKind` | Asset type enum | lowercase |
| `Scope` | Install scope enum | lowercase |
| `Frontmatter` | YAML metadata struct | Serialize + Deserialize |
| `PromptAsset` | Parsed source file | Internal use |

**Consistency Check**:
- ✅ All enums use `#[serde(rename_all = "...")]`
- ✅ Required fields are documented
- ✅ Defaults are sensible (`AssetKind::Action`, `Scope::Project`)

---

## 3. JSON Output Format Review

### 3.1 Event-Based NDJSON Structure

All commands emit events in a consistent format:

```json
{"event": "start", "command": "<name>", ...context}
{"event": "<action>", ...data}
{"event": "complete", "command": "<name>", ...summary}
```

### 3.2 Command Output Samples

**`calvin deploy --json`**:
```json
{"command":"deploy","event":"start"}
{"command":"deploy","errors":0,"event":"complete","skipped":1,"status":"success","written":109}
```

**`calvin check --json`**:
```json
{"command":"check","event":"start","mode":"Balanced","strict_warnings":false,"verbose":0}
{"command":"check","event":"check","name":"commands","platform":"Claude Code","status":"pass","message":"OK"}
{"command":"check","event":"complete","errors":0,"passes":5,"warnings":4,"success":true}
```

**`calvin watch --json`** (NDJSON stream):
```json
{"event":"started","source":".promptpack"}
{"event":"sync_started"}
{"event":"sync_complete","written":36,"skipped":111,"errors":0}
```

### 3.3 JSON Consistency Analysis

| Field | Convention | Consistent |
|-------|------------|------------|
| `event` | lowercase | ✅ Always present |
| `command` | lowercase | ✅ In start/complete |
| `status` | lowercase | ✅ pass/warning/fail/success |
| `*_count` | snake_case | ✅ written, skipped, errors |

**Issues Found**:

1. **Minor**: `check` uses `passes` (plural) but `errors`/`warnings` (plural) - consistent
2. **Minor**: `watch` events use `sync_started` (snake_case) vs `started` (no prefix) - slight inconsistency

**Recommendation**: Standardize watch events to `watch_started`, `watch_sync_started` (low priority)

---

## 4. Configuration API Review

### 4.1 Config File Structure

```toml
# .promptpack/config.toml
[format]
version = "1.0"

[security]
mode = "balanced"
allow_naked = false
deny = ["*.secret"]

[security.mcp]
allowlist = ["filesystem", "github"]

[targets]
enabled = ["claude-code", "cursor"]

[sync]
atomic_writes = true
respect_lockfile = true

[output]
verbosity = "normal"
```

**Assessment**:
- ✅ Clear section hierarchy
- ✅ Sensible defaults
- ✅ Type-safe with serde validation
- ✅ Unknown key detection with suggestions

### 4.2 Environment Variables

| Variable | Maps To | Implemented |
|----------|---------|-------------|
| `CALVIN_SECURITY_MODE` | `security.mode` | ✅ |
| `CALVIN_TARGETS` | `targets.enabled` | ✅ |
| `CALVIN_VERBOSITY` | `output.verbosity` | ✅ |
| `CALVIN_ATOMIC_WRITES` | `sync.atomic_writes` | ✅ |

**Assessment**: Clean `CALVIN_*` prefix convention. ✅

---

## 5. Versioning Strategy Review

### 5.1 Dual-Layer Versioning

| Layer | Current | Policy |
|-------|---------|--------|
| Source Format | 1.0 | SemVer (major = breaking) |
| CLI | 0.2.0 | SemVer |
| Adapters | v1 (all) | Per-adapter versioning |

### 5.2 Deprecation Policy

```
v1.0 → v1.1 (deprecated) → v1.2 → v2.0 (removed)
```

- ✅ N-2 version support documented
- ✅ `calvin migrate` command exists
- ✅ Warning levels defined (info → deprecated → error)

### 5.3 Breaking Change Management

| Type | Process |
|------|---------|
| Source format change | Bump format version, provide migration |
| CLI command change | Hide deprecated, show warning |
| Adapter output change | Version adapter, update min IDE version |

**Assessment**: Comprehensive versioning strategy. ✅

---

## 6. Documentation Review

### 6.1 API Documentation Files

| Document | Status | Quality |
|----------|--------|---------|
| `docs/command-reference.md` | ✅ Complete | Excellent |
| `docs/configuration.md` | ✅ Complete | Excellent |
| `docs/api/versioning.md` | ✅ Complete | Excellent |
| `docs/api/changelog.md` | ✅ Complete | Good |
| Rust docstrings | ⚠️ Partial | Good for public API |

### 6.2 Missing Documentation

| Item | Priority | Recommendation |
|------|----------|----------------|
| OpenAPI/JSON Schema | P3 | Not applicable (CLI tool) |
| Library usage examples | P2 | Add to `lib.rs` docs |
| Adapter extension guide | P3 | Document `TargetAdapter` trait |

---

## 7. Security Review (API Context)

### 7.1 Input Validation

| Input | Validation | Status |
|-------|------------|--------|
| File paths | Path traversal check (`PathEscape` error) | ✅ |
| YAML frontmatter | Schema validation via serde | ✅ |
| CLI arguments | clap validation | ✅ |
| Config keys | Unknown key detection | ✅ |
| Target enum | Exhaustive match | ✅ |

### 7.2 Security Considerations

- ✅ No network requests without explicit `--remote` flag
- ✅ Path safety validated before writes
- ✅ Lockfile prevents accidental overwrites
- ✅ `--force` required for destructive operations

---

## 8. Issues & Recommendations

### 8.1 Issues Found

| ID | Severity | Description |
|----|----------|-------------|
| API-1 | Low | Watch event naming inconsistency (`started` vs `sync_started`) |
| API-2 | Info | `security_baseline` module unnecessarily public |
| API-3 | Info | No library usage examples in docstrings |

### 8.2 Recommendations

| Priority | Action | Impact |
|----------|--------|--------|
| P3 | Standardize watch event names | Minor JSON consistency |
| P3 | Make `security_baseline` private | Smaller public API |
| P2 | Add `//! # Examples` to lib.rs | Better library UX |
| P3 | Add JSON Schema for outputs | CI/tooling integration |

---

## 9. Compatibility Matrix

### 9.1 CLI Compatibility

| Feature | v0.1.x | v0.2.x | Notes |
|---------|--------|--------|-------|
| `sync` command | ✅ | ⚠️ Hidden | Use `deploy` |
| `install` command | ✅ | ⚠️ Hidden | Use `deploy --home` |
| `deploy` command | ❌ | ✅ | New in v0.2 |
| `--json` flag | ✅ | ✅ | Stable |
| Exit codes | ✅ | ✅ | Stable |

### 9.2 Breaking Changes Since v0.1

None - v0.2 maintains full backward compatibility via hidden deprecated commands.

---

## 10. Summary

### Strengths

1. **Consistent CLI design** - Standard conventions, clear help text
2. **Clean library API** - Minimal exports, good error types
3. **Documented versioning** - Clear deprecation policy
4. **Secure defaults** - Path validation, lockfile protection
5. **Machine-friendly** - Consistent JSON output, proper exit codes

### Areas for Improvement

1. Minor event naming inconsistency in watch mode
2. Could reduce public API surface (`security_baseline`)
3. Library examples in documentation

### Verdict

**API Quality: Excellent (8.5/10)**

The API is well-designed for both human users and automation. No breaking changes needed.
