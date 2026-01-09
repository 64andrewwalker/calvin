# Calvin Development Guide

> For AI coding assistants working on this codebase.

## What is Calvin?

Calvin is a PromptOps compiler. It takes a `.promptpack/` directory of Markdown files and compiles them into platform-specific outputs for Claude Code, Cursor, VSCode, Antigravity, Codex, and OpenCode.

**Core command**: `calvin deploy` - compiles and writes outputs to project.

**Documentation**: https://64andrewwalker.github.io/calvin/

## Quick Reference

```bash
cargo build          # Build
cargo test           # Test (must pass before committing)
cargo fmt            # Format (required)
cargo clippy         # Lint
```

## Architecture

Calvin uses **Clean Architecture** with 4 layers. See `docs/architecture/layers.md` for details.

```
Layer 0: Presentation  →  Layer 1: Application  →  Layer 2: Domain  ←  Layer 3: Infrastructure
```

**Key principle**: Domain defines traits (ports); Infrastructure implements them.

### Where to Find Things

| Task | Location | Reference |
|------|----------|-----------|
| Add CLI command | `src/commands/`, `src/presentation/cli.rs` | |
| Add platform adapter | `src/infrastructure/adapters/` | `docs/target-platforms.md` |
| Modify deploy logic | `src/application/deploy/use_case.rs` | |
| Change business rules | `src/domain/services/`, `src/domain/policies/` | |
| Add new port/trait | `src/domain/ports/` | `docs/architecture/ports.md` |

For complete directory structure, see `docs/architecture/directory.md`.

### Platform Adapters

5 adapters in `infrastructure/adapters/`: Antigravity, Claude Code, Codex, Cursor, VS Code.

Each implements `TargetAdapter` trait from `domain/ports/`.

## Development Rules

1. **TDD always** - Write tests first. All features need tests before merging.
2. **No god objects** - Keep files under 400 lines.
3. **Domain purity** - `domain/` never imports `infrastructure/` or `application/`.
4. **Security by default** - Never weaken security. Deny lists are mandatory.
5. **Never fail silently** - Invalid input should warn, not silently misbehave.
6. **Minimal changes** - Solve the problem at hand. Don't add unrequested features.

## Common Tasks

### Adding a new platform adapter

1. Create `src/infrastructure/adapters/<platform>.rs`
2. Implement `TargetAdapter` trait
3. Register in `src/infrastructure/adapters/mod.rs`
4. Add `Target` variant in `src/domain/value_objects/target.rs`
5. Add tests

See `docs/target-platforms.md` for format specifications.

### Modifying deploy behavior

1. Read `src/application/deploy/use_case.rs`
2. Check `src/domain/services/planner.rs` for conflict detection
3. Check `src/domain/services/compiler.rs` for output generation
4. Run `cargo test deploy` after changes

## Key Documents

| Document | Content |
|----------|---------|
| `docs/architecture/layers.md` | Layer responsibilities and dependencies |
| `docs/architecture/directory.md` | Complete directory structure |
| `docs/architecture/ports.md` | Port (trait) definitions |
| `docs/testing/testing-philosophy.md` | **Contract-first testing principles** |
| `docs/testing/contract-registry.md` | **All contracts Calvin guarantees** |
| `docs/target-platforms.md` | Platform output formats |
| `docs/configuration.md` | Config file options |
| `docs/command-reference.md` | CLI commands |

## Pre-commit Checklist

1. `cargo fmt`
2. `cargo clippy` - no warnings
3. `cargo test` - all pass

## Documentation Sync

> **RULE**: If you add a feature or introduce a breaking change, you MUST update the documentation in the same PR.

| Change Type | Update Locations (Dev & User) |
|-------------|-------------------------------|
| CLI option | `docs/command-reference.md` AND `docs-content/docs/api/index.mdx` |
| Config | `docs/configuration.md` AND `docs-content/docs/guides/configuration.mdx` |
| Adapter | `docs/target-platforms.md` AND `docs-content/docs/api/frontmatter.mdx` |
| Architecture | `docs/architecture/*.md` |
| New Feature | Relevant guide in `docs-content/docs/guides/` |
