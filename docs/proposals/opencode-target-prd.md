# OpenCode Target Support PRD

> **Status**: Draft  
> **Author**: AI Assistant  
> **Created**: 2026-01-06  
> **Target Version**: 0.9.0

---

## Executive Summary

This PRD proposes adding **OpenCode** as a new deployment target in Calvin. OpenCode is an open-source AI coding agent for the terminal, developed by SST, that supports agents, skills, commands, and modes with a flexible configuration system.

**Key insight**: OpenCode has a well-defined directory structure with different asset types mapping to specific subdirectories under `.opencode/`. It supports both JSON configuration (`opencode.json`) and Markdown files with YAML frontmatter.

**Decision**: TBD - awaiting stakeholder input.

---

## 1. Problem Statement

### Current State

Calvin currently supports 5 deployment targets:

| Target | Status | Native Agent Support | Native Commands |
|--------|--------|---------------------|-----------------|
| Claude Code | ✅ Stable | ✅ `.claude/agents/` | ✅ `.claude/commands/` |
| Cursor | ✅ Stable | ⚠️ Command fallback | ✅ `.cursor/commands/` |
| VS Code | ✅ Stable | ⚠️ Instructions fallback | ❌ |
| Antigravity | ✅ Stable | ⚠️ Workflow fallback | ⚠️ Workflows |
| Codex | ✅ Stable | ⚠️ Prompt fallback | ⚠️ Prompts |

### OpenCode Overview

OpenCode is an open-source, provider-agnostic AI coding agent built for the terminal by SST.

**Key Features**:
- Multi-provider support (Anthropic, OpenAI, Google, etc.)
- Terminal UI (TUI) with rich interface
- Client/Server architecture
- Claude Code compatibility (reads `.claude/skills/`)
- Native slash commands support
- Custom modes for different workflows

**Directory Structure** (per OpenCode docs):

```
~/.config/opencode/           # Global config directory (XDG compliant)
├── opencode.json             # Global configuration (JSON/JSONC)
├── AGENTS.md                 # Global rules/instructions (NOT OPENCODE.md!)
├── agent/                    # Global agents (Markdown)
│   └── reviewer.md
├── command/                  # Global commands (Markdown)
│   └── test.md
├── mode/                     # Global modes (DEPRECATED - use agents instead)
│   └── review.md
├── plugin/                   # Global plugins (JS/TS)
│   └── my-plugin.ts
├── tool/                     # Custom tools (JS/TS definitions, can call any language)
│   └── my-tool.ts
└── skill/                    # Global skills
    └── git-release/
        └── SKILL.md

./                            # Project root
├── opencode.json             # Project configuration (also opencode.jsonc)
├── AGENTS.md                 # Project rules/instructions (merged with global)
└── .opencode/                # Project-specific directory
    ├── agent/                # Project agents (*.md, filename = agent name)
    │   └── reviewer.md
    ├── command/              # Project commands (*.md, filename = command name)
    │   └── deploy.md
    ├── mode/                 # Project modes (DEPRECATED)
    │   └── debug.md
    ├── plugin/               # Local plugins (*.{js,ts})
    ├── tool/                 # Local custom tools (*.{js,ts} definitions)
    ├── skill/                # Project skills
    │   └── my-skill/
    │       └── SKILL.md
    └── package.json          # Optional: npm deps for plugin/tool (Bun installs)
```

**IMPORTANT**: OpenCode also reads Claude Code skills from:
- `.claude/skills/<name>/SKILL.md` (project)
- `~/.claude/skills/<name>/SKILL.md` (user)

### The Question

Should Calvin add OpenCode as a dedicated target with native format support?

**Arguments FOR OpenCode target**:

1. Native `.opencode/` directory support with proper structure
2. Growing open-source community (SST backing)
3. Native slash command support (`.opencode/command/`)
4. Different tools format requiring conversion
5. Skills support with Claude compatibility

**Arguments AGAINST**:

1. OpenCode reads Claude Code skills from `.claude/skills/`
2. Adds maintenance burden
3. OpenCode still evolving

---

## 2. Goals

### Primary Goals

1. Add `opencode` as a new `Target` variant
2. Create `OpenCodeAdapter` implementing `TargetAdapter` trait
3. Support all asset types mapping:
   - `agent` → `.opencode/agent/<id>.md`
   - `action` → `.opencode/command/<id>.md` ← **Actions become commands**
   - `policy` → `.opencode/mode/<id>.md` or `AGENTS.md`
   - `skill` → `.opencode/skill/<id>/SKILL.md`
4. Handle tool/model format conversion
5. Document OpenCode-specific frontmatter fields

### Non-Goals

1. Plugin generation (`.opencode/plugin/`)
2. Custom tool generation (`.opencode/tool/`)
3. MCP server configuration
4. Provider configuration generation

---

## 3. OpenCode Path Matrix

| Asset Type | Project Scope | User Scope |
|------------|---------------|------------|
| Configuration | `opencode.json` / `opencode.jsonc` | `~/.config/opencode/opencode.json` |
| Rules/Instructions | `AGENTS.md` (project root) | `~/.config/opencode/AGENTS.md` |
| Agents | `.opencode/agent/<id>.md` | `~/.config/opencode/agent/<id>.md` |
| Commands | `.opencode/command/<id>.md` | `~/.config/opencode/command/<id>.md` |
| Modes (DEPRECATED) | `.opencode/mode/<id>.md` | `~/.config/opencode/mode/<id>.md` |
| Skills | `.opencode/skill/<id>/SKILL.md` | `~/.config/opencode/skill/<id>/SKILL.md` |

**Important Notes**: 
- OpenCode respects `XDG_CONFIG_HOME` if set, defaulting to `~/.config/`
- `OPENCODE_CONFIG_DIR` env var can override `.opencode/` structure
- OpenCode also reads Claude Code skills from `.claude/skills/*/SKILL.md`
- **Modes are DEPRECATED** since v1.1.1 - prefer using agents instead
- Rules/instructions file is `AGENTS.md`, NOT `AGENTS.md`

