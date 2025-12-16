# Technology Decisions

This document records the key technology choices for Calvin, including alternatives considered and rationale.

## TD-1: Programming Language

**Decision**: Rust

**Status**: Accepted

**Context**: Calvin needs to:
- Produce a single static binary
- Cross-compile for mac/win/linux
- Have zero runtime dependencies
- Perform concurrent file watching efficiently

**Alternatives Considered**:

| Option | Pros | Cons |
|--------|------|------|
| **Rust** | Zero runtime deps, small binary, memory safe, excellent concurrency | Steeper learning curve |
| Go | Simple concurrency, easy cross-compile, fast compile times | Larger binaries, less elegant template handling |
| Node/Deno | Team familiarity, fast iteration | Runtime dependency, Windows deployment issues |
| Python | Rapid development, rich ecosystem | venv hell, bundling complexity, startup time |

**Rationale**: The spec explicitly requires "single static executable" and "no Python/Node runtime". Rust is the only choice that fully satisfies these constraints while providing safety guarantees.

---

## TD-2: CLI Framework

**Decision**: `clap` with derive macros

**Status**: Accepted

**Alternatives**:
- `argh`: Simpler but less feature-rich
- `structopt`: Deprecated in favor of clap derive
- Manual parsing: Too error-prone

**Rationale**: clap is the industry standard, provides excellent UX (auto-completion, colored help), and derive macros reduce boilerplate.

---

## TD-3: YAML Frontmatter Parsing

**Decision**: `serde_yaml` + custom frontmatter extraction

**Status**: Accepted

**Implementation Approach**:
```rust
// Extract frontmatter between --- delimiters
fn extract_frontmatter(content: &str) -> Result<(Frontmatter, &str), ParseError> {
    // Split on "---" lines
    // Parse first section as YAML
    // Return remaining content as body
}
```

**Alternatives**:
- `gray_matter`: Feature-complete but adds dependency
- `pulldown-cmark`: For Markdown parsing, but doesn't handle frontmatter

**Rationale**: YAML frontmatter parsing is simple enough that a custom implementation with serde_yaml is cleaner than adding a specialized crate.

---

## TD-4: Template Engine

**Decision**: Context-aware string substitution with format-specific escaping

**Status**: Accepted (revised after P2 pitfall review)

**Rationale**: For a PromptOps tool, we need:
1. Variable substitution (`$ARGUMENTS`, `$1`, `$KEY`) 
2. Format-specific escaping to prevent corruption

**⚠️ Critical Rule**: Never use raw `replace()` for structured data (JSON/TOML/YAML).

**Implementation**:
```rust
pub enum OutputFormat {
    Markdown,   // No escaping
    Json,       // Escape quotes, backslashes, newlines
    Toml,       // Escape quotes in strings
    Yaml,       // Handle special chars, multiline
    Raw,        // Pass through
}

fn substitute(template: &str, vars: &HashMap<String, String>, format: OutputFormat) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        let escaped = match format {
            OutputFormat::Json => escape_json(value),
            OutputFormat::Toml => escape_toml(value),
            OutputFormat::Yaml => escape_yaml(value),
            OutputFormat::Markdown | OutputFormat::Raw => value.clone(),
        };
        result = result.replace(&format!("${}", key), &escaped);
    }
    result
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('"', "\\\"")
     .replace('\n', "\\n")
     .replace('\r', "\\r")
     .replace('\t', "\\t")
}
```

**Best Practice for Structured Output**:
```rust
// ✅ GOOD: Use serde - it handles escaping automatically
let settings = ClaudeSettings { instruction: user_content };
serde_json::to_string_pretty(&settings)?

// ❌ BAD: String interpolation - WILL corrupt on quotes
format!(r#"{{"instruction": "{}"}}"#, user_content)
```

**Why NOT Jinja**:
- Runtime parsing overhead
- Python-ish syntax feels foreign in Rust
- No compile-time validation
- Overkill for variable substitution

---

## TD-5: File System Watcher

**Decision**: `notify` crate

**Status**: Accepted

**Alternatives**:
- `hotwatch`: Wrapper around notify, less control
- OS-specific APIs: Non-portable
- Polling: Inefficient

**Rationale**: `notify` is the de-facto standard for cross-platform file watching in Rust, supporting inotify (Linux), FSEvents (macOS), and ReadDirectoryChangesW (Windows).

---

## TD-6: Remote Sync Strategy

**Decision**: Shell out to system tools with Windows fallback

**Status**: Accepted (revised after P3 pitfall review)

**⚠️ Problem**: `rsync` on Windows requires MSYS2/Cygwin/Git Bash - unreliable.

