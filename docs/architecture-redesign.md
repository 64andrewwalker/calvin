# Architecture Redesign (Skills Branch)

> **Date**: 2025-12-27  
> **Branch**: `feat/skills`  
> **Goal**: Reduce duplication + complexity introduced by Skills support while preserving Calvin’s clean-architecture boundaries and current behavior.

---

## Executive Summary

Skills support is implemented and well-tested, but the implementation currently spreads “skill concerns” across multiple adapters, the security module, and multiple use cases. This increases maintenance cost and makes future “directory asset” features (e.g., MCP) harder to add without repeating the same patterns.

**Primary recommendation**: Introduce a small set of shared “Skill Support” utilities (domain policy + application helpers + infrastructure adapter helper) to eliminate the most obvious duplication:

1. **Shared skill compilation helper** for adapters (Claude/Codex/Cursor).
2. **Shared dangerous-tool policy** for `allowed-tools` warnings (adapters + `security/checks.rs`).
3. **Shared skill output path classifier** used by both Deploy and Clean.
4. **Centralized target capability** (`Target::supports_skills()`), used for deploy-time validation and warnings.

This keeps complexity flat as the feature set grows: “add one helper + one test suite” rather than “copy 3–5 similar implementations”.

---

## 1) Current State Assessment

### 1.1 Component Map (Skills-relevant)

- **Domain**
  - `AssetKind::Skill` + `Asset::supplementals` + `Asset::allowed_tools`
  - `merge_layers()` scopes IDs by kind (`skill:<id>`) to avoid conflicts with file assets
- **Infrastructure**
  - `FsAssetRepository` loads skill directories + supplementals and rejects unsafe content
  - Adapters generate `SKILL.md` + copy supplementals to target-specific skill directories
- **Application**
  - `DeployUseCase` validates skill targets, surfaces adapter diagnostics for skill outputs, and handles skill directory cleanup rules
  - `CleanUseCase` treats “skill supplementals” as safe-to-delete when the directory’s `SKILL.md` is Calvin-signed
- **Security**
  - `security/checks.rs` validates deployed skill folders + warns on dangerous `allowed-tools` usage

### 1.2 Data Flow (Skills)

```mermaid
flowchart LR
  A[.promptpack/skills/<id>/SKILL.md + files] --> B[FsAssetRepository::load_skills]
  B --> C[domain::Asset(kind=Skill, supplementals, allowed_tools)]
  C --> D[DeployUseCase]
  D --> E[compile_assets]
  E --> F[TargetAdapters (Claude/Codex/Cursor)]
  F --> G[OutputFile list (SKILL.md + supplementals)]
  G --> H[Planner + OrphanDetector]
  H --> I[Destination FS write + lockfile update]
```

### 1.3 Coupling & Cohesion Notes (Evidence)

**High duplication (low cohesion)**:

- Skill compilation is duplicated across 3 adapters:
  - `src/infrastructure/adapters/claude_code.rs:42`
  - `src/infrastructure/adapters/codex.rs:67`
  - `src/infrastructure/adapters/cursor.rs:68`
- Skill `allowed-tools` dangerous-tool validation is duplicated 3× in adapters and 1× in security checks:
  - `src/infrastructure/adapters/claude_code.rs:214`
  - `src/infrastructure/adapters/codex.rs:229`
  - `src/infrastructure/adapters/cursor.rs:202`
  - `src/security/checks.rs:21`
- Skill output path classification is duplicated across deploy and clean:
  - `src/application/deploy/use_case.rs:1249`
  - `src/application/clean/use_case.rs:258`

**Large files impacted by skills support**:

- `src/infrastructure/repositories/asset.rs` (skill loading + tests): 564 LOC
- `src/infrastructure/adapters/claude_code.rs`: 508 LOC
- `src/security/checks.rs`: 665 LOC (already `calvin-no-split` with justification)

---

## 2) Issue List (Prioritized, With Severity)

### 🔴 P0 — DRY violation: duplicated skill compilation in adapters

