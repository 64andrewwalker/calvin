# Skills Support PRD

> **Status**: Draft  
> **Author**: Arona (AI Assistant)  
> **Created**: 2025-12-27  
> **Target Version**: 0.6.0

---

## Executive Summary

This PRD proposes introducing **Skills** support to Calvin. Skills are a directory-based asset type that compiles to `SKILL.md` format for Claude Code, Codex, and Cursor (which uses Claude's skill paths).

**Important**: Antigravity and VS Code do **not** support skills. Calvin will not compile skills for these platforms.

This is a **significant extension**, not a minor feature. Skills differ from existing asset kinds in structure (directory vs. single file), content (supports supplemental files), and semantics (semantic activation vs. explicit invocation).

**Decision**: Skills support is approved. The goal is to let users manage skills in `.promptpack/` and distribute them to different projects/user home through Calvin.

---

## 1. Problem Statement

### Current State

Calvin currently supports three `AssetKind` values:

| Kind | Purpose | Output | Example |
|------|---------|--------|---------|
| `policy` | Long-term rules, always active | Rules/instructions | Code style, security policy |
| `action` | Triggered commands | Slash commands | Generate tests, PR review |
| `agent` | Specialized sub-personas | Agent files | Code reviewer, security auditor |

All assets are **single Markdown files** with YAML frontmatter.

### Industry Trend: Skills

Several platforms now support a **Skills** format:

| Platform | Format | Location | Status |
|----------|--------|----------|--------|
| **Claude Code** | `SKILL.md` folder | `.claude/skills/<id>/` | Production |
| **Codex (OpenAI)** | `SKILL.md` folder | `.codex/skills/<id>/` | Production |
| **Cursor** | `SKILL.md` folder | `.claude/skills/<id>/` | Production |
| **Antigravity** | Workflows | `.agent/workflows/` | Conceptually similar |

**Skills differ from Actions/Commands in critical ways**:

1. **Semantic Activation**: AI decides when to use a skill based on `description`
2. **Progressive Disclosure**: Only `SKILL.md` is loaded initially; details in separate files
3. **Executable Scripts**: Skills can include helper scripts the AI can run
4. **Directory Structure**: A skill is a folder, not a single file

### The Question

Should Calvin support skills as a new asset kind, or are existing kinds sufficient?

**Arguments FOR skills support**:

- Industry convergence on `SKILL.md` format
- Users with existing skills want to manage them through Calvin
- Skills offer capabilities (scripts, multi-file) that Actions don't

**Arguments AGAINST skills support**:

- Adds significant complexity (directory assets, supplemental files)
- Actions already serve the "triggerable automation" use case
- Calvin's value is in unification, not feature parity with every platform

---

## 2. Goals

### Primary Goals

1. Add `skill` as a new `AssetKind`
2. Support directory-based assets (`skills/<id>/SKILL.md`)
3. Compile skills to platform-specific formats
4. Integrate with multi-layer system (user/team/project)
5. Track skill outputs in lockfile

### Non-Goals

- **NOT** implementing skill runtime/execution (platform responsibility)
- **NOT** implementing skill versioning or dependency management
- **NOT** implementing skill marketplace/discovery
- **NOT** supporting binary/compiled scripts (text files only)
- **NOT** implementing MCP integration in skills (deferred)

### Success Criteria

1. User can create skill in `.promptpack/skills/my-skill/SKILL.md`
2. `calvin deploy` compiles skill to appropriate platform directories
3. Skill supplemental files (`reference.md`, `scripts/`) are included
4. Skills respect multi-layer override semantics (project > team > user)
5. `calvin clean` removes skill directories correctly

---

## 3. Design Philosophy

### Principle 1: Skills Are Directories, Not Files

Unlike `policy`/`action`/`agent` (single files), skills are **directory structures**:

```text
.promptpack/
├── policies/
│   └── code-style.md           # Single file asset (id: code-style)
├── actions/
│   ├── review.md               # Single file asset (id: review)
│   └── deploy.md               # Single file asset (id: deploy)
├── agents/
│   └── security-auditor.md     # Single file asset (id: security-auditor)
└── skills/
    ├── review/                 # Directory asset (id: review) - SAME NAME AS ACTION, NO CONFLICT
    │   ├── SKILL.md            # Entry point (required)
    │   └── checklist.md        # Supplemental file
    └── draft-commit-message/   # Directory asset (id: draft-commit-message)
        ├── SKILL.md            # Entry point (required)
        ├── reference.md        # Optional: detailed docs
        └── scripts/            # Optional: helper scripts
            └── validate.py
```

**Note**: Skills and actions can have the same name (e.g., `review`) without conflict because they live in different directories. IDs are scoped by their parent folder type.

This is a **structural break** from existing assets. The repository must detect and load directory assets differently.

### Principle 2: Minimal Frontmatter Extension

Following Calvin's philosophy of minimal required fields, skills add only **one new optional field**:

```yaml
---
description: Draft a conventional commit message.  # Required (existing)
kind: skill                                         # Required (new value)
scope: user                                         # Optional (existing)
targets:                                            # Optional (existing)
  - claude-code
  - codex
allowed-tools:                                      # NEW: Optional, skill-only
  - git
  - cat
---
```

The `allowed-tools` field is validated to warn on dangerous tools.

### Principle 3: Complete Override, Not Merge

Following the multi-layer PRD's merge semantics (§4.2):

> **相同 `id` 的 asset**：高层级完全覆盖低层级（不做字段级合并）

If project layer has `skills/my-skill/`, it **completely replaces** user layer's `skills/my-skill/`. Supplemental files are also replaced, not merged. But warn user about this. DO NOT SILENTLY OVERWRITE.

### Principle 4: No Fallback Compilation

Calvin does **NOT** attempt to "fake" skill support on platforms that don't have it:

| Platform | Skill Support | Behavior |
|----------|---------------|----------|
| Claude Code | Native `SKILL.md` | ✓ Compile skill |
| Codex | Native `SKILL.md` | ✓ Compile skill |
| Cursor | Uses Claude's `.claude/skills/` | ✓ Compile skill (same as Claude Code) |
| Antigravity | **Not supported** | ✗ Skip skill |
| VS Code | **Not supported** | ✗ Skip skill |

**Rationale**: Fallback compilation creates false expectations. If a skill has scripts or supplemental files, a "flattened" version loses critical functionality. Better to be honest: "This platform doesn't support skills."

### Principle 5: Never Fail Silently

Following Calvin's non-silent failure policy (multi-layer PRD §13.5):

| Situation | Behavior |
|-----------|----------|
| `skills/foo/` exists but no `SKILL.md` | **Error**: "Skill directory 'foo' missing SKILL.md" |
| `SKILL.md` has `allowed-tools` with dangerous tool | **Warning** + continue |
| Skill targets `antigravity` or `vscode` | **Warning**: "Skills not supported on [platform], skipping" |
| Skill has no supported targets | **Error**: "Skill 'foo' has no supported targets" |
| Supplemental file is binary | **Error**: "Binary files not supported" |

**Validation Layer Assignments** (I3 - Confirmed):

| Validation | Layer | Location | Rationale |
| ---------- | ----- | -------- | --------- |
| Missing SKILL.md | **Repository** | `FsAssetRepository::load_skills()` | Fail fast during asset loading |
| Binary file detection | **Repository** | `FsAssetRepository::load_supplemental()` | Part of loading, not compilation |
| Dangerous tools warning | **Adapter** | `TargetAdapter::validate()` | Each adapter can have different rules |
| Unsupported platform skip | **Adapter** | `TargetAdapter::compile()` | Return `vec![]` when `kind == Skill` |
| No supported targets | **CompilerService** | After all adapters return | Check if all outputs are empty |

---

## 4. Detailed Design

### 4.1 Asset Loading Changes

**Current flow** (`FsAssetRepository::load_all`):

1. Scan `.promptpack/` for `*.md` files
2. Parse each file's frontmatter
3. Create `Asset` from parsed data

**New flow** (with skills):

1. Scan `.promptpack/` for `*.md` files (existing behavior)
2. **NEW**: Scan `.promptpack/skills/` for directories
3. For each skill directory:
   a. Require `SKILL.md` exists
   b. Parse `SKILL.md` frontmatter
   c. Collect supplemental files (non-binary text files)
   d. Create extended `Asset` with supplementals
4. Merge all assets

**Decision**: Extend `Asset` (Option A)

- Add `supplementals: HashMap<PathBuf, String>` field
- Add `allowed_tools: Vec<String>` field
- Existing adapters ignore these fields for non-skill assets
- Simpler: one type flows through the pipeline

### 4.2 Domain Entity Changes

```rust
// domain/entities/asset.rs

pub enum AssetKind {
    Policy,
    Action,
    Agent,
    Skill,  // NEW
}

pub struct Asset {
    // Existing fields
    id: String,
    source_path: PathBuf,
    description: String,
    kind: AssetKind,
    scope: Scope,
    targets: Vec<Target>,
    content: String,
    apply: Option<String>,
    
    // NEW: Skill-specific (empty for non-skills)
    supplementals: HashMap<PathBuf, String>,
    allowed_tools: Vec<String>,
}
```

### 4.3 Frontmatter Schema Extension

```rust
// models.rs

pub struct Frontmatter {
    pub description: String,
    #[serde(default)]
    pub kind: AssetKind,
    #[serde(default)]
    pub scope: Scope,
    #[serde(default)]
    pub targets: Vec<Target>,
    #[serde(default)]
    pub apply: Option<String>,
    
    // NEW: Skill-specific
    #[serde(default, rename = "allowed-tools")]
    pub allowed_tools: Vec<String>,
}
```

### 4.4 Output Path Matrix

| Platform | Kind | Scope | Output Path |
|----------|------|-------|-------------|
| **Claude Code** | Skill | Project | `.claude/skills/<id>/SKILL.md` |
| **Claude Code** | Skill | User | `~/.claude/skills/<id>/SKILL.md` |
| **Codex** | Skill | Project | `.codex/skills/<id>/SKILL.md` |
| **Codex** | Skill | User | `~/.codex/skills/<id>/SKILL.md` |
| **Cursor** | Skill | Project | `.claude/skills/<id>/SKILL.md` (shares Claude Code path) |
| **Cursor** | Skill | User | `~/.claude/skills/<id>/SKILL.md` (shares Claude Code path) |
| **Antigravity** | Skill | - | ❌ Not supported |
| **VS Code** | Skill | - | ❌ Not supported |

**Note**: Cursor uses Claude Code's skill paths. This is intentional - Cursor reads from `.claude/skills/`. Antigravity and VS Code do not support skills; assets with `kind: skill` targeting these platforms are skipped with a warning.

### 4.5 Adapter Changes

#### Claude Code Adapter

```rust
impl TargetAdapter for ClaudeCodeAdapter {
    fn compile(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError> {
        match asset.kind() {
            AssetKind::Skill => self.compile_skill(asset),
            _ => self.compile_standard(asset),
        }
    }
    
    fn compile_skill(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError> {
        let mut outputs = vec![];
        
        let base_dir = match asset.scope() {
            Scope::Project => PathBuf::from(".claude/skills"),
            Scope::User => PathBuf::from("~/.claude/skills"),
        };
        
        let skill_dir = base_dir.join(asset.id());
        
        // 1. Generate SKILL.md
        let skill_content = self.generate_skill_md(asset);
        outputs.push(OutputFile::new(
            skill_dir.join("SKILL.md"),
            skill_content,
            Target::ClaudeCode,
        ));
        
        // 2. Copy supplemental files
        for (rel_path, content) in asset.supplementals() {
            outputs.push(OutputFile::new(
                skill_dir.join(rel_path),
                content.clone(),
                Target::ClaudeCode,
            ));
        }
        
        Ok(outputs)
    }
    
    fn generate_skill_md(&self, asset: &Asset) -> String {
        let mut out = String::new();
        
        // Frontmatter
        out.push_str("---\n");
        out.push_str(&format!("name: {}\n", asset.id()));
        out.push_str(&format!("description: {}\n", asset.description()));
        
        if !asset.allowed_tools().is_empty() {
            out.push_str("allowed-tools:\n");
            for tool in asset.allowed_tools() {
                out.push_str(&format!("  - {}\n", tool));
            }
        }
        out.push_str("---\n\n");
        
        // Content
        out.push_str(asset.content());
        
        // Footer
        out.push_str("\n\n");
        out.push_str(&self.footer(&asset.source_path_normalized()));
        
        out
    }
}
```

#### Cursor Adapter

Cursor shares Claude Code's skill paths:

```rust
impl TargetAdapter for CursorAdapter {
    fn compile_skill(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError> {
        // Cursor uses Claude Code's skill paths
        // This mirrors ClaudeCodeAdapter::compile_skill but with Target::Cursor
        let base_dir = match asset.scope() {
            Scope::Project => PathBuf::from(".claude/skills"),
            Scope::User => PathBuf::from("~/.claude/skills"),
        };
        
        let skill_dir = base_dir.join(asset.id());
        let mut outputs = vec![];
        
        // Same structure as Claude Code
        outputs.push(OutputFile::new(
            skill_dir.join("SKILL.md"),
            self.generate_skill_md(asset),
            Target::Cursor,
        ));
        
        for (rel_path, content) in asset.supplementals() {
            outputs.push(OutputFile::new(
                skill_dir.join(rel_path),
                content.clone(),
                Target::Cursor,
            ));
        }
        
        Ok(outputs)
    }
}
```

#### Antigravity & VS Code Adapters (No Skill Support)

These adapters explicitly reject skills:

```rust
impl TargetAdapter for AntigravityAdapter {
    fn compile(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError> {
        if asset.kind() == AssetKind::Skill {
            // Skills not supported - return empty (warning emitted elsewhere)
            return Ok(vec![]);
        }
        // ... existing compile logic for Policy/Action/Agent
    }
}

impl TargetAdapter for VSCodeAdapter {
    fn compile(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError> {
        if asset.kind() == AssetKind::Skill {
            // Skills not supported - return empty (warning emitted elsewhere)
            return Ok(vec![]);
        }
        // ... existing compile logic for Policy/Action/Agent
    }
}
```

### 4.6 Lockfile Changes

**Decision**: Lockfile tracks skills at the **folder level**, not individual files. The entire skill directory is treated as one unit.

```toml
# ./calvin.lock

version = 1

# Skill tracked at folder level
[files."project:.claude/skills/draft-commit"]
hash = "sha256:combined_hash_of_all_files..."
source_layer = "user"
source_asset = "draft-commit"
source_path = "skills/draft-commit"
kind = "skill"
```

**Rationale**:

- A skill is a cohesive unit; individual files inside don't need separate tracking
- Hash is computed from all files in the skill directory combined
- Simplifies orphan detection (remove entire directory, not individual files)

### 4.7 Orphan Detection

**With folder-level tracking**, orphan detection is simpler:

1. `OrphanDetector` compares lockfile skill entries against new output
2. If a skill ID is in lockfile but not in new output, the entire skill folder is orphaned
3. `calvin deploy --cleanup` removes the entire orphan skill directory

**No need to track individual files** - when a skill changes, Calvin:

1. Deletes the entire old skill directory
2. Writes the entire new skill directory
3. Updates the lockfile with new combined hash

### 4.8 Security Considerations

#### `allowed-tools` Validation

```rust
const DANGEROUS_TOOLS: &[&str] = &[
    "rm", "sudo", "chmod", "chown", "curl", "wget", 
    "nc", "netcat", "ssh", "scp", "rsync"
];

fn validate_allowed_tools(tools: &[String]) -> Vec<AdapterDiagnostic> {
    let mut diags = vec![];
    
    for tool in tools {
        if DANGEROUS_TOOLS.contains(&tool.as_str()) {
            diags.push(AdapterDiagnostic {
                severity: DiagnosticSeverity::Warning,
                message: format!(
                    "Tool '{}' in allowed-tools may pose security risks. \
                     Ensure this is intentional.",
                    tool
                ),
            });
        }
    }
    
    diags
}
```

**Note**: This is a **warning**, not an error. Following Calvin's philosophy (TD-13), we "provide the tools, don't force the religion."

#### Binary File Detection

```rust
fn is_binary(content: &[u8]) -> bool {
    // Simple heuristic: check for null bytes
    content.contains(&0)
}

fn load_supplemental(path: &Path) -> Result<String, LoadError> {
    let bytes = fs::read(path)?;
    
    if is_binary(&bytes) {
        return Err(LoadError::BinaryFile(path.to_path_buf()));
    }
    
    String::from_utf8(bytes)
        .map_err(|_| LoadError::InvalidUtf8(path.to_path_buf()))
}
```

---

## 5. Migration & Compatibility

### 5.1 Backward Compatibility

- Existing `kind: action` and `kind: agent` assets work unchanged
- `allowed-tools` field is ignored for non-skill assets
- Projects without skills see no behavior change

### 5.2 Manual Skill Migration

Users with existing manual skills can copy them to `.promptpack/skills/`:

```bash
# Manual migration (future: `calvin import-skill`)
cp -r ~/.claude/skills/my-skill .promptpack/skills/

# Redeploy
calvin deploy
```

### 5.3 Config Changes

No configuration changes required. Skills use existing `scope` and `targets` fields.

---

## 6. CLI Changes

### 6.1 `calvin deploy`

No new options. Skills are automatically detected in `.promptpack/skills/`.

Verbose output (`-v`) shows skill structure:

```
$ calvin deploy -v

ℹ Layer Stack:
  3. [project] ./.promptpack/ (5 assets, 2 skills)
  2. [user]    ~/.calvin/.promptpack (3 assets, 1 skill)

ℹ Skills:
  • draft-commit-message (user layer)
    ├── SKILL.md
    ├── reference.md
    └── scripts/validate.py

✓ Compiled 8 assets, 3 skills
✓ Deployed to: claude-code, codex, cursor
⚠ Skills skipped for: antigravity, vscode (not supported)
```

### 6.2 `calvin layers`

Updated to show skill counts:

```
$ calvin layers

Layer Stack (priority: high → low)
┌────────────┬───────────────────────────┬─────────┬────────┐
│ Name       │ Path                      │ Assets  │ Skills │
├────────────┼───────────────────────────┼─────────┼────────┤
│ project    │ ./.promptpack             │ 5       │ 2      │
│ user       │ ~/.calvin/.promptpack     │ 3       │ 1      │
└────────────┴───────────────────────────┴─────────┴────────┘
```

### 6.3 `calvin check`

Updated to validate skills:

```
$ calvin check

Claude Code
  ✓ Skills well-formed
  ⚠ Skill 'my-skill' uses 'curl' in allowed-tools (security warning)
```

### 6.4 Skill Management

**Decision**: No separate `calvin skills` command. Skills are managed the same way as actions - through `calvin deploy`, `calvin layers`, and `calvin check`. This keeps the CLI simple and consistent.

---

## 7. Implementation Plan (TDD)

> **TDD Principle**: Write the failing test first, then implement the minimum code to pass it.
>
> Reference: [Testing Philosophy](../testing/TESTING_PHILOSOPHY.md)

### Progress Overview

| Phase | Status | Tests | Implementation |
| ----- | ------ | ----- | -------------- |
| 0. Decision Gate | � Complete | N/A | 3/3 decisions |
| 1. Domain Layer | 🔴 Not Started | 0/8 | 0/7 |
| 2. Adapters | 🔴 Not Started | 0/10 | 0/7 |
| 3. Application Layer | 🔴 Not Started | 0/6 | 0/4 |
| 4. CLI & UI | 🔴 Not Started | 0/4 | 0/4 |
| 5. Documentation | 🔴 Not Started | N/A | 0/3 |

**Legend**: 🔴 Not Started | 🟡 In Progress | 🟢 Complete

---

### Phase 0: Decision Gate ✅ COMPLETE

All design decisions have been resolved:

| Decision | Options | Chosen | Rationale |
| -------- | ------- | ------ | --------- |
| Is skills support worth the complexity? | Yes / No / Defer | ✅ **Yes** | Users manage skills in `.promptpack/`, Calvin distributes to projects/home |
| Extend `Asset` or create `SkillAsset`? | Extend (A) / Separate (B) | ✅ **Extend (A)** | Simpler: one type flows through pipeline |
| Should `calvin skills` command exist? | Yes / No | ✅ **No** | Manage skills like actions, keep CLI simple |

---

### Phase 1: Domain Layer (3-4 days)

**Goal**: Add `Skill` as a new `AssetKind` with supplemental file support.

#### 1.1 Contracts to Implement

These contracts MUST be tested first:

```text
CONTRACT-SKILL-001: Skill is a valid AssetKind
CONTRACT-SKILL-002: Asset can hold supplemental files (HashMap<PathBuf, String>)
CONTRACT-SKILL-003: Asset can hold allowed_tools (Vec<String>)
CONTRACT-SKILL-004: Frontmatter supports 'kind: skill'
CONTRACT-SKILL-005: Frontmatter supports 'allowed-tools' field
CONTRACT-SKILL-006: Skill directory without SKILL.md is an error
CONTRACT-SKILL-007: Binary supplemental files are rejected
CONTRACT-SKILL-008: Skill ID is derived from directory name
```

#### 1.2 TDD Workflow

```text
Step 1: Write failing test for CONTRACT-SKILL-001
Step 2: Add Skill variant to AssetKind enum
Step 3: Verify test passes
Step 4: Write failing test for CONTRACT-SKILL-002
Step 5: Add supplementals field to Asset
... continue for each contract ...
```

#### 1.3 TODO Checklist

**Tests First** (write these before implementation):

- [ ] `test_asset_kind_skill_exists` - `AssetKind::Skill` is valid
- [ ] `test_asset_supplementals_empty_by_default` - New field defaults empty
- [ ] `test_asset_with_supplementals_builder` - Builder method works
- [ ] `test_asset_allowed_tools_empty_by_default` - New field defaults empty
- [ ] `test_frontmatter_skill_kind_parses` - `kind: skill` deserializes
- [ ] `test_frontmatter_allowed_tools_parses` - `allowed-tools` deserializes
- [ ] `test_load_skill_directory_valid` - Loads skill with SKILL.md
- [ ] `test_load_skill_directory_missing_skill_md_errors` - Missing SKILL.md = error

**Implementation** (after tests exist):

- [ ] Add `Skill` to `AssetKind` in `src/domain/entities/asset.rs`
- [ ] Add `supplementals: HashMap<PathBuf, String>` to `Asset`
- [ ] Add `allowed_tools: Vec<String>` to `Asset`
- [ ] Add `with_supplementals()` and `with_allowed_tools()` builders
- [ ] Update `Frontmatter` in `src/models.rs` with `allowed_tools` field
- [ ] Implement `load_skills()` in `FsAssetRepository`
- [ ] Add `is_binary()` utility for supplemental validation

**Files to modify**:

- `src/domain/entities/asset.rs`
- `src/models.rs`
- `src/infrastructure/repositories/asset.rs`
- `src/parser.rs` (if skill directory parsing differs)

#### 1.4 Acceptance Criteria

Phase 1 complete when:

- [ ] All 8 contract tests pass
- [ ] `cargo test skill` runs without failures
- [ ] `AssetKind::Skill` compiles
- [ ] Skills can be loaded from `.promptpack/skills/<id>/SKILL.md`

---

### Phase 2: Adapters (2-3 days)

**Goal**: Compile skills to Claude Code, Codex, and Cursor. Reject skills for Antigravity/VS Code.

#### 2.1 Contracts to Implement

```text
CONTRACT-ADAPTER-001: ClaudeCodeAdapter compiles skill to .claude/skills/<id>/
CONTRACT-ADAPTER-002: ClaudeCodeAdapter includes all supplemental files
CONTRACT-ADAPTER-003: CodexAdapter compiles skill to .codex/skills/<id>/
CONTRACT-ADAPTER-004: CursorAdapter compiles skill to .claude/skills/<id>/ (shares path)
CONTRACT-ADAPTER-005: AntigravityAdapter returns empty vec for skills
CONTRACT-ADAPTER-006: VSCodeAdapter returns empty vec for skills
CONTRACT-ADAPTER-007: Skill SKILL.md includes name and description in frontmatter
CONTRACT-ADAPTER-008: Skill SKILL.md includes allowed-tools if present
CONTRACT-ADAPTER-009: Skill with dangerous allowed-tool emits warning
CONTRACT-ADAPTER-010: Skill with only unsupported targets is error
```

#### 2.2 TODO Checklist

**Tests First**:

- [ ] `test_claude_code_compile_skill_path` - Output path is `.claude/skills/<id>/SKILL.md`
- [ ] `test_claude_code_compile_skill_supplementals` - Includes reference.md, scripts/
- [ ] `test_claude_code_skill_frontmatter` - Has name, description, allowed-tools
- [ ] `test_codex_compile_skill_path` - Output path is `.codex/skills/<id>/SKILL.md`
- [ ] `test_cursor_compile_skill_uses_claude_path` - Uses `.claude/skills/`
- [ ] `test_antigravity_compile_skill_returns_empty` - Returns `vec![]`
- [ ] `test_vscode_compile_skill_returns_empty` - Returns `vec![]`
- [ ] `test_skill_dangerous_tool_warning` - Warns for `rm`, `sudo`, etc.
- [ ] `test_skill_no_supported_targets_error` - Error if only antigravity/vscode
- [ ] `test_skill_user_scope_uses_home_path` - Respects scope

**Implementation**:

- [ ] Add `compile_skill()` to `ClaudeCodeAdapter`
- [ ] Add `compile_skill()` to `CodexAdapter`
- [ ] Add `compile_skill()` to `CursorAdapter`
- [ ] Add skill rejection to `AntigravityAdapter::compile()`
- [ ] Add skill rejection to `VSCodeAdapter::compile()`
- [ ] Add `validate_allowed_tools()` helper
- [ ] Add `DANGEROUS_TOOLS` constant

**Files to modify**:

- `src/infrastructure/adapters/claude_code.rs`
- `src/infrastructure/adapters/codex.rs`
- `src/infrastructure/adapters/cursor.rs`
- `src/infrastructure/adapters/antigravity.rs`
- `src/infrastructure/adapters/vscode.rs`

#### 2.3 Acceptance Criteria

Phase 2 complete when:

- [ ] All 10 contract tests pass
- [ ] `cargo test adapter` runs without failures
- [ ] Skills compile correctly to supported platforms
- [ ] Skills are skipped for unsupported platforms

---

### Phase 3: Application Layer (2-3 days)

**Goal**: Deploy skills correctly, including multi-file output and orphan detection.

#### 3.1 Contracts to Implement

```text
CONTRACT-APP-001: DeployUseCase handles multi-file skill output
CONTRACT-APP-002: Lockfile tracks each skill file separately
CONTRACT-APP-003: Orphan detector removes old skill files when skill changes
CONTRACT-APP-004: Orphan detector removes empty skill directories
CONTRACT-APP-005: WatchUseCase detects skill directory changes
CONTRACT-APP-006: Skill override (project > user) emits warning
```

#### 3.2 TODO Checklist

**Tests First**:

- [ ] `test_deploy_skill_creates_directory` - Creates `.claude/skills/<id>/`
- [ ] `test_deploy_skill_multiple_files` - SKILL.md + supplementals in output
- [ ] `test_lockfile_tracks_skill_files` - Each file has lockfile entry
- [ ] `test_orphan_removes_old_skill_files` - Detects removed supplementals
- [ ] `test_orphan_removes_empty_skill_dir` - Cleans up empty directories
- [ ] `test_watch_skill_directory_change` - Triggers redeploy

**Implementation**:

- [ ] Update `DeployUseCase` to handle `Vec<OutputFile>` per asset
- [ ] Update `OrphanDetector` for directory-level cleanup
- [ ] Update `IncrementalCache` for skill directory watching
- [ ] Add skill override warning in layer merger

**Files to modify**:

- `src/application/deploy/use_case.rs`
- `src/domain/services/orphan_detector.rs`
- `src/application/watch/cache.rs`
- `src/infrastructure/layer/merger.rs` (if exists)

#### 3.3 Acceptance Criteria

Phase 3 complete when:

- [ ] All 6 contract tests pass
- [ ] Full deploy-clean-redeploy cycle works for skills
- [ ] `calvin clean` removes skill directories correctly

---

### Phase 4: CLI and UI (1-2 days)

**Goal**: Update CLI output to show skill information.

#### 4.1 TODO Checklist

**Tests First**:

- [ ] `test_layers_output_shows_skill_count` - Skills column in output
- [ ] `test_deploy_verbose_shows_skill_structure` - Tree view of skill files
- [ ] `test_check_validates_skills` - Skill validation in check command
- [ ] `test_deploy_warns_unsupported_platforms` - Warning message shown

**Implementation**:

- [ ] Update `calvin layers` to show skill counts
- [ ] Update `calvin deploy -v` to show skill tree structure
- [ ] Update `calvin check` to validate skills
- [ ] Add skills skipped warning in deploy output

**Files to modify**:

- `src/commands/deploy/cmd.rs`
- `src/commands/check/engine.rs`
- `src/ui/views/layers.rs` (or similar)

#### 4.2 Acceptance Criteria

Phase 4 complete when:

- [ ] All 4 contract tests pass
- [ ] `calvin deploy -v` shows skill structure
- [ ] `calvin layers` shows skill counts

---

### Phase 5: Documentation (1 day)

#### 5.1 TODO Checklist

- [ ] Update `docs/target-platforms.md` with skill paths
- [ ] Create `docs/skills.md` usage guide
- [ ] Update `GEMINI.md` (MEMORY file) with skill info

#### 5.2 Acceptance Criteria

Phase 5 complete when:

- [ ] All documentation updated
- [ ] Skills guide has working examples

---

### Total Estimated Effort: 10-14 days

| Phase | Days | Tests | Impl Tasks |
| ----- | ---- | ----- | ---------- |
| 0. Decision | 0.5 | 0 | 3 decisions |
| 1. Domain | 3-4 | 8 | 6 |
| 2. Adapters | 2-3 | 10 | 7 |
| 3. Application | 2-3 | 6 | 4 |
| 4. CLI and UI | 1-2 | 4 | 4 |
| 5. Documentation | 1 | 0 | 3 |
| **Total** | **10-14** | **28** | **27** |

---

### Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Directory assets uncover edge cases | High | Medium | Extensive TDD, incremental rollout |
| Multi-file orphan cleanup complexity | Medium | Medium | Test with complex skill structures |
| Platform format changes (Claude/Codex) | Low | High | Version adapters, maintain compatibility table |
| Performance with many supplementals | Low | Low | Add size limits, warn on large files |

---

### How to Track Progress

1. **Update this PRD** after completing each TODO
2. **Use commit prefixes**: `feat(skills):`, `test(skills):`
3. **Mark contracts as complete**: Change 🔴 to 🟢 in Progress Overview
4. **Link PRs to this PRD**: Reference in commit messages

## 8. Testing Strategy

### Contract Tests

```rust
/// CONTRACT: Skill directories require SKILL.md
#[test]
fn contract_skill_requires_skill_md() {
    // Create skill directory without SKILL.md
    // Expect: Error with clear message
}

/// CONTRACT: Skill supplementals are included in output
#[test]
fn contract_skill_supplementals_compiled() {
    // Create skill with reference.md and scripts/
    // Deploy to Claude Code
    // Verify all files exist in output
}

/// CONTRACT: Skill layer override is complete (not merge)
#[test]
fn contract_skill_layer_complete_override() {
    // User layer: skill with supplement A
    // Project layer: skill with supplement B
    // Result: Only supplement B in output
}
```

### Scenario Tests

```rust
/// SCENARIO: Developer creates skill with scripts
#[test]
fn scenario_skill_with_scripts() {
    // 1. Create .promptpack/skills/my-skill/
    // 2. Add SKILL.md and scripts/helper.py
    // 3. Run calvin deploy
    // 4. Verify .claude/skills/my-skill/ has all files
}

/// SCENARIO: Skill targets platform without support
#[test]
fn scenario_skill_unsupported_platform() {
    // 1. Create skill with targets: [antigravity]
    // 2. Deploy
    // 3. Verify warning about unsupported platform
    // 4. Verify NO output files for antigravity
}
```

---

## 9. Confirmed Decisions (Formerly Open Questions)

1. **Skills do NOT support `apply` glob pattern**
   - Skills are semantically-triggered, file patterns don't apply

2. **Skill directory nesting is supported**
   - E.g., `skills/my-skill/subdir/file.md` → `.claude/skills/my-skill/subdir/file.md`
   - Arbitrary nesting preserved in output

3. **No supplemental size limit**
   - User responsibility to manage file sizes

4. **Skill format versioning is deferred**
   - Current format is Claude Code compatible
   - Will address if/when Calvin adds skill-specific features

5. **`kind: skill` is OPTIONAL in SKILL.md frontmatter**
   - Files in `.promptpack/skills/` are auto-detected as skills
   - Explicit `kind: skill` is allowed but not required

6. **Skills and actions can have the same name without conflict**
   - They live in different directories: `skills/review/` vs `actions/review.md`
   - IDs are scoped by directory

7. **`allowed-tools` on non-skill assets is silently ignored**
   - No warning needed; just parsed and not used

---

## 10. Appendix: Skill Format Reference

### Minimal Skill

```
.promptpack/skills/gen-tests/
└── SKILL.md
```

```markdown
---
description: Generate unit tests for a function.
kind: skill
---

# Instructions

When asked to generate tests, analyze the function and create comprehensive unit tests.
```

### Full Skill

```
.promptpack/skills/draft-commit/
├── SKILL.md
├── reference.md
├── examples.md
└── scripts/
    └── validate.py
```

```markdown
---
description: Draft a conventional commit message.
kind: skill
scope: user
targets:
  - claude-code
  - codex
allowed-tools:
  - git
  - cat
---

# Instructions

When asked to draft a commit message:

1. Run `git diff --cached --stat` to see staged changes
2. Analyze the type of change
3. Format following conventional commits

## Reference

See `reference.md` for the full specification.
See `examples.md` for example messages.

## Scripts

- `scripts/validate.py`: Validates commit message format
```

---

## 11. References

- [Skill Organization Guide](../guides/skill-organization-codex-claude-cursor-antigravity.md)
- [Multi-Layer PRD](../archive/multi-layer-implementation/multi-layer-promptpack-prd.md)
- [Target Platforms](../target-platforms.md)
- [Testing Philosophy](../testing/TESTING_PHILOSOPHY.md)
- [Tech Decisions](../tech-decisions.md)

---

## Version History

| Date       | Author | Change                                            |
| ---------- | ------ | ------------------------------------------------- |
| 2025-12-27 | Arona  | Initial draft                                     |
| 2025-12-27 | Arona  | Revised with deeper Calvin integration            |
| 2025-12-27 | Arona  | Final review: all decisions confirmed by Sensei   |
