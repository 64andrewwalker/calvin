# Calvin Library API Reference

> Version: 0.2.0 | Last Updated: 2025-12-21

Calvin is available as a Rust library for programmatic use.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
calvin = "0.2"
```

## Module Overview

```
calvin
├── application/        # Use cases and orchestration
│   ├── DeployUseCase   # Deploy assets to targets
│   ├── CheckUseCase    # Security validation
│   ├── WatchUseCase    # File watching with auto-deploy
│   ├── DiffUseCase     # Preview changes
│   └── AssetPipeline   # Parse, filter, compile
├── domain/             # Pure business logic
│   ├── entities/       # Asset, OutputFile, Lockfile
│   ├── value_objects/  # Scope, Target, Hash
│   ├── services/       # Planner, OrphanDetector
│   ├── policies/       # ScopePolicy, SecurityPolicy
│   └── ports/          # Trait interfaces
├── infrastructure/     # I/O implementations
│   ├── adapters/       # Platform adapters
│   ├── fs/             # LocalFs, RemoteFs
│   └── repositories/   # Asset, Lockfile repos
├── config/             # Configuration types
├── models/             # PromptAsset, Frontmatter
├── parser/             # Frontmatter parsing
└── security/           # Security checks
```

---

## Core Types

### PromptAsset

Source asset parsed from `.promptpack/` files.

```rust
use calvin::PromptAsset;

pub struct PromptAsset {
    pub id: String,
    pub path: PathBuf,
    pub frontmatter: Frontmatter,
    pub body: String,
}
```

### Frontmatter

YAML metadata extracted from source files.

```rust
use calvin::Frontmatter;

pub struct Frontmatter {
    pub description: String,
    pub kind: Option<AssetKind>,
    pub target: Option<Scope>,
    pub targets: Option<Vec<Target>>,
    pub apply: Option<Vec<String>>,
    pub arguments: Option<Vec<Argument>>,
}
```

### OutputFile

Compiled output ready for deployment.

```rust
use calvin::OutputFile;

pub struct OutputFile {
    path: PathBuf,
    content: String,
    target: Target,
    hash: ContentHash,
}

impl OutputFile {
    pub fn new(path: impl Into<PathBuf>, content: impl Into<String>, target: Target) -> Self;
    pub fn path(&self) -> &Path;
    pub fn content(&self) -> &str;
    pub fn target(&self) -> Target;
    pub fn hash(&self) -> &ContentHash;
    pub fn is_empty(&self) -> bool;
}
```

### Scope

Deployment scope: project or user.

```rust
use calvin::Scope;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Project,  // Deploy to project directory (e.g., ./.claude/)
    User,     // Deploy to home directory (e.g., ~/.claude/)
}
```

### Target

Supported AI coding assistant platforms.

```rust
use calvin::Target;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    ClaudeCode,
    Cursor,
    VSCode,
    Antigravity,
    Codex,
}

impl Target {
    pub fn all() -> Vec<Target>;
    pub fn display_name(&self) -> &'static str;
    pub fn from_str(s: &str) -> Option<Target>;
}
```

### AssetKind

Type of prompt asset.

```rust
use calvin::AssetKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AssetKind {
    #[default]
    Action,     // Slash commands
    Policy,     // Rules/guidelines
    Agent,      // Sub-agent definitions
}
```

---

## Application Layer

### DeployUseCase

Orchestrates the complete deploy flow.

```rust
use calvin::application::{DeployUseCase, DeployOptions, DeployResult};
use calvin::Config;

// Builder pattern
let deploy = DeployUseCase::builder()
    .source(&PathBuf::from(".promptpack"))
    .destination(&PathBuf::from("."))
    .targets(vec![Target::ClaudeCode, Target::Cursor])
    .config(&Config::default())
    .dry_run(false)
    .force(false)
    .cleanup(false)
    .build();

// Execute
let result: DeployResult = deploy.execute()?;

// With event callbacks
let result = deploy.execute_with_events(|event| {
    match event {
        DeployEvent::Started { source, targets } => { /* ... */ }
        DeployEvent::Compiled { asset_count, file_count } => { /* ... */ }
        DeployEvent::FileWritten { path } => { /* ... */ }
        DeployEvent::Completed { result } => { /* ... */ }
    }
})?;
```

### DeployResult

Result of a deployment operation.

```rust
pub struct DeployResult {
    pub written: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,
    pub errors: Vec<DeployError>,
    pub orphans_removed: Vec<PathBuf>,
    pub warnings: Vec<String>,
}