**What**: `compile_skill()` + `generate_skill_md()` are effectively identical across Claude/Codex/Cursor adapters, differing only by `(target, base dir)`.

**Why it matters**:
- Bug fixes / feature changes (e.g., adding new frontmatter fields) must be made 3×.
- Risk of adapters drifting over time.

**Evidence**:
- `src/infrastructure/adapters/claude_code.rs:42`
- `src/infrastructure/adapters/codex.rs:67`
- `src/infrastructure/adapters/cursor.rs:68`

### 🔴 P0 — DRY + security semantics: duplicated dangerous-tool list + parsing

**What**: `DANGEROUS_TOOLS` / `SKILL_DANGEROUS_TOOLS` constants and the parsing+warning logic exist in 4 places.

**Why it matters**:
- Security policy can drift (one list updated, another forgotten).
- A “security-by-default” repo should not duplicate safety-critical lists.

**Evidence**:
- `src/security/checks.rs:21`
- `src/infrastructure/adapters/claude_code.rs:215`
- `src/infrastructure/adapters/codex.rs:230`
- `src/infrastructure/adapters/cursor.rs:203`

### 🟠 P1 — Duplicated “skill directory ownership” logic across Deploy + Clean

**What**: `skill_root_from_path()` and `is_part_of_calvin_skill()` are duplicated (same algorithm, different FS).

**Why it matters**:
- Orphan cleanup / clean behavior is safety-sensitive and should have one canonical implementation.
- Adds cognitive load to maintainers.

**Evidence**:
- `src/application/deploy/use_case.rs:1249`
- `src/application/clean/use_case.rs:258`

### 🟠 P1 — Target capability knowledge is scattered and ad hoc

**What**: Deploy validation/warnings embed “which targets support skills” in `DeployUseCase` instead of a canonical capability definition.

**Why it matters**:
- Feature flags (skills, MCP, etc.) will proliferate. Without a capability API, complexity grows non-linearly.

**Evidence**:
- `src/application/deploy/use_case.rs:1267` (manual target match)
- `src/application/deploy/use_case.rs:1313` (manual unsupported list)

### 🟡 P2 — Discovery is split between parser + repository (double traversal / rules split)

**What**: Parser explicitly skips `skills/`, and repository loads skills separately. This is understandable, but it creates two “sources of truth” for promptpack traversal rules.

**Why it matters**:
- Adds friction for future directory-based assets.
- Requires careful coordination between `parser.rs` and `FsAssetRepository`.

### 🟡 P2 — Test helper duplication

**What**: `write_project_skill()` appears in multiple integration test files.

**Evidence**:
- `tests/cli_deploy_skills.rs:7`
- `tests/cli_skills.rs:7`

---

## 3) Requirements Alignment

Any redesign must preserve:

- **Clean architecture rule**: Presentation → Application → Domain ← Infrastructure
- **Domain purity**: policies/logic are pure; I/O stays in infrastructure/security modules
- **Security-by-default**: centralize, don’t weaken checks; avoid divergent allow/deny lists
- **No silent failures**: invalid skill structure must surface actionable errors/warnings
- **Behavioral compatibility**:
  - Skills compile to `SKILL.md` directories for Claude/Codex/Cursor.
  - Skills are skipped for VS Code / Antigravity with warnings; errors when skills have only unsupported targets.
  - Skill supplementals are copied and preserved (relative structure), with path safety constraints.

---

## 4) Proposed Architecture

### 4.1 Proposed Components

**Domain**

- `Target::supports_skills() -> bool`
- `AllowedToolsPolicy` (or extend `SecurityPolicy`) to define:
  - canonical dangerous tool list
  - `fn is_dangerous_tool(&self, tool: &str) -> bool`

**Application**

- `application/skills.rs` (new) – shared helpers:
  - `skill_root_from_path(path: &Path) -> Option<PathBuf>`
  - `is_part_of_calvin_skill(fs: &dyn FileSystem, path: &Path) -> bool`

**Infrastructure**