---

## 4. Format Specifications

### 4.1 Supported File Formats

| Resource Type | Format | Location |
|---------------|--------|----------|
| Configuration | JSON / JSONC | `opencode.json` |
| Agents | Markdown + YAML frontmatter | `.opencode/agent/*.md` |
| Commands | Markdown + YAML frontmatter | `.opencode/command/*.md` |
| Modes | Markdown + YAML frontmatter | `.opencode/mode/*.md` |
| Skills | Markdown + YAML frontmatter | `.opencode/skill/*/SKILL.md` |
| Plugins | JavaScript / TypeScript | `.opencode/plugin/*.{js,ts}` |
| Tools | JavaScript / TypeScript | `.opencode/tool/*.{js,ts}` |

### 4.2 Agent Format Comparison

| Field | Claude Code | OpenCode |
|-------|-------------|----------|
| `name` | Required string | **File name is agent name** (e.g., `reviewer.md` → `reviewer`) |
| `description` | Required string | Required string |
| `tools` | Comma-separated string | **Object `{ tool: boolean }`** ⚠️ DEPRECATED v1.1.1+ |
| `model` | `sonnet`/`opus`/`haiku` | **Stable alias** `anthropic/claude-sonnet-4-5` |
| `permissionMode` | Enum string | Not used (see `permission` object) |
| `skills` | Comma-separated string | Not applicable |
| `mode` | Not used | **`primary` or `subagent`** |
| `temperature` | Not used | **Float 0.0-1.0** |
| `permission` | Not used | **String or object** (see §4.5) |

> **⚠️ DEPRECATION**: Since OpenCode v1.1.1, the `tools` boolean config is DEPRECATED. 
> Use `permission` object instead for tool access control. Calvin should output BOTH 
> for backwards compatibility with older OpenCode versions.

### 4.3 Tool Mapping

**OpenCode Available Tools** (from official docs):

| Tool | Description | Permission-controlled |
|------|-------------|----------------------|
| `bash` | Execute shell commands | ✅ (supports pattern rules) |
| `edit` | Modify existing files (writes/patches/edits) | ✅ |
| `write` | Create new files | - |
| `read` | Read file contents | - |
| `grep` | Search file contents | - |
| `glob` | Find files by pattern | - |
| `list` | List directory contents | - |
| `patch` | Apply patches to files | - |
| `webfetch` | Fetch web content | ✅ |
| `skill` | Load skills | ✅ (supports pattern rules) |
| `task` | Spawn subagent/subtask | ✅ |
| `todoread` | Read todo lists | - |
| `todowrite` | Manage todo lists | - |

**Calvin → OpenCode Tool Mapping**:

| Calvin/Claude Tool | OpenCode Tool Key | Notes |
|-------------------|-------------------|-------|
| `Read` | `read` | Direct mapping (lowercase) |
| `Write` | `write` | Direct mapping (lowercase) |
| `Edit` | `edit` | Direct mapping (lowercase) |
| `Bash` | `bash` | Direct mapping (lowercase) |
| `Grep` | `grep` | Direct mapping (lowercase) |
| `Glob` | `glob` | Direct mapping (lowercase) |
| `WebFetch` | `webfetch` | Direct mapping (lowercase) |
| `Task` | `task` | Spawns subagent/subtask |
| `Skill` | `skill` | Skill invocation |

**IMPORTANT**: 
- OpenCode tools use **lowercase** names in configuration
- Do NOT use `readTool`, `globTool` etc. - these are incorrect
- The `edit` permission covers writes, patches, AND edits

### 4.4 Model Handling

> **Important**: Calvin does NOT convert model names. It passes through as-is and only validates.

**Common Model Names** (for reference):

| Provider | Model Examples |
|----------|----------------|
| Anthropic | `anthropic/claude-sonnet-4-5`, `anthropic/claude-opus-4-5` |
| OpenAI | `openai/gpt-4o`, `openai/gpt-4-turbo` |
| Google | `google/gemini-2.0-flash`, `google/gemini-pro` |

