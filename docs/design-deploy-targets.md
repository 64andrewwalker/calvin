# Deploy Target Design

> **Created**: 2025-12-19  
> **Status**: Discussion Draft

---

## Problem Statement

Current deploy behavior:

```bash
# Current: Source and project target are bound together
calvin deploy                    # Source: .promptpack, Target: .promptpack 的父目录
calvin deploy --home             # Source: .promptpack, Target: ~/
calvin deploy --remote user@host # Source: .promptpack, Target: remote host
```

**Issue**: When deploying to "project", the target is always the parent of `.promptpack`.
This means:
- If `.promptpack` is in `/Volumes/DevWork/projects/calvin/.promptpack`
- Deploy to project = deploy to `/Volumes/DevWork/projects/calvin/`
- User cannot deploy to a **different** project from the same `.promptpack`

---

## Use Cases

### Use Case 1: Project-Embedded PromptPack (Current Design) ✅

```
my-project/
├── .promptpack/           # Source
│   └── commands/
│       └── cmd.md
├── .claude/               # Target (created by deploy)
│   └── commands/
│       └── cmd.md
└── src/
    └── ...
```

**Workflow**:
1. Each project has its own `.promptpack`
2. `calvin deploy` creates IDE-specific files in the same project
3. This works well for project-specific prompts

### Use Case 2: Centralized PromptPack (Not Supported) ⚠️

```
~/prompts/.promptpack/     # Centralized source
    └── commands/
        └── shared.md

~/project-a/               # Target A
    └── .claude/...

~/project-b/               # Target B
    └── .claude/...
```

**Desired workflow**:
```bash
cd ~/prompts
calvin deploy --target ~/project-a
calvin deploy --target ~/project-b
```

**Current limitation**: This is not possible without copying `.promptpack` to each project.

---

## Proposed Solution

### Option A: Keep Current Design (Recommended for v1)

**Rationale**:
1. Simple mental model: `.promptpack` belongs to a project
2. Version control friendly: prompts are versioned with the project
3. Clear ownership: each project manages its own prompts

**Workaround for centralized prompts**:
- Use `--home` to deploy shared prompts to home directory
- Use symlinks if needed: `ln -s ~/prompts/.promptpack .promptpack`

### Option B: Add `--target` Flag (Future Enhancement)

```bash
# Deploy to specific project directory
calvin deploy --target ~/project-a

# Deploy to multiple targets
calvin deploy --target ~/project-a --target ~/project-b
```

**Pros**:
- Flexible for power users
- Supports centralized prompt management

**Cons**:
- More complex
- Harder to track which projects have which prompts
- Lockfile location becomes ambiguous

---

## Decision

For v1: **Keep current design (Option A)**

Reason:
1. Simpler implementation
2. Aligns with "prompts as code" philosophy (prompts belong to a project)
3. `--home` handles the "shared prompts" use case

### How to Deploy to Project

The current `calvin deploy` (without `--home`) DOES work for project deployment:

```bash
cd my-project
calvin deploy                    # Deploys to my-project/.claude/, .cursor/, etc.
```

**What deploy does**:
- Source: `my-project/.promptpack/`
- Target: `my-project/`
- Creates: `my-project/.claude/commands/`, `my-project/.cursor/rules/`, etc.

**This is the intended behavior** - it creates IDE configuration files from the PromptPack.

---

## Clarification: "Already Up-to-date"

If you see:
```
✓ Already Up-to-date
0 files written
✓ 110 files already up-to-date
```

This means:
1. All files already exist with matching content (hash match)
2. No changes needed
3. This is correct behavior - deploy is incremental

To force re-deploy:
```bash
calvin deploy --force
```

---

## Summary

| Target | Command | Description |
|--------|---------|-------------|
| Project (local) | `calvin deploy` | Deploy to `.promptpack`'s parent directory |
| Home | `calvin deploy --home` | Deploy to user home directory |
| Remote | `calvin deploy --remote user@host` | Deploy to remote server |

All three targets are working as designed.