impl DeployResult {
    pub fn written_count(&self) -> usize;
    pub fn skipped_count(&self) -> usize;
    pub fn error_count(&self) -> usize;
    pub fn orphan_count(&self) -> usize;
    pub fn is_success(&self) -> bool;
}
```

### CheckUseCase

Security and configuration validation.

```rust
use calvin::application::{CheckUseCase, CheckOptions, CheckResult};

let check = CheckUseCase::new(CheckOptions {
    root: PathBuf::from("."),
    mode: SecurityMode::Balanced,
    strict_warnings: false,
});

let result: CheckResult = check.execute()?;

if result.is_success() {
    println!("All checks passed!");
} else {
    for item in result.items() {
        println!("{}: {}", item.name, item.message);
    }
}
```

### WatchUseCase

File watching with auto-deployment.

```rust
use calvin::application::watch::{WatchUseCase, WatchOptions, WatchEvent};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

let running = Arc::new(AtomicBool::new(true));
let options = WatchOptions::builder()
    .source(PathBuf::from(".promptpack"))
    .targets(vec![Target::ClaudeCode])
    .build();

WatchUseCase::new(options).start(running.clone(), |event: WatchEvent| {
    match event {
        WatchEvent::Started { source } => println!("Watching {}", source.display()),
        WatchEvent::FileChanged { path } => println!("Changed: {}", path.display()),
        WatchEvent::SyncComplete { result } => println!("Synced: {} files", result.written),
        WatchEvent::Error { message } => eprintln!("Error: {}", message),
        WatchEvent::Shutdown => println!("Stopped"),
    }
})?;
```

### DiffUseCase

Preview what would change without writing.

```rust
use calvin::application::{DiffUseCase, DiffOptions, DiffResult, ChangeType};

let diff = DiffUseCase::new(DiffOptions {
    source: PathBuf::from(".promptpack"),
    destination: PathBuf::from("."),
    targets: vec![Target::ClaudeCode],
    scope: Some(Scope::Project),
});

let result: DiffResult = diff.execute()?;

for entry in result.entries() {
    match entry.change_type {
        ChangeType::Added => println!("+ {}", entry.path.display()),
        ChangeType::Modified => println!("~ {}", entry.path.display()),
        ChangeType::Removed => println!("- {}", entry.path.display()),
        ChangeType::Unchanged => { /* skip */ }
    }
}
```

### compile_assets

Low-level compilation function.

```rust
use calvin::{compile_assets, PromptAsset, Target, Config};
use calvin::domain::entities::OutputFile;

let assets: Vec<PromptAsset> = /* load from .promptpack */;
let targets = vec![Target::ClaudeCode, Target::Cursor];
let config = Config::default();

let outputs: Vec<OutputFile> = compile_assets(&assets, &targets, &config)?;

for output in outputs {
    println!("{}: {} bytes", output.path().display(), output.content().len());
}
```

---

## Infrastructure Layer

### Target Adapters

Platform-specific adapters that implement `TargetAdapter` trait.

```rust
use calvin::infrastructure::adapters::{ClaudeCodeAdapter, CursorAdapter};
use calvin::domain::ports::TargetAdapter;

// Get all adapters
let adapters = calvin::all_adapters();

// Get specific adapter
let claude = calvin::get_adapter(Target::ClaudeCode);

// Use adapter directly
let adapter = ClaudeCodeAdapter::new();
let outputs = adapter.compile(&asset)?;
```

### File Systems

Abstract file system operations.

```rust
use calvin::infrastructure::fs::{LocalFs, RemoteFs};
use calvin::domain::ports::FileSystem;

// Local file system
let fs = LocalFs::new();
fs.write_atomic(&path, &content)?;
let exists = fs.exists(&path);
let content = fs.read_to_string(&path)?;

// Remote file system (SSH)
let remote = RemoteFs::parse("user@host:/path")?;
remote.write_atomic(&path, &content)?;
```

### Repositories

Data access implementations.

```rust
use calvin::infrastructure::repositories::{FsAssetRepository, TomlLockfileRepository};
use calvin::domain::ports::{AssetRepository, LockfileRepository};

