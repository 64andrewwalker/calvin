# Calvin API Reference

> Version: 0.2.0 | Last Updated: 2025-12-21

This directory contains API documentation for Calvin.

## Index

| Document | Description |
|----------|-------------|
| [versioning.md](./versioning.md) | API versioning policy |
| [changelog.md](./changelog.md) | Format version history |
| [library.md](./library.md) | Rust library API reference |
| [frontmatter.md](./frontmatter.md) | Frontmatter specification |

## Quick Reference

### CLI Commands

See [Command Reference](../command-reference.md) for full CLI documentation.

```bash
calvin deploy           # Deploy to project
calvin deploy --home    # Deploy to home directory
calvin deploy --remote  # Deploy to remote server
calvin check            # Validate configuration
calvin watch            # Watch for changes
calvin diff             # Preview changes
calvin explain          # Show usage information
calvin version          # Show version info
```

### Library Usage

Calvin can be used as a Rust library:

```rust
use calvin::{Config, compile_assets, DeployResult};
use calvin::application::{DeployUseCase, DeployOptions};
use calvin::infrastructure::{LocalFs, FsAssetRepository};

// Load configuration
let config = Config::load_or_default(Some(&project_root));

// Create a deploy use case
let deploy = DeployUseCase::builder()
    .source(&source_path)
    .destination(&dest_path)
    .targets(config.enabled_targets())
    .config(&config)
    .build();

// Execute deployment
let result = deploy.execute()?;
println!("Deployed {} files", result.written_count());
```

### PromptPack Structure

```
.promptpack/
├── config.toml          # Configuration file
├── policies/            # Rules that apply to AI behavior
│   ├── code-style.md    # → Becomes .cursor/rules/, .agent/rules/, etc.
│   └── security.md
├── actions/             # Slash commands and workflows
│   ├── generate-tests.md
│   └── pr-review.md
├── agents/              # Sub-agent definitions
│   └── reviewer.md
└── mcp/                 # MCP server configurations
    └── github.toml
```

### Frontmatter Fields

```yaml
---
description: Short description (required)
target: user | project              # Deploy scope (default: project)
targets: [claude-code, cursor]      # Platform filter (default: all)
apply: ["**/*.ts", "**/*.js"]       # Glob patterns for rules
arguments:                          # Command arguments
  - name: target_file
    description: File to process
    required: true
---
```

See [frontmatter.md](./frontmatter.md) for complete specification.