**Strategy**: 
- **Pass through**: Model names are passed through unchanged to OpenCode output
- **Validation only**: Validate against known model patterns using external library
- **Warning on unknown**: If model name is not recognized, emit warning (not error)
- **Implementation Note**: Use [rust-genai](https://github.com/jeremychone/rust-genai) library for model name validation
  - `AdapterKind` enum provides automatic provider detection from model name prefix
  - Model names starting with "gpt" → OpenAI, "claude" → Anthropic, "gemini" → Gemini, etc.
  - Unknown prefixes → emit warning (not error)

**Warning Example**:
```
Warning: Model 'custom-model-xyz' may not be a standard model name.
Hint: Check OpenCode documentation for supported models.
```

### 4.5 Permission Mode to Permission Object

**OpenCode Permission Format** (v1.1.1+):

#### Permission Values

| Value | Description |
|-------|-------------|
| `"allow"` | Permit without asking |
| `"ask"` | Prompt user each time |
| `"deny"` | Block completely |

#### Two Forms of Permission

**Form 1: Global String** - Applies to ALL tools

```yaml
permission: allow       # Allow everything
permission: ask         # Ask for everything
permission: deny        # Deny everything
```

**Form 2: Granular Object** - Per-tool control

```yaml
permission:
  edit: allow           # Allow file edits
  bash: ask             # Ask before shell commands
  webfetch: deny        # Block web requests
  task: allow           # Allow subagent spawning
  skill: allow          # Allow skill invocation
```

#### Pattern-Based Rules (Advanced)

For `bash` and `skill`, you can use pattern matching:

```yaml
permission:
  bash:
    "git status": allow     # Exact match
    "git push": ask         # Exact match
    "npm *": allow          # Wildcard: any npm command
    "rm -rf *": deny        # Block dangerous commands
    "*": ask                # Default: ask for all others
  skill:
    "code-review": allow
    "internal-*": deny      # Block skills starting with internal-
    "*": ask                # Default
```

#### Wildcard Rules

| Pattern | Matches | Example |
|---------|---------|---------|
| `*` | Zero or more chars | `npm *` → `npm install`, `npm test` |
| `?` | Exactly one char | `test?` → `test1`, `testA` |

**Priority**: More specific patterns override wildcards.
- `"git push": ask` takes precedence over `"git *": allow`

#### Implementation Note

Permission handling follows the existing Claude Code implementation in Calvin:
- See `src/infrastructure/adapters/agents.rs` for reference
- Valid Claude Code values: `default`, `acceptEdits`, `dontAsk`, `bypassPermissions`, `plan`, `ignore`
- Invalid values emit warning (not error)

#### Calvin → OpenCode Mapping

**Calvin → OpenCode Permission Mapping**:

| Claude Code `permissionMode` | OpenCode `permission` |
|------------------------------|----------------------|
| `default` | Omit (use global config) |
| `acceptEdits` | `{ edit: "allow" }` |
| `dontAsk` | `"allow"` (string form) OR `{ bash: "allow", edit: "allow", webfetch: "allow" }` |
| `bypassPermissions` | `"allow"` (string form - allows everything) |
| `plan` | `{ edit: "deny", bash: "deny" }` |
| `ignore` | Not mappable (skip with warning) |

> **Note**: OpenCode's `permission` system is more granular than Claude Code's `permissionMode`.
> When mapping, prefer the string form `"allow"`/`"deny"` for simple cases.

---

## 5. Calvin → OpenCode Asset Mapping

### 5.0 Complete Mapping Summary

**Calvin `.promptpack/` Structure → OpenCode Output**:

```
.promptpack/                           OpenCode Output
├── agents/                            
│   └── code-reviewer.md        →      .opencode/agent/code-reviewer.md
│   └── debugger.md             →      .opencode/agent/debugger.md
│                                      
├── actions/                           
│   └── test.md                 →      .opencode/command/test.md
│   └── deploy.md               →      .opencode/command/deploy.md
│                                      
├── policies/                          (OPTION A - Recommended)
│   └── code-style.md           →      AGENTS.md (aggregated)
│   └── security.md             →      AGENTS.md (aggregated)
│                                      (OPTION B - Deprecated)
│   └── code-review.md          →      .opencode/mode/code-review.md
│                                      
├── skills/                            
│   └── git-release/            →      .opencode/skill/git-release/
│       └── SKILL.md            →          └── SKILL.md
│       └── scripts/            →          └── scripts/
│       └── references/         →          └── references/
│                                      
└── memory/                            
    └── project-context.md      →      AGENTS.md (appended)
```

**Quick Reference Table**:

| Calvin Asset | Calvin Path | OpenCode Path | File Format |
|--------------|-------------|---------------|-------------|
| **Agent** | `.promptpack/agents/<id>.md` | `.opencode/agent/<id>.md` | Markdown + YAML frontmatter |
| **Action** | `.promptpack/actions/<id>.md` | `.opencode/command/<id>.md` | Markdown + YAML frontmatter |
| **Policy** | `.promptpack/policies/<id>.md` | `AGENTS.md` (aggregated) | Markdown (no frontmatter) |
| **Skill** | `.promptpack/skills/<id>/SKILL.md` | `.claude/skills/<id>/SKILL.md` | Reuse Claude Code output |
| **Memory** | `.promptpack/memory/*.md` | `AGENTS.md` (appended) | Markdown |

**Note on Skills**: 
- `targets: [opencode, claude-code]` → Skills deploy to `.claude/skills/` only (OpenCode reads this path)
- `targets: [opencode]` only → Skills deploy to `.opencode/skill/<id>/SKILL.md`

**Scope Handling**:

| Scope | OpenCode Project Output | OpenCode User Output |
|-------|------------------------|---------------------|
| `project` | `.opencode/agent/<id>.md` | N/A |
| `user` | N/A | `~/.config/opencode/agent/<id>.md` |

### 5.1 Agent → `.opencode/agent/<id>.md`

**Calvin Input** (`.promptpack/agents/code-reviewer.md`):

```markdown
---
description: Review code changes for security, performance, and maintainability.
kind: agent
scope: project
targets: [opencode]
tools:
  - Read
  - Grep
  - Bash
model: sonnet
permission-mode: acceptEdits
mode: subagent
temperature: 0.3
---

You are a senior code reviewer. Focus on:
- Security vulnerabilities
- Performance issues
- Code maintainability
```

**OpenCode Output** (`.opencode/agent/code-reviewer.md`):

```markdown
---
description: Review code changes for security, performance, and maintainability.
mode: subagent
model: sonnet
temperature: 0.3
tools:
  read: true
  grep: true
  bash: true
  write: false
  edit: false
permission:
  edit: allow
  bash:
    "git *": allow
    "*": ask
---

You are a senior code reviewer. Focus on:
- Security vulnerabilities
- Performance issues
- Code maintainability

<!-- Generated by Calvin. Source: .promptpack/agents/code-reviewer.md. DO NOT EDIT. -->
```

> **Note**: Both `tools` (DEPRECATED) and `permission` are output for backwards 
> compatibility. Newer OpenCode versions will use `permission` only.

### 5.2 Action → `.opencode/command/<id>.md`

Calvin actions map to OpenCode slash commands.

**Calvin Input** (`.promptpack/actions/test.md`):

```markdown
---
description: Generate comprehensive test suite
kind: action
scope: project
targets: [opencode]
---

Create tests for $ARGUMENTS

Requirements:
- Unit tests with 100% coverage
- Integration tests for API endpoints
- Mock external dependencies
```

**OpenCode Output** (`.opencode/command/test.md`):

```markdown
---
description: Generate comprehensive test suite
agent: build
subtask: true
---

Create tests for $ARGUMENTS

Requirements:
- Unit tests with 100% coverage
- Integration tests for API endpoints
- Mock external dependencies

<!-- Generated by Calvin. Source: .promptpack/actions/test.md. DO NOT EDIT. -->
```

**Usage**: `/test authentication module`

**Command Fields**:
| Field | Type | Description |
|-------|------|-------------|
| `description` | string | Required command description |
| `agent` | string | Which agent to execute the command. **Omit if not specified** (uses OpenCode default) |
| `subtask` | boolean | `true` = run in sub-session (isolated), `false` = run in main context |
| `model` | string | Model override for this command |

> **Important**: 
> - If Calvin action doesn't specify `agent` → omit field entirely (OpenCode uses default)
> - When `agent` is a subagent, `subtask` defaults to `true`

### 5.3 Policy → `AGENTS.md` or `.opencode/mode/<id>.md`

> **⚠️ Note**: Modes are **DEPRECATED** since OpenCode v1.1.1. The recommended approach 
> is to use `AGENTS.md` (Option A) or agents with specific configurations.

**Option A: Compile to `AGENTS.md`** (RECOMMENDED)

Aggregate all policies into `AGENTS.md` at project root:

```markdown
# Project Guidelines

<!-- Generated by Calvin. DO NOT EDIT. -->

## Code Style (from: .promptpack/policies/code-style.md)

Follow these coding standards...

## Security (from: .promptpack/policies/security.md)

Never expose secrets...
```

OpenCode automatically reads `AGENTS.md` from project root and global config.

**Calvin Behavior**:
- If `AGENTS.md` already exists in target location → **SKIP** (do not overwrite)
- This follows Calvin's default behavior for existing files

> **Note**: OpenCode modes are DEPRECATED. Calvin does NOT support mode generation.
> All policies compile to `AGENTS.md` only.

### 5.4 Skill → `.opencode/skill/<id>/SKILL.md`

**Calvin Input** (`.promptpack/skills/git-release/SKILL.md`):

```markdown
---
name: git-release
description: Create consistent releases and changelogs
allowed-tools:
  - git
  - cat
---

## What I do

- Draft release notes from merged PRs
- Propose a version bump
- Provide a copy-pasteable `gh release create` command
```

**OpenCode Output** (`.opencode/skill/git-release/SKILL.md`):

```markdown
---
name: git-release
description: Create consistent releases and changelogs
license: MIT
compatibility: opencode
---

## What I do

- Draft release notes from merged PRs
- Propose a version bump
- Provide a copy-pasteable `gh release create` command

<!-- Generated by Calvin. Source: .promptpack/skills/git-release/SKILL.md. DO NOT EDIT. -->
```

**Note**: OpenCode also reads from `.claude/skills/*/SKILL.md`, so Claude Code skill outputs work in OpenCode.

---

## 6. New Frontmatter Fields

To support OpenCode-specific features, Calvin should accept these optional fields:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mode` | `"primary"` \| `"subagent"` | `"subagent"` | Agent execution mode |
| `temperature` | `float` (0.0-1.0) | None | Model temperature |
| `opencode-model` | `string` | None | Full OpenCode model ID override |
| `agent` | `string` | None | For commands: which agent to use |
| `subtask` | `boolean` | `true` (if subagent) | For commands: run in isolated session |

**Example (Agent)**:

```yaml
---
description: Code reviewer agent
kind: agent
targets: [opencode]
mode: subagent
temperature: 0.3
tools:
  - Read
  - Grep
  - Bash
---
```

**Example (Command/Action)**:

```yaml
---
description: Run tests for a module
kind: action
targets: [opencode]
agent: build
subtask: false
---
Run tests for $ARGUMENTS
```

---

## 7. Proposed Design

### 7.1 Target Enum Extension

```rust
// src/domain/value_objects/target.rs
pub enum Target {
    ClaudeCode,
    Cursor,
    VSCode,
    Antigravity,
    Codex,
    OpenCode,  // NEW
    All,
}

impl Target {
    pub const ALL_CONCRETE: [Target; 6] = [
        Target::ClaudeCode,
        Target::Cursor,
        Target::VSCode,
        Target::Antigravity,
        Target::Codex,
        Target::OpenCode,  // NEW
    ];
}
```

### 7.2 OpenCode Adapter Structure

```rust
// src/infrastructure/adapters/opencode.rs

pub struct OpenCodeAdapter;

impl OpenCodeAdapter {
    fn agents_dir(&self, scope: Scope) -> PathBuf {
        match scope {
            Scope::User => PathBuf::from("~/.config/opencode/agent"),
            Scope::Project => PathBuf::from(".opencode/agent"),
        }
    }

    fn commands_dir(&self, scope: Scope) -> PathBuf {
        match scope {
            Scope::User => PathBuf::from("~/.config/opencode/command"),
            Scope::Project => PathBuf::from(".opencode/command"),
        }
    }

    fn modes_dir(&self, scope: Scope) -> PathBuf {
        match scope {
            Scope::User => PathBuf::from("~/.config/opencode/mode"),
            Scope::Project => PathBuf::from(".opencode/mode"),
        }
    }

    fn skills_dir(&self, scope: Scope) -> PathBuf {
        match scope {
            Scope::User => PathBuf::from("~/.config/opencode/skill"),
            Scope::Project => PathBuf::from(".opencode/skill"),
        }
    }
}

impl TargetAdapter for OpenCodeAdapter {
    fn target(&self) -> Target {
        Target::OpenCode
    }

    fn compile(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError> {
        match asset.kind() {
            AssetKind::Agent => self.compile_agent(asset),
            AssetKind::Action => self.compile_command(asset),  // Action → Command
            AssetKind::Policy => self.compile_mode(asset),     // Policy → Mode
            AssetKind::Skill => self.compile_skill(asset),
        }
    }
}
```

### 7.3 OpenCode Agent Frontmatter Struct

```rust
#[derive(serde::Serialize)]
pub(crate) struct OpenCodeAgentFrontmatter {
    pub description: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,  // "primary" | "subagent"
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    
    #[serde(skip_serializing_if = "is_all_default")]
    pub tools: OpenCodeTools,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<OpenCodePermission>,
}

#[derive(serde::Serialize, Default)]
pub(crate) struct OpenCodeTools {
    #[serde(skip_serializing_if = "is_false")]
    pub read: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub write: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub edit: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub bash: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub grep: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub glob: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub webfetch: bool,
}
```

### 7.4 OpenCode Command Frontmatter

```rust
#[derive(serde::Serialize)]
pub(crate) struct OpenCodeCommandFrontmatter {
    pub description: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,  // Which agent to execute the command
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtask: Option<bool>,  // Run in isolated session (defaults to true for subagents)
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}
```

---

## 8. Implementation Plan

### Phase 1: Core Adapter (MVP) - 2 days

1. Add `Target::OpenCode` variant and update `ALL_CONCRETE`
2. Create basic `OpenCodeAdapter` structure
3. Implement path resolution for all directories
4. Add basic agent compilation (passthrough)
5. Add unit tests

### Phase 2: Agent Format Conversion - 2 days

1. Implement tool format conversion (list → object)
2. Implement model name conversion
3. Implement permission mode → permission object
4. Add `mode` and `temperature` to Asset/Frontmatter
5. Add contract tests

### Phase 3: Commands (Actions) - 1 day

1. Implement action → command compilation
2. Handle `$ARGUMENTS` placeholder
3. Add `agent` field to frontmatter
4. Add command tests

### Phase 4: Modes (Policies) - 1 day

1. Implement policy → mode compilation
2. Add tool restrictions based on policy content
3. Optionally generate `AGENTS.md` aggregation
4. Add mode tests

### Phase 5: Skills Support - 1 day

1. Implement skill compilation to `.opencode/skill/`
2. Handle supplemental files
3. Add `compatibility` and `license` fields
4. Add skill tests

### Phase 6: Documentation - 1 day

1. Update `docs/target-platforms.md`
2. Update `docs-content/docs/api/frontmatter.mdx`
3. Add OpenCode guide to `docs/guides/`

**Total Effort**: ~9 days

---

## 9. Design Decisions

### D1: Policy Handling

**Decision**: Compile to `AGENTS.md` only.

- Modes are DEPRECATED → Calvin does NOT generate `.opencode/mode/` files
- If `AGENTS.md` already exists → **SKIP** (do not overwrite)
- This follows Calvin's default behavior for existing files

### D2: JSON Config Output

**Decision**: No JSON config generation.

- Calvin outputs Markdown files only
- Does NOT generate `opencode.json` or `instructions` configuration
- Users configure OpenCode manually if needed

### D3: Skills Path Strategy

**Decision**: Path depends on enabled targets.

| Scenario | Skills Output Path |
|----------|-------------------|
| `targets: [claude-code, opencode]` | `.claude/skills/<id>/SKILL.md` (reuse, no duplication) |
| `targets: [opencode]` only | `.opencode/skill/<id>/SKILL.md` |

OpenCode reads both paths natively.

### D4: Permission vs Tools Format

**Decision**: Output `permission` object only (modern format).

- `tools` config is DEPRECATED since v1.1.1
- Calvin outputs `permission` object only
- Users on older OpenCode should upgrade

### D5: Command Agent Field

**Decision**: Omit if not specified.

- If Calvin action doesn't specify `agent` → omit field entirely
- OpenCode will use its default agent
- No error for missing agent specification

---

## 10. Error Handling & Edge Cases

### 10.1 Unknown Tool Names

**Behavior**: Emit warning, do not fail compilation.

```
Warning: Unknown tool 'CustomTool' for OpenCode target.
  Valid tools: read, write, edit, bash, grep, glob, webfetch
  Hint: This tool will be omitted from the OpenCode output.
```

### 10.2 Invalid Model Names

**Behavior**: Pass through unknown model names with warning.

```
Warning: Unknown model 'gpt-4' for OpenCode target.
  Hint: Using model name as-is. Ensure it's a valid OpenCode provider/model ID.
```

### 10.3 Permission Format Conversion Failures

**Behavior**: Emit warning when `permission-mode` cannot be cleanly converted.

```
Warning: permission-mode 'ignore' has no OpenCode equivalent.
  Hint: Permission settings will be omitted.
```

### 10.4 Concurrent Claude Code + OpenCode Deployment

When `targets: [claude-code, opencode]` or `targets: all`:

1. **Agents**: Deploy to BOTH `.claude/agents/` AND `.opencode/agent/` with format conversion
2. **Actions/Commands**: Deploy to BOTH `.claude/commands/` AND `.opencode/command/`
3. **Skills**: Deploy to `.claude/skills/` ONLY (OpenCode reads from there)
4. **Policies**: Claude Code writes `CLAUDE.md`, OpenCode writes `.opencode/mode/`

**Test Contract**: `contract_claude_opencode_agent_different_format`

### 10.5 Existing Modified Files

When a target file exists and was manually modified:

**Behavior**: Check hash in lockfile. If mismatch:

```
Warning: .opencode/agent/reviewer.md has been manually modified.
  Use --force to overwrite, or remove from lockfile.
```

### 10.6 OpenCode Version Compatibility

**Minimum Version**: OpenCode 0.1.0

If user's OpenCode version is detected (via `opencode --version`) and is older:

```
Warning: OpenCode 0.0.x may not support all generated features.
  Hint: Upgrade to OpenCode 0.1.0+ for full compatibility.
```

---

## 11. Test Contracts

Test contracts define the **mandatory behavior** that must pass before the feature is considered complete. These contracts are written BEFORE implementation (TDD).

### 11.1 Path Contracts

| Contract | Input | Expected Output Path |
|----------|-------|---------------------|
| `contract_opencode_agent_project_path` | Agent, project scope | `.opencode/agent/<id>.md` |
| `contract_opencode_agent_user_path` | Agent, user scope | `~/.config/opencode/agent/<id>.md` |
| `contract_opencode_command_project_path` | Action, project scope | `.opencode/command/<id>.md` |
| `contract_opencode_command_user_path` | Action, user scope | `~/.config/opencode/command/<id>.md` |
| `contract_opencode_mode_project_path` | Policy, project scope | `.opencode/mode/<id>.md` |
| `contract_opencode_skill_project_path` | Skill, project scope | `.opencode/skill/<id>/SKILL.md` |

### 11.2 Format Conversion Contracts

| Contract | Assertion |
|----------|-----------|
| `contract_opencode_tools_object_format` | Output MUST have `tools: { read: true }` NOT `tools: Read` |
| `contract_opencode_tools_empty_omitted` | When no tools specified, `tools:` MUST be omitted |
| `contract_opencode_model_passthrough` | `model: sonnet` → `model: sonnet` (unchanged, with warning if unknown) |
| `contract_opencode_model_passthrough` | Full model IDs MUST pass through unchanged |
| `contract_opencode_mode_field_present` | Agents MUST have `mode:` field (default: `subagent`) |
| `contract_opencode_temperature_preserved` | `temperature: 0.3` MUST appear in output |
| `contract_opencode_permission_conversion` | `permission-mode: acceptEdits` → `permission: { edit: allow }` |

### 11.3 Asset Mapping Contracts

| Contract | Assertion |
|----------|-----------|
| `contract_opencode_action_becomes_command` | `kind: action` → `.opencode/command/` |
| `contract_opencode_policy_becomes_mode` | `kind: policy` → `.opencode/mode/` |
| `contract_opencode_command_preserves_arguments` | `$ARGUMENTS` placeholder MUST be preserved |
| `contract_opencode_command_has_description` | Output MUST have `description:` in frontmatter |
| `contract_opencode_mode_disables_tools` | Read-only policies → `tools: { write: false, edit: false }` |

### 11.4 Skill Contracts

| Contract | Assertion |
|----------|-----------|
| `contract_opencode_skill_path_singular` | Path MUST be `.opencode/skill/` NOT `.opencode/skills/` |
| `contract_opencode_skill_has_name` | Output MUST have `name:` matching directory |
| `contract_opencode_skill_has_description` | Output MUST have `description:` (min 20 chars) |
| `contract_opencode_skill_supplementals_copied` | `scripts/`, `references/` MUST be copied |

### 11.5 Error Handling Contracts

| Contract | Assertion |
|----------|-----------|
| `contract_opencode_unknown_tool_warning` | Unknown tool name → Warning, not error |
| `contract_opencode_invalid_model_warning` | Unknown model → Warning, pass through |
| `contract_opencode_empty_description_error` | Empty description → Error |
| `contract_opencode_version_incompatible_warning` | OpenCode < 0.1.0 → Warning about compatibility |
| `contract_opencode_existing_file_modified_warning` | Existing file with manual edits → Warning before overwrite |

### 11.6 Cross-Platform Contracts

| Contract | Assertion |
|----------|-----------|
| `contract_claude_and_opencode_no_duplicate_skills` | When both targets enabled, skill deploys to both ONLY if `targets: [opencode, claude-code]` |
| `contract_opencode_skill_reads_claude_format` | Info message: "OpenCode also reads .claude/skills/" when only claude-code enabled |
| `contract_action_to_command_not_action` | Action assets → `.opencode/command/`, never `.opencode/action/` |
| `contract_claude_opencode_agent_different_format` | Same agent deployed to both targets has different frontmatter format |

---

## 12. TDD Implementation Plan

Following Calvin's TDD workflow, each phase starts with **writing tests first**.

### Phase 1: Core Adapter (TDD)

**Step 1: Write failing tests**

```rust
// tests/contracts/opencode.rs

/// CONTRACT: OpenCode agent project path
#[test]
fn contract_opencode_agent_project_path() {
    let env = TestEnv::builder().build();
    env.write_project_file(
        ".promptpack/agents/reviewer.md",
        r#"---
description: Code reviewer
targets: [opencode]
---
Content
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "opencode"]);
    assert!(result.success);

    assert!(
        env.project_path(".opencode/agent/reviewer.md").exists(),
        "CONTRACT: Agent MUST be deployed to .opencode/agent/"
    );
}

/// CONTRACT: OpenCode agent user scope path
#[test]
fn contract_opencode_agent_user_path() {
    let env = TestEnv::builder().build();
    env.write_project_file(
        ".promptpack/agents/global.md",
        r#"---
description: Global agent
scope: user
targets: [opencode]
---
Content
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "opencode"]);
    assert!(result.success);

    let home_path = env.home_path(".config/opencode/agent/global.md");
    assert!(
        home_path.exists(),
        "CONTRACT: User-scope agent MUST be at ~/.config/opencode/agent/"
    );
}
```

**Step 2: Run tests (expect FAIL)**

```bash
cargo test contract_opencode -- --nocapture
# Expected: FAIL - Target::OpenCode not found
```

**Step 3: Implement minimum code**

1. Add `Target::OpenCode` to enum
2. Create `OpenCodeAdapter` skeleton
3. Implement `agents_dir()` path resolution
4. Run tests again

**Step 4: Refactor if needed**

### Phase 2: Agent Format Conversion (TDD)

**Step 1: Write failing tests**

```rust
/// CONTRACT: Tools must be object format, not string
#[test]
fn contract_opencode_tools_object_format() {
    let env = TestEnv::builder().build();
    env.write_project_file(
        ".promptpack/agents/test.md",
        r#"---
description: Test agent
targets: [opencode]
tools:
  - Read
  - Bash
---
Content
"#,
    );

    env.run(&["deploy", "--yes", "--targets", "opencode"]);

    let content = env.read_deployed_file(".opencode/agent/test.md");

    // MUST be object format
    assert!(
        content.contains("read: true"),
        "CONTRACT: tools MUST be object format with 'read: true'"
    );
    assert!(
        content.contains("bash: true"),
        "CONTRACT: tools MUST be object format with 'bash: true'"
    );

    // MUST NOT be string format
    assert!(
        !content.contains("tools: Read"),
        "CONTRACT: tools MUST NOT be comma-separated string"
    );
}