// Load assets from file system
let repo = FsAssetRepository::new();
let assets = repo.load_all(&source_path)?;

// Manage lockfile
let lockfile_repo = TomlLockfileRepository::new();
let lockfile = lockfile_repo.load(&project_root)?;
lockfile_repo.save(&project_root, &lockfile)?;
```

---

## Configuration

### Config

Configuration management.

```rust
use calvin::{Config, SecurityMode};

// Load from project
let config = Config::load_or_default(Some(&project_root));

// Access settings
let mode = config.security.mode;
let targets = config.enabled_targets();
let atomic = config.sync.atomic_writes;

// With environment overrides
let config = Config::load_with_env(Some(&project_root));
```

### SecurityMode

Security strictness levels.

```rust
use calvin::SecurityMode;

pub enum SecurityMode {
    Yolo,      // No security checks
    Balanced,  // Default: reasonable protections
    Strict,    // Maximum security, warnings = errors
}
```

---

## Parsing

### parse_frontmatter

Parse YAML frontmatter from markdown content.

```rust
use calvin::parse_frontmatter;

let content = r#"---
description: My action
target: project
---

# Action Content

This is the body.
"#;

let (frontmatter, body) = parse_frontmatter(content)?;
assert_eq!(frontmatter.description, "My action");
assert!(body.contains("Action Content"));
```

---

## Security

### run_doctor

Run security health checks.

```rust
use calvin::security::{run_doctor, DoctorReport};
use calvin::{Config, SecurityMode};

let config = Config::load_or_default(Some(&root));
let mut report = DoctorReport::new();

run_doctor(&root, SecurityMode::Balanced, &config, &mut report);

if report.has_errors() {
    for check in report.failed_checks() {
        eprintln!("Error: {}: {}", check.name, check.message);
    }
}
```

---

## Error Handling

### CalvinError

Unified error type.

```rust
use calvin::{CalvinError, CalvinResult};

pub enum CalvinError {
    Config(String),           // Configuration errors
    Parse(String),            // Parsing errors
    Io(std::io::Error),       // File system errors
    Compile(String),          // Compilation errors
    Sync(String),             // Sync errors
    Security(String),         // Security violations
}

// Use CalvinResult for fallible operations
fn my_function() -> CalvinResult<OutputFile> {
    // ...
}
```

---

## Examples

### Basic Deployment

```rust
use calvin::{Config, Target};
use calvin::application::{DeployUseCase, DeployOptions};
use std::path::PathBuf;

fn deploy_project() -> calvin::CalvinResult<()> {
    let source = PathBuf::from(".promptpack");
    let dest = PathBuf::from(".");
    let config = Config::load_or_default(Some(&dest));
    
    let deploy = DeployUseCase::builder()
        .source(&source)
        .destination(&dest)
        .targets(config.enabled_targets())
        .config(&config)
        .build();
    
    let result = deploy.execute()?;
    
    println!("Deployed {} files", result.written_count());
    if !result.warnings.is_empty() {
        for warn in &result.warnings {
            eprintln!("Warning: {}", warn);
        }
    }
    
    Ok(())
}
```

### Custom Adapter

```rust
use calvin::domain::ports::TargetAdapter;
use calvin::domain::entities::{Asset, OutputFile};
use calvin::domain::value_objects::Target;

struct MyCustomAdapter;

impl TargetAdapter for MyCustomAdapter {
    fn target(&self) -> Target {
        Target::Custom("my-tool".into())
    }
    
    fn compile(&self, asset: &Asset) -> Result<Vec<OutputFile>, AdapterError> {
        // Custom compilation logic
        let path = format!(".my-tool/{}.md", asset.id());
        let content = format!("# {}\n\n{}", asset.description(), asset.content());
        
        Ok(vec![OutputFile::new(path, content, self.target())])
    }
    
    fn validate(&self, output: &OutputFile) -> Vec<AdapterDiagnostic> {
        // Custom validation
        vec![]
    }
}
```

---

## See Also

- [Command Reference](../command-reference.md) - CLI documentation
- [Configuration](../configuration.md) - Configuration options
- [Frontmatter Spec](./frontmatter.md) - Source file format
- [Target Platforms](../target-platforms.md) - Platform-specific details

