# Skills

Calvin supports **Skills** as a directory-based asset type inside `.promptpack/skills/`.

## Layout

```text
.promptpack/
└── skills/
    └── <id>/
        ├── SKILL.md          # Required
        ├── reference.md      # Optional supplemental
        └── scripts/          # Optional supplemental subtree
            └── validate.py
```

- The **skill id** is the directory name (`<id>`).
- `SKILL.md` is the entrypoint and **must exist**.
- All other files under the directory are treated as **supplementals** and are copied to outputs with the same relative paths.

## `SKILL.md` Frontmatter

`SKILL.md` uses YAML frontmatter. Required and supported fields:

```yaml
---
description: Draft a conventional commit message.
kind: skill            # Optional inside skills/ (must be 'skill' if present)
scope: project         # Optional (project|user)
targets: [codex]       # Optional (defaults to all)
allowed-tools:         # Optional (skill-only)
  - git
  - cat
---
```

- `apply` is **not supported** for skills.
- `allowed-tools` is validated and Calvin warns on risky tools (e.g. `rm`, `sudo`, `curl`).

## Supplemental File Rules

To keep skills safe and portable, Calvin enforces:

- **Valid UTF-8 for text files**: text files with invalid encodings are rejected.
- **No symlinks**: symlinks inside skill directories are rejected.
- **Path safety**: supplemental paths cannot escape the skill directory (no `../` traversal).

### Binary Asset Support

Binary files (images, PDFs, fonts, etc.) in skill directories are automatically detected and deployed:

```text
.promptpack/skills/my-skill/
├── SKILL.md
├── references/
│   └── guide.md          # Text file
└── assets/
    ├── logo.png          # Binary file - deployed
    └── diagram.pdf       # Binary file - deployed
```

Calvin uses a **null-byte heuristic** to detect binary files. When a binary file is found:

1. The file is stored separately from text supplementals
2. A warning is displayed: `Skill 'my-skill': binary file 'assets/logo.png' will be deployed (45.2 KB)`
3. The file is deployed byte-for-byte to the target directory
4. The lockfile tracks the file with `is_binary = true`

Binary files are fully supported for orphan detection and cleanup (`--cleanup`).

## Platform Support and Output Paths

Skills compile to `SKILL.md` folders on supported platforms:

| Platform | Project scope | User scope |
|----------|--------------|------------|
| Claude Code | `.claude/skills/<id>/SKILL.md` | `~/.claude/skills/<id>/SKILL.md` |
| Codex | `.codex/skills/<id>/SKILL.md` | `~/.codex/skills/<id>/SKILL.md` |
| Cursor | `.claude/skills/<id>/SKILL.md` | `~/.claude/skills/<id>/SKILL.md` |

**Not supported**: VS Code + Copilot, Antigravity. Skills are skipped for these targets (with warnings when applicable).

## Multi-layer Semantics

Skills follow Calvin’s layer precedence rules (project > team > user):

- The higher-priority skill **fully overrides** the lower-priority skill (directory is treated as a unit).
- Calvin surfaces overrides in `calvin deploy` warnings and in verbose output (`calvin deploy -v`).

## Commands

- `calvin deploy`: compiles skills alongside policies/actions/agents.
- `calvin deploy -v`: shows skill structure (skill + supplemental files).
- `calvin layers`: shows per-layer counts for both assets and skills.
- `calvin check`: validates deployed skill folders and warns on dangerous `allowed-tools`.

