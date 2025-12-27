# Skill Organization in Codex, Claude Code, Cursor, and Antigravity

This guide covers how different AI coding environments organize "skills" or custom automation rules. While the implementations vary, the goal is consistent: defining reusable instructions and tools for your AI assistant.

## Codex (OpenAI)

OpenAI’s **Codex** (as of late 2025) supports a *Skills* system that mirrors Claude’s open skill spec. Users can define custom skills either globally or per-project.

### Directory Structure

* **Global Skills**: `~/.codex/skills/<skill-name>/`
* **Project Skills**: `.codex/skills/` (within the project root)

Codex automatically discovers these skills on startup.

### Skill Definition

Each skill directory **must contain a `SKILL.md` file**.

* **Frontmatter**: Begins with a YAML block specifying at least `name` and `description`.
* **Body**: Follows with Markdown guidelines or patterns for the model.

**Example `SKILL.md`:**

```markdown
---
name: draft-commit-message
description: Draft a conventional commit message from a change summary.
---

# Instructions
When the user provides a summary of changes, format the commit message follows:
...
```

Codex injects the skill’s name and description into the context. The full instructions are loaded only when the skill is invoked, efficiently managing context window usage.

### Advanced Features

* **Supplemental Resources**: Support for subfolders like `scripts/`, `assets/`, or `references/`.
* **Progressive Disclosure**: Extra files are not loaded by default but can be referenced by the skill instructions.
* **Compatibility**: Codex adopts Anthropic’s format. Existing Claude Code skills (with `SKILL.md`) work out-of-the-box in Codex if placed in `~/.codex/skills/`.

---

## Claude Code (Anthropic)

**Claude Code** introduced the *Agent Skills* format, now a de-facto standard.

### Directory Structure

* **Global Skills**: `~/.claude/skills/`
* **Project Skills**: `.claude/skills/` (inside the project root)

### Skill Definition

A skill is a folder containing a **`SKILL.md`** file.

* **Semantic Matching**: Claude uses the `description` in the YAML frontmatter to decide when to activate the skill.
* **Structure**:

    ```text
    .claude/skills/
    └── my-skill/
        ├── SKILL.md          # Entry point
        ├── reference.md      # Optional docs
        └── scripts/          # Optional executable tools
            └── helper.py
    ```

### Key Capabilities

* **Progressive Disclosure**: Main `SKILL.md` should be concise (~500 lines). Details can be offloaded to other files (e.g., `reference.md`), which Claude reads only when necessary.
* **Executable Tools**: The `scripts/` directory can hold scripts (Python, Shell) that Claude can execute as sandboxed tools.
* **Safety**: The `allowed-tools` field in frontmatter can restrict which CLI tools are permissible.

---

## Cursor

**Cursor** uses a different approach, focusing on **project rules** and **command files** rather than "skills".

### Compatibility

Cursor does not natively search `.claude/skills` now, but according to their docs, they plan to support it in the future.

---

## Google Antigravity

**Google Antigravity** (late 2025) is an "agent-first" IDE focusing on autonomous multi-agent workflows.

### Antigravity Compatibility

Antigravity does not natively search `.claude/skills` now.

---

## Summary Comparison

| Feature | Codex | Claude Code | Cursor | Antigravity |
| :--- | :--- | :--- | :--- | :--- |
| **Primary Unit** | `SKILL.md` folder | `SKILL.md` folder | `.cursorrules` & `.md` cmds | `.antigravity` file |
| **Global Path** | `~/.codex/skills/` | `~/.claude/skills/` | `~/.cursor/commands/` | (Enterprise presets) |
| **Project Path** | `.codex/skills/` | `.claude/skills/` | `.cursor/commands/` | `.antigravity` |
| **Execution** | Prompt + Scripts | Prompt + Scripts | Prompt templates | Autonomous Agents |
| **Standard** | Matches Claude | **The Standard** | Custom | Custom/Agentic |

## References

1. [OpenAI Codex Skills Guide](https://www.reddit.com/r/ChatGPTCoding/)
2. [Claude Code Agent Skills Docs](https://code.claude.com/docs/en/skills)
3. [Awesome Cursor Rules](https://github.com/PatrickJS/awesome-cursorrules)
4. [Antigravity AI Directory](https://antigravityai.directory/)
