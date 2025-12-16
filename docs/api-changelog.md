# API Changelog

All notable changes to Calvin's formats and interfaces.

## Format Changelog

### Source Format

#### [1.0] - 2024-12-17

**Initial Release**

- Frontmatter with required `description` field
- Optional fields: `targets`, `scope`, `apply`
- Directory structure: categorized by workflow stage

---

### Adapter Changelogs

#### Claude Code Adapter

##### [1.0] - 2024-12-17

**Initial Release**

- Output: `.claude/commands/<id>.md`
- Output: `.claude/settings.json` with permissions.deny
- Support: `$ARGUMENTS` variable substitution
- Support: Generated-by header marker

---

#### Cursor Adapter

##### [1.0] - 2024-12-17

**Initial Release**

- Output: `.cursor/rules/<id>/RULE.md` with frontmatter
- Output: `.cursor/commands/<id>.md`
- Mapping: `description` → rule frontmatter
- Mapping: `apply` glob → `globs` field

---

#### VS Code Adapter

##### [1.0] - 2024-12-17

**Initial Release**

- Output: `.github/copilot-instructions.md` (merged mode)
- Output: `.github/instructions/<id>.md` (split mode)
- Output: `AGENTS.md` summary index
- Mapping: `apply` glob → `applyTo` frontmatter

---

#### Antigravity Adapter

##### [1.0] - 2024-12-17

**Initial Release**

- Output: `.agent/rules/<id>.md`
- Output: `.agent/workflows/<id>.md`
- Status: beta - format may change

---

#### Codex Adapter

##### [1.0] - 2024-12-17

**Initial Release**

- Output: `~/.codex/prompts/<id>.md` (user scope only)
- Support: `$ARGUMENTS`, `$1`-`$9` placeholders
- Status: experimental

---

## CLI Changelog

### [Unreleased]

- Initial implementation

### Planned Commands

| Command | Description |
|---------|-------------|
| `calvin sync` | Compile and sync to all targets |
| `calvin watch` | Watch mode with incremental compile |
| `calvin diff` | Preview changes without writing |
| `calvin doctor` | Validate configuration |
| `calvin audit` | Security audit for CI |
| `calvin migrate` | Migrate between format versions |

---

## Breaking Changes

### How We Handle Breaking Changes

1. **Advance Notice**: Announced 2 minor versions ahead
2. **Deprecation Period**: Warning in CLI output
3. **Migration Tool**: `calvin migrate` command provided
4. **Documentation**: Migration guide published

### Upcoming Breaking Changes

*None planned*

---

## Deprecated Features

*None yet*

---

## Migration Guides

### Migrating from v0.x to v1.0

Not applicable - v1.0 is initial release.