/// CONTRACT: Model names must be converted to full IDs
#[test]
fn contract_opencode_model_full_id() {
    let env = TestEnv::builder().build();
    env.write_project_file(
        ".promptpack/agents/test.md",
        r#"---
description: Test agent
targets: [opencode]
model: sonnet
---
Content
"#,
    );

    env.run(&["deploy", "--yes", "--targets", "opencode"]);

    let content = env.read_deployed_file(".opencode/agent/test.md");

    assert!(
        content.contains("anthropic/claude-sonnet-4-20250514"),
        "CONTRACT: 'sonnet' MUST be converted to full model ID"
    );
    assert!(
        !content.contains("model: sonnet"),
        "CONTRACT: Short model name MUST NOT appear in output"
    );
}

/// CONTRACT: Mode field must be present
#[test]
fn contract_opencode_mode_field_present() {
    let env = TestEnv::builder().build();
    env.write_project_file(
        ".promptpack/agents/test.md",
        r#"---
description: Test agent
targets: [opencode]
---
Content
"#,
    );

    env.run(&["deploy", "--yes", "--targets", "opencode"]);

    let content = env.read_deployed_file(".opencode/agent/test.md");

    assert!(
        content.contains("mode: subagent") || content.contains("mode: primary"),
        "CONTRACT: Agent MUST have 'mode:' field"
    );
}
```

**Step 2: Run tests, expect FAIL**

**Step 3: Implement conversions**

### Phase 3: Commands (TDD)

**Step 1: Write failing tests**

```rust
/// CONTRACT: Action becomes command
#[test]
fn contract_opencode_action_becomes_command() {
    let env = TestEnv::builder().build();
    env.write_project_file(
        ".promptpack/actions/deploy.md",
        r#"---
description: Deploy application
targets: [opencode]
---
Deploy $ARGUMENTS to production.
"#,
    );

    env.run(&["deploy", "--yes", "--targets", "opencode"]);

    // MUST be in command directory
    assert!(
        env.project_path(".opencode/command/deploy.md").exists(),
        "CONTRACT: Action MUST be compiled to .opencode/command/"
    );

    // MUST NOT be in agent directory
    assert!(
        !env.project_path(".opencode/agent/deploy.md").exists(),
        "CONTRACT: Action MUST NOT be in agent directory"
    );
}