- `infrastructure/adapters/skills.rs` (new) – shared adapter helper:
  - `compile_skill(asset, target, base_dir_fn) -> Vec<OutputFile>`
  - `render_skill_md(asset, footer) -> String`
  - `validate_skill_allowed_tools(output, policy) -> Vec<AdapterDiagnostic>`

Adapters become thin wrappers around these helpers.

### 4.2 Proposed Component Diagram

```mermaid
flowchart TB
  subgraph Domain
    T[Target supports_skills()]
    SP[SecurityPolicy / AllowedToolsPolicy]
  end

  subgraph Application
    DU[DeployUseCase]
    CU[CleanUseCase]
    SU[application/skills.rs]
  end

  subgraph Infrastructure
    AR[FsAssetRepository]
    ACLAUDE[ClaudeCodeAdapter]
    ACODEX[CodexAdapter]
    ACURSOR[CursorAdapter]
    ASH[adapters/skills.rs]
  end

  subgraph Security
    SC[security/checks.rs]
  end

  DU --> SU
  CU --> SU

  ACLAUDE --> ASH
  ACODEX --> ASH
  ACURSOR --> ASH

  ASH --> SP
  SC --> SP
  DU --> T
```

---

## 5) Migration Plan (Step-by-Step, With Risk)

### Step 0 — Add missing characterization tests (low risk)

- Add unit tests for the new helper modules (before moving code):
  - `skill_root_from_path()` classification for `.claude/skills/...` and `.codex/skills/...`
  - `Target::supports_skills()` truth table
  - `AllowedToolsPolicy` dangerous list behavior

Rollback: revert the new helper modules and tests.

### Step 1 — Centralize dangerous-tool policy (medium risk, security-sensitive)

- Introduce `AllowedToolsPolicy` (or extend `SecurityPolicy`) in domain.
- Replace:
  - `src/security/checks.rs:21`
  - `src/infrastructure/adapters/*:DANGEROUS_TOOLS`
- Keep behavior identical (warnings, not errors).

Rollback: keep old constants for one release behind a shim if needed.

### Step 2 — Extract adapter skill helper (medium risk)

- Create `src/infrastructure/adapters/skills.rs` and move:
  - `generate_skill_md`
  - supplemental path validation
  - the core compile loop
- Update the 3 adapters to call the helper with:
  - `(Target, skills_dir(scope))`

Rollback: revert adapters to local implementations (helper module stays unused).

### Step 3 — Deduplicate deploy/clean skill directory ownership logic (medium risk)

- Create `src/application/skills.rs`
- Move:
  - `skill_root_from_path` (currently duplicated)
  - `is_part_of_calvin_skill` (currently duplicated)
- Update both use cases to call the shared functions.

Rollback: revert to local functions in each use case.

### Step 4 — Centralize “supports skills” capability (low risk)

- Add `Target::supports_skills()` in `src/domain/value_objects/target.rs`
- Replace ad-hoc target matching in:
  - `src/application/deploy/use_case.rs:1267`
  - `src/application/deploy/use_case.rs:1313`

Rollback: keep the old match blocks temporarily or revert.

### Step 5 (Optional) — Reduce `yaml_has_key()` complexity in skill loader (higher risk)

Current implementation needs `yaml_has_key()` because `Frontmatter.kind` defaults mask whether the key was explicitly provided.

Option A (safe-ish, incremental): add a “raw key presence” field to the extracted frontmatter structure.

Option B (larger change): change `Frontmatter.kind` to `Option<AssetKind>` and apply defaulting in one place.

Rollback: keep the current `yaml_has_key()` approach.

---

## 6) Breaking Changes Inventory

Expected **none** for users if Steps 0–4 are implemented as internal refactors.

Potential breaking changes only apply if Step 5 is chosen (changes to parsing semantics/types).

---

## 7) Rollback Strategy

- Keep each step as a small PR/commit series, each independently revertible.
- Prefer “introduce helper → switch one caller → switch remaining callers → delete old code” to minimize risk windows.
- For security-sensitive changes (dangerous tool list), run the full test suite and add one “policy consistency” test that ensures adapter + security checks share the same list.

