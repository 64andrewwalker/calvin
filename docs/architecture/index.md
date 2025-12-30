# Calvin Architecture

> **Created**: 2025-12-19  
> **Updated**: 2025-12-20  
> **Status**: Implemented (v2)

This directory contains Calvin's architecture design documents.

## Document Index

| Document | Description |
|----------|-------------|
| [overview.md](./overview.md) | Architecture overview: design goals and key insights |
| [layers.md](./layers.md) | Four-layer architecture details |
| [directory.md](./directory.md) | Directory structure specification |
| [ports.md](./ports.md) | Port interfaces (dependency inversion) |
| [platform.md](./platform.md) | Cross-platform layer design (macOS/Linux/Windows) |
| [migration.md](./migration.md) | Migration path and comparison |
| [docs.md](./docs.md) | Documentation architecture |
| [file-size-guidelines.md](./file-size-guidelines.md) | File size and splitting guidelines |
| [TODO.md](./TODO.md) | Refactoring progress tracking |

## Core Insight

```
Assets → Outputs → Files
(source) (intermediate) (target)
```

Calvin is a **compiler + deployer**.

## Dependency Rule

```
Presentation → Application → Domain ← Infrastructure
```

Domain has no external dependencies - this is key to testability and extensibility.

