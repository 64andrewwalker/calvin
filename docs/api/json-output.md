# Calvin JSON Output Format

> **Version**: 0.6.0
> **Last Updated**: 2025-12-26

Calvin commands support a `--json` flag that outputs machine-readable NDJSON (Newline-Delimited JSON) for CI/CD pipelines and automation tools.

## Event Protocol

All JSON output follows a consistent event protocol:

```json
{"event": "start", "command": "CMD", ...}
{"event": "progress", "command": "CMD", ...}
{"event": "data", "command": "CMD", ...}
{"event": "complete", "command": "CMD", "success": true|false, ...}
```

For errors:
```json
{"event": "error", "command": "CMD", "code": "ERROR_CODE", "message": "...", ...}
```

## Common Fields

All events include these fields:

| Field | Type | Description |
|-------|------|-------------|
| `event` | string | Event type: `start`, `progress`, `data`, `complete`, `error` |
| `command` | string | Command name: `deploy`, `clean`, `check`, `layers`, etc. |

## Event Types

### start

Emitted when a command begins execution.

```json
{"event": "start", "command": "deploy", "source": ".promptpack", "destination": "~/", "asset_count": 5}
```

### progress

Emitted during long-running operations to report incremental progress.

```json
{"event": "progress", "command": "clean", "type": "file_deleted", "path": "/path/to/file"}
```

### data

Emitted for data-centric responses (list commands, queries).

```json
{"event": "data", "command": "layers", "layers": [...]}
```

### complete

Emitted when a command finishes.

```json
{"event": "complete", "command": "deploy", "success": true, "written": 10, "skipped": 2}
```

### error

Emitted when an error occurs.

```json
{"event": "error", "command": "deploy", "code": "FILE_NOT_FOUND", "message": "Config not found"}
```

---

## Command-Specific Schemas

### deploy

```bash
calvin deploy --json
```

Events:
1. `start` - Deploy begins
2. `compiled` - Assets compiled
3. `item_start` - File processing begins
4. `item_written` / `item_skipped` / `item_error` - File result
5. `orphans_detected` - Orphan files found (if any)
6. `complete` - Deploy finished

**Example output:**

```json
{"event":"start","command":"deploy","source":".promptpack","destination":"./","asset_count":3}
{"event":"compiled","command":"deploy","output_count":5}
{"event":"item_written","command":"deploy","index":0,"path":".cursor/rules/coding.mdc"}
{"event":"complete","command":"deploy","status":"success","written":5,"skipped":0,"errors":0,"deleted":0}
```

### clean

```bash
calvin clean --json
```

Events:
1. `start` - Clean begins
2. `progress` (type: `file_deleted` / `file_skipped`) - Per-file progress
3. `complete` - Clean finished

**Example output:**

```json
{"event":"start","command":"clean","type":"clean_start","scope":"project","file_count":3}
{"event":"progress","command":"clean","type":"file_deleted","path":".cursor/rules/test.mdc","key":"project:.cursor/rules/test.mdc"}
{"event":"complete","command":"clean","type":"clean_complete","deleted":1,"skipped":0,"errors":0}
```

> **Note**: The `type` field is deprecated and will be removed in v1.0. Use `event` instead.

### clean --all

```bash
calvin clean --all --json
```

Events:
1. `start` - Clean all begins (with project count)
2. `progress` (type: `project_skipped` / `project_error` / `project_complete`) - Per-project progress
3. `complete` - Clean all finished

### check

```bash
calvin check --json
```

Events:
1. `start` - Check begins
2. `check` - Per-check result
3. `complete` - Check finished

**Example output:**

```json
{"event":"start","command":"check","mode":"Balanced","strict_warnings":false,"verbose":0}
{"event":"check","command":"check","platform":"claude-code","name":"CLAUDE.md","status":"pass","message":"Found"}
{"event":"complete","command":"check","mode":"Balanced","strict_warnings":false,"passes":5,"warnings":0,"errors":0,"success":true}
```

### layers

```bash
calvin layers --json
```

Single `data` event with layer information:

```json
{"event":"data","command":"layers","layers":[{"name":"user","layer_type":"user","original_path":"~/.calvin/.promptpack","resolved_path":"/home/user/.calvin/.promptpack","asset_count":2}],"merged_asset_count":5,"overridden_asset_count":1}
```

### projects

```bash
calvin projects --json
```

Single `data` event with project list:

```json
{"event":"data","command":"projects","type":"projects","count":2,"projects":[...],"pruned":[]}
```

> **Note**: The `type` field is deprecated and will be removed in v1.0.

### provenance

```bash
calvin provenance --json
```

Single `data` event with provenance information:

```json
{"event":"data","command":"provenance","type":"provenance","count":5,"entries":[...]}
```

> **Note**: The `type` field is deprecated and will be removed in v1.0.

### interactive (default command)

```bash
calvin --json
```

Single `data` event with project state:

```json
{"event":"data","command":"interactive","state":"configured","assets":{"total":5},"layers":[...]}
```

### explain

```bash
calvin explain --json
```

Events:
1. `start` - Explain begins
2. `complete` - Explain finished (with data payload)

### init

```bash
calvin init --json
```

Events:
1. `complete` - Init finished

**Example output:**

```json
{"event":"complete","command":"init","path":"/path/to/project/.promptpack","template":"standard"}
```

**Error case:**

```json
{"event":"error","command":"init","kind":"already_exists","path":".promptpack","message":".promptpack directory already exists"}
```

### watch

```bash
calvin watch --json
```

Events:
1. `watch_started` - Watch begins
2. `file_changed` - File modification detected
3. `sync_started` - Redeploy begins
4. `sync_complete` - Redeploy finished
5. `error` - Error occurred
6. `shutdown` - Watch stopped

**Example output:**

```json
{"event":"watch_started","command":"watch","source":".promptpack","watch_all_layers":false,"watching":[".promptpack"]}
{"event":"file_changed","command":"watch","path":"policies/security.md"}
{"event":"sync_started","command":"watch"}
{"event":"sync_complete","command":"watch","written":5,"skipped":2,"errors":0}
{"event":"shutdown","command":"watch"}
```

---

## Parsing NDJSON

Each line is a valid JSON object. Parse line by line:

**jq example:**

```bash
calvin deploy --json | jq -c 'select(.event == "complete")'
```

**Node.js example:**

```javascript
const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin });

rl.on('line', (line) => {
  const event = JSON.parse(line);
  if (event.event === 'complete') {
    process.exit(event.success ? 0 : 1);
  }
});
```

**Python example:**

```python
import json
import sys

for line in sys.stdin:
    event = json.loads(line)
    if event['event'] == 'complete':
        sys.exit(0 if event.get('success') else 1)
```

---

## Backward Compatibility

### Deprecated Fields

The following fields are deprecated and will be removed in v1.0:

| Field | Replacement | Commands |
|-------|-------------|----------|
| `type` | `event` + `command` | clean, projects, provenance |

During the transition period, both `event` and `type` are emitted for backward compatibility.

### Migration Guide

**Before (v0.4):**
```json
{"type": "clean_start", "scope": "project", "file_count": 3}
```

**After (v0.5+):**
```json
{"event": "start", "command": "clean", "type": "clean_start", "scope": "project", "file_count": 3}
```

Update your parsers to use `event` instead of `type`:

```bash
# Old
jq 'select(.type == "clean_complete")'

# New
jq 'select(.event == "complete" and .command == "clean")'
```