/// CONTRACT: Command preserves $ARGUMENTS
#[test]
fn contract_opencode_command_preserves_arguments() {
    let env = TestEnv::builder().build();
    env.write_project_file(
        ".promptpack/actions/test.md",
        r#"---
description: Test action
targets: [opencode]
---
Run tests for $ARGUMENTS
"#,
    );

    env.run(&["deploy", "--yes", "--targets", "opencode"]);

    let content = env.read_deployed_file(".opencode/command/test.md");

    assert!(
        content.contains("$ARGUMENTS"),
        "CONTRACT: $ARGUMENTS placeholder MUST be preserved"
    );
}
```

### Phase 4: Modes (TDD)

```rust
/// CONTRACT: Policy becomes mode
#[test]
fn contract_opencode_policy_becomes_mode() {
    let env = TestEnv::builder().build();
    env.write_project_file(
        ".promptpack/policies/security.md",
        r#"---
description: Security guidelines
targets: [opencode]
---
Never expose secrets.
"#,
    );

    env.run(&["deploy", "--yes", "--targets", "opencode"]);

    assert!(
        env.project_path(".opencode/mode/security.md").exists(),
        "CONTRACT: Policy MUST be compiled to .opencode/mode/"
    );
}
```

### Phase 5: Skills (TDD)

```rust
/// CONTRACT: Skill path uses singular 'skill'
#[test]
fn contract_opencode_skill_path_singular() {
    let env = TestEnv::builder().build();
    env.write_skill_file(
        "git-release",
        r#"---
name: git-release
description: Create releases and changelogs
---
# Git Release
"#,
    );

    env.run(&["deploy", "--yes", "--targets", "opencode"]);

    // MUST use singular 'skill'
    assert!(
        env.project_path(".opencode/skill/git-release/SKILL.md").exists(),
        "CONTRACT: Skill path MUST be .opencode/skill/ (singular)"
    );

    // MUST NOT use plural 'skills'
    assert!(
        !env.project_path(".opencode/skills/git-release/SKILL.md").exists(),
        "CONTRACT: Skill path MUST NOT be .opencode/skills/ (plural)"
    );
}
```

---

## 13. Acceptance Criteria

Each phase has explicit acceptance criteria:

### Phase 1 Complete When:

- [ ] `Target::OpenCode` variant exists
- [ ] `OpenCodeAdapter` implements `TargetAdapter`
- [ ] All path contracts pass
- [ ] `cargo test contract_opencode_.*_path` → 6/6 PASS

### Phase 2 Complete When:

- [ ] Tools conversion implemented
- [ ] Model mapping implemented
- [ ] `mode` field always present
- [ ] `cargo test contract_opencode_tools` → PASS
- [ ] `cargo test contract_opencode_model` → PASS
- [ ] `cargo test contract_opencode_mode` → PASS

### Phase 3 Complete When:

- [ ] Action → Command mapping works
- [ ] `$ARGUMENTS` preserved
- [ ] `cargo test contract_opencode_action` → PASS
- [ ] `cargo test contract_opencode_command` → PASS

### Phase 4 Complete When:

- [ ] Policy → Mode mapping works
- [ ] Tool restrictions applied
- [ ] `cargo test contract_opencode_policy` → PASS
- [ ] `cargo test contract_opencode_mode` → PASS

### Phase 5 Complete When:

- [ ] Skill compilation works
- [ ] Supplemental files copied
- [ ] Path is `.opencode/skill/` (singular)
- [ ] `cargo test contract_opencode_skill` → PASS

### Full Feature Complete When:

- [ ] All 20+ contract tests pass
- [ ] `cargo test opencode` → 100% PASS
- [ ] Documentation updated
- [ ] `cargo clippy` → no warnings

---

---

## 14. Rollout Plan

1. **Alpha (0.9.0-alpha)**: Add adapter with "experimental" flag
2. **Beta (0.9.0)**: Stabilize based on community feedback
3. **GA (0.10.0)**: Remove "experimental" flag, full documentation

---

## 15. References

- OpenCode Official: https://opencode.ai
- OpenCode GitHub: https://github.com/sst/opencode
- OpenCode Config Docs: https://opencode.ai/docs/config/
- OpenCode Agents Docs: https://opencode.ai/docs/agents/
- OpenCode Commands Docs: https://opencode.ai/docs/commands/
- OpenCode Modes Docs: https://opencode.ai/docs/modes/
- OpenCode Skills Docs: https://opencode.ai/docs/skills/
- OpenCode Plugins Docs: https://opencode.ai/docs/plugins/

---

## Appendix A: Compatibility Matrix Update

After implementation:

| Feature | Claude Code | Cursor | VS Code | Antigravity | Codex | OpenCode |
|---------|-------------|--------|---------|-------------|-------|----------|
| Agents (native) | ✅ | ⚠️¹ | ⚠️² | ⚠️³ | ⚠️³ | ✅ |
| Commands | ✅ | ✅ | ❌ | ⚠️ | ⚠️ | ✅ |
| Modes | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| Skills | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ |
| Policies | ✅ | ✅ | ✅ | ✅ | ⚠️ | ✅⁴ |

**Notes**:
1. Cursor: Falls back to commands
2. VS Code: Falls back to instructions
3. Antigravity/Codex: Falls back to workflows/prompts
4. OpenCode: Compiled to `AGENTS.md` (recommended) or modes (deprecated)

---

## Appendix B: Directory Structure Summary

| Calvin Asset | OpenCode Output | Notes |
|--------------|-----------------|-------|
| `agents/<id>.md` | `.opencode/agent/<id>.md` | Native agent format |
| `actions/<id>.md` | `.opencode/command/<id>.md` | Becomes slash command |
| `policies/<id>.md` | `AGENTS.md` (aggregated) | Skip if exists |
| `skills/<id>/SKILL.md` | `.claude/skills/<id>/SKILL.md` | Reuse Claude output (OpenCode reads it) |
| `memory/*.md` | `AGENTS.md` (appended) | Project context |

**Notes**:
- Modes NOT supported (deprecated)
- Skills path depends on targets:
  - Both targets → `.claude/skills/` (reuse, deduplication)
  - OpenCode only → `.opencode/skill/`

---

## Appendix C: Detailed File Format Examples

### C.1 Agent File Format

**Calvin Input** (`.promptpack/agents/reviewer.md`):

```yaml
---
description: Reviews code for best practices
kind: agent
scope: project
targets: [opencode]
tools:
  - Read
  - Grep
model: sonnet
permission-mode: acceptEdits
temperature: 0.3
---
Review code changes focusing on:
- Security issues
- Performance problems
```

**OpenCode Output** (`.opencode/agent/reviewer.md`):

```yaml
---
description: Reviews code for best practices
mode: subagent
model: sonnet
temperature: 0.3
tools:
  read: true
  grep: true
  write: false
  edit: false
  bash: false
permission:
  edit: allow
---
Review code changes focusing on:
- Security issues
- Performance problems

<!-- Generated by Calvin. Source: .promptpack/agents/reviewer.md. DO NOT EDIT. -->
```

### C.2 Action/Command File Format

**Calvin Input** (`.promptpack/actions/test.md`):

```yaml
---
description: Generate tests for a module
kind: action
scope: project
targets: [opencode]
---
Create comprehensive tests for $ARGUMENTS
```

**OpenCode Output** (`.opencode/command/test.md`):

```yaml
---
description: Generate tests for a module
subtask: true
---
Create comprehensive tests for $ARGUMENTS

<!-- Generated by Calvin. Source: .promptpack/actions/test.md. DO NOT EDIT. -->
```

**Usage**: `/test utils/parser`

### C.3 Policy File Format (AGENTS.md)

**Calvin Input** (multiple files):

- `.promptpack/policies/code-style.md`
- `.promptpack/policies/security.md`

**OpenCode Output** (`AGENTS.md` at project root):

```markdown
# Project Guidelines

<!-- Generated by Calvin. DO NOT EDIT. -->

## Code Style

Source: `.promptpack/policies/code-style.md`

Follow TypeScript best practices...

---

## Security Policy

Source: `.promptpack/policies/security.md`

Never commit secrets to repository...
```

### C.4 Skill File Format

**Calvin Input** (`.promptpack/skills/git-release/SKILL.md`):

```yaml
---
name: git-release
description: Creates releases with changelogs
---
## What I do

Draft release notes from merged PRs...
```

**OpenCode Output** (`.opencode/skill/git-release/SKILL.md`):

```yaml
---
name: git-release
description: Creates releases with changelogs
license: MIT
compatibility: opencode
---
## What I do

Draft release notes from merged PRs...

<!-- Generated by Calvin. Source: .promptpack/skills/git-release/SKILL.md. DO NOT EDIT. -->
```

### C.5 Frontmatter Field Transformation Summary

| Calvin Field | OpenCode Field | Transformation |
|--------------|----------------|----------------|
| `description` | `description` | Pass through |
| `kind: agent` | (implied by path) | `.opencode/agent/` |
| `kind: action` | (implied by path) | `.opencode/command/` |
| `kind: policy` | (aggregated) | `AGENTS.md` |
| `kind: skill` | (implied by path) | `.opencode/skill/` |
| `scope: project` | (project path) | `.opencode/...` |
| `scope: user` | (user path) | `~/.config/opencode/...` |
| `targets` | (stripped) | Used for routing only |
| `tools: [Read, Bash]` | `tools: { read: true, bash: true }` | List → Object |
| `model: <value>` | `model: <value>` | Pass through unchanged (validate only) |
| `permission-mode` | `permission` | See §4.5 |
| `temperature` | `temperature` | Pass through |
| `mode` | `mode` | Pass through (`primary`/`subagent`) |
| `agent` | `agent` | For commands only |
| `subtask` | `subtask` | For commands only |
---