**Fallback Chain**:
```rust
fn sync_remote(src: &Path, dest: &str) -> Result<()> {
    // 1. Try rsync (most efficient, common on Unix)
    if which::which("rsync").is_ok() {
        return sync_via_rsync(src, dest);
    }
    
    // 2. Fall back to scp (Windows 10+ has built-in OpenSSH)
    if which::which("scp").is_ok() {
        return sync_via_scp(src, dest);
    }
    
    // 3. Error with actionable message
    Err(anyhow!("No sync tool found. Install OpenSSH (Windows 10+) or rsync."))
}

fn sync_via_rsync(src: &Path, dest: &str) -> Result<()> {
    Command::new("rsync")
        .args(["-avz", "--delete", src.to_str()?, dest])
        .status()?;
    Ok(())
}

fn sync_via_scp(src: &Path, dest: &str) -> Result<()> {
    // scp -r for recursive copy (less efficient than rsync, but reliable)
    Command::new("scp")
        .args(["-r", src.to_str()?, dest])
        .status()?;
    Ok(())
}
```

**Why System SSH**:
1. Respects user's `~/.ssh/config` and agent
2. No key management in our codebase
3. Battle-tested security
4. Works with any auth method

---


## TD-7: Configuration File Format

**Decision**: TOML

**Status**: Accepted

**Alternatives**:
- YAML: Already used for frontmatter, but more complex parsing rules
- JSON: No comments, verbose
- INI: Limited nesting

**Rationale**: TOML is:
- Native to Rust ecosystem (Cargo uses it)
- Supports comments
- Human-readable with simple syntax
- Clear semantics for nested tables

**Config Locations**:
```
~/.config/calvin/config.toml    # User config (XDG compliant)
.promptpack/config.toml         # Project config
```

---

## TD-8: JSON Output for CI

**Decision**: `serde_json` with `--json` flag

**Status**: Accepted

**Output Modes**:
1. Default: Human-readable with colors (terminal)
2. `--json`: Machine-readable NDJSON (Newline Delimited JSON)

**Rationale**: NDJSON allows streaming output, perfect for long-running `watch` mode. Each line is a complete JSON object that CI systems can parse.

```jsonl
{"event":"compile","asset":"security-policy","status":"success","outputs":[".claude/settings.json"]}
{"event":"sync","file":".claude/settings.json","status":"written","bytes":1234}
```

---

## TD-9: Testing Strategy

**Decision**: `cargo test` + `insta` for snapshot testing

**Status**: Accepted

**Test Categories**:
1. **Unit Tests**: Parser, adapter logic
2. **Snapshot Tests**: Generated file contents (using insta)
3. **Integration Tests**: Full compile cycle
4. **Golden Tests**: Reference `.promptpack/` → expected output

**Rationale**: Snapshot testing is ideal for compiler output - easily update expected files when format intentionally changes.

---

## TD-10: Error Handling

**Decision**: `anyhow` for application errors, `thiserror` for library errors

**Status**: Accepted

**Error Categories**:
```rust
#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("missing required field '{field}' in {file}:{line}")]
    MissingField { field: String, file: PathBuf, line: usize },
    
    #[error("invalid frontmatter: {0}")]
    InvalidFrontmatter(#[from] serde_yaml::Error),
}
```

**Exit Codes**:
| Code | Meaning |
|------|---------|
| 0    | Success |
| 1    | Validation/compilation error |
| 2    | System error (IO, permissions) |
| 3    | Configuration error |

---

## TD-11: Binary Distribution

**Decision**: GitHub Releases with pre-built binaries + `cargo-binstall` support

**Status**: Planned

**Targets**:
- `x86_64-apple-darwin` (Intel Mac)
- `aarch64-apple-darwin` (Apple Silicon)
- `x86_64-pc-windows-msvc` (Windows)
- `x86_64-unknown-linux-gnu` (Linux)
- `aarch64-unknown-linux-gnu` (Linux ARM)

**CI**: GitHub Actions with `cross` for cross-compilation

---

## TD-12: MCP Server Allowlist

**Decision**: Built-in allowlist + extensible via config

**Status**: Planned

**Built-in Allowed Servers**:
```toml
# Trusted MCP servers
[mcp.allowlist]
filesystem = { publisher = "anthropic" }
github = { publisher = "official" }
# ... etc
```

**Custom Allowlist**:
```toml
# In user or project config
[mcp.allowlist.custom]
internal-tools = { repo = "company/mcp-internal" }
```

**Rationale**: MCP servers execute arbitrary code, so whitelisting is available for team safety (but not enforced in yolo mode).

---

## TD-13: Security Mode Philosophy

**Decision**: Three-tier security modes with `balanced` as default

**Status**: Accepted

**Modes**:

| Mode | Behavior | Use Case |
|------|----------|----------|
| `yolo` | No enforcement, INFO logs only | Personal projects, experienced users |
| `balanced` | Generate protections, WARN on issues | Most teams (default) |
| `strict` | Block on security violations | Enterprise, compliance-heavy |

**Configuration**:
```toml
# .promptpack/config.toml
[security]
mode = "yolo"  # Let me live dangerously
```

**Rationale**:

The original spec mandated strict security (deny lists, MCP blocking). However:

1. **Power users know what they're doing** - forcing security theater on experienced developers breeds resentment
2. **Most users run full-access mode anyway** - AI tools like Claude Code, Cursor are typically given full permissions
3. **Security should be opt-in for personal projects** - the overhead isn't worth it for side projects
4. **Teams can enforce strict mode** - enterprises that need it can set `mode = "strict"`

**Design Principle**: Provide the tools, don't force the religion.

**Doctor Behavior**:
```
$ promptctl doctor --mode yolo

Claude Code
  ℹ Security checks disabled (yolo mode)
  ✓ 3 commands synced

$ promptctl doctor --mode strict  

Claude Code
  ✗ ERROR: permissions.deny not configured
  ✗ ERROR: Unknown MCP server detected
```

---

## TD-14: Atomic File Writes

**Decision**: Always write via temp file + rename

**Status**: Required (P3 pitfall mitigation)

**Problem**: IDE may lock `settings.json` while reading → write fails or corrupts file.

**Implementation**:
```rust
use tempfile::NamedTempFile;

fn atomic_write(path: &Path, content: &[u8]) -> Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut temp = NamedTempFile::new_in(dir)?;
    temp.write_all(content)?;
    
    // Atomic rename (POSIX guarantees atomicity)
    temp.persist(path)?;
    Ok(())
}
```

**Windows Handling**:
- `persist()` may fail if file locked → retry 3x with 100ms/500ms/1000ms backoff
- If still fails, report actionable error: "File locked by another process"

**Dependency**: `tempfile` crate

---

## TD-15: Lockfile for Change Detection

**Decision**: Track generated file hashes in `.calvin.lock`

**Status**: Required (P4 pitfall mitigation)

**Problem**: User edits generated file → Calvin watch overwrites their changes

**Lockfile Location**: `.promptpack/.calvin.lock`

**Format**:
```toml
# Auto-generated by Calvin. Do not edit.
[files]
".claude/settings.json" = { hash = "sha256:abc123...", generated_at = "2024-12-17T01:30:00Z" }
".cursor/rules/security/RULE.md" = { hash = "sha256:def456...", generated_at = "2024-12-17T01:30:00Z" }
```

**Sync Decision Tree**:
```
For each target file:
  1. Exists? NO → Write, record hash
  2. In lockfile? NO → User-created, SKIP (or --force)
  3. Hash matches lockfile? YES → Safe to overwrite
     NO → User modified! → SKIP + WARNING
          Use --force to overwrite
```

**Dependency**: `sha2` crate for hashing

---

## TD-16: Hardcoded Minimum Security

**Decision**: Even in `yolo` mode, generate minimum deny list

**Status**: Required (P6 pitfall mitigation)

**Problem**: Team sets `yolo`, new dev's Claude reads `.env` and private keys

**Hardcoded Deny List** (always applied):
```rust
const MINIMUM_DENY: &[&str] = &[
    ".env",
    ".env.*",
    "*.pem",
    "*.key",
    "id_rsa",
    "id_ed25519",
    ".git/",
];
```

**Opt-out**: Requires explicit `allow_naked = true` in config + generates warning:
```
⚠ WARNING: Security protections disabled!
  You have set allow_naked = true.
  .env, private keys, and .git are now visible to AI assistants.
  This is your responsibility.
```

---

## TD-17: Binary Size Optimization

**Decision**: Optimize release profile for size

**Status**: Planned

**Cargo.toml Profile**:
```toml
[profile.release]
lto = true              # Link-time optimization
codegen-units = 1       # Better optimization
strip = true            # Strip debug symbols
panic = "abort"         # Smaller binary
opt-level = "z"         # Optimize for size
```

**Dependency Minimization**:
```toml
[dependencies]
clap = { version = "4", default-features = false, features = ["std", "derive", "help"] }
serde = { version = "1", default-features = false, features = ["derive"] }
notify = { version = "6", default-features = false }
```

**Expected Size**:
| Config | Size |
|--------|------|
| Default | ~15-20 MB |
| Optimized | ~5-8 MB |

---

## TD-18: Versioned Adapters

**Decision**: Adapters must declare version compatibility

**Status**: Planned (P1 pitfall mitigation)

**Problem**: IDE updates path from `.cursor/rules/` to `.cursor/instructions/` → Calvin breaks

**Adapter Trait**:
```rust
pub trait TargetAdapter {
    fn name(&self) -> &str;
    fn min_version(&self) -> Option<SemVer>;
    fn max_tested_version(&self) -> Option<SemVer>;
    fn detect_version(&self) -> Result<SemVer, DetectionError>;
    fn compile(&self, asset: &PromptAsset, version: SemVer) -> Vec<OutputFile>;
}
```

**Doctor Integration**:
```
$ calvin doctor

Cursor v0.45.0 detected
  ⚠ WARNING: Calvin tested up to v0.42.x
  ⚠ Output format may have changed
```

---

## Cross-Reference

All pitfall mitigations are detailed in [docs/pitfall-mitigations.md](./pitfall-mitigations.md).
