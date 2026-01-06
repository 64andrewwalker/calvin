# OpenCode Target Guide

This guide explains how Calvin compiles a `.promptpack/` into **OpenCode**-compatible outputs.

## Output Locations

Calvin writes OpenCode outputs to:

- **Project scope**: `.opencode/`
  - Agents: `.opencode/agent/<id>.md`
  - Commands (actions): `.opencode/command/<id>.md`
  - Skills: `.opencode/skill/<id>/SKILL.md`
  - Policies: `AGENTS.md` (project root)
- **User scope**: `~/.config/opencode/`
  - Agents: `~/.config/opencode/agent/<id>.md`
  - Commands (actions): `~/.config/opencode/command/<id>.md`
  - Skills: `~/.config/opencode/skill/<id>/SKILL.md`
  - Policies: `~/.config/opencode/AGENTS.md`

## Skills + Claude Code Compatibility

OpenCode also reads Claude Code skills from `.claude/skills/`.

When you deploy to **both** `claude-code` and `opencode`, Calvin writes skills to `.claude/skills/` only (no duplication under `.opencode/skill/`).

## Policies → `AGENTS.md`

OpenCode uses `AGENTS.md` as a rules/instructions file. Calvin aggregates all `kind: policy` assets targeting `opencode` into:

- `AGENTS.md` (project scope)
- `~/.config/opencode/AGENTS.md` (user scope)

If an `AGENTS.md` already exists and is not managed by Calvin, `calvin deploy` will skip it unless you deploy with `--force`.

## OpenCode Frontmatter Fields

Calvin supports a few OpenCode-only fields in source frontmatter (used only when compiling for `opencode`):

- Agents: `mode`, `temperature`, `opencode-model`
- Actions: `agent`, `subtask`, `opencode-model`

See `docs-content/docs/api/frontmatter.mdx` for the full specification.

