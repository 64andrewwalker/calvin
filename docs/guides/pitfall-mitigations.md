# Pitfall Mitigations

> **Critical architectural decisions to prevent long-term maintenance death**
> 
> **Updated**: 2025-12-21

This document addresses the 7 fatal pitfalls identified during architecture review.

---

## P1: Target Fragility (IDE Vendor Changes)

**Risk**: `.cursor/rules/`, `.claude/settings.json` paths are unofficial and can change without notice.

### Mitigation: Versioned Adapters

```rust
pub trait TargetAdapter {
    /// Minimum supported version
    fn min_version(&self) -> Option<SemVer>;
    
    /// Maximum tested version (warning if exceeded)
    fn max_tested_version(&self) -> Option<SemVer>;
    
    /// Detect installed IDE version
    fn detect_version(&self) -> Result<SemVer, DetectionError>;
    
    /// Compile for specific version
    fn compile(&self, asset: &PromptAsset, version: SemVer) -> Vec<OutputFile>;
}
```

### Mitigation: Version Detection

Calvin does not currently detect installed IDE/app versions. Today you can check Calvin + adapter versions via:

```bash
$ calvin version

Calvin v0.2.0
Source Format: 1.0

Adapters:
  - ClaudeCode   v1
  - Cursor       v1
  - VSCode       v1
  - Antigravity  v1
  - Codex        v1
```

Future work (planned): surface compatibility checks via `calvin check`.

**Detection Methods**:
| IDE | Detection |
|-----|-----------|
| VS Code | `code --version` |
| Cursor | `cursor --version` or `~/.cursor/version` |
| Claude Code | Check CLI version or config file format |

### Mitigation: Community Feedback Loop

- **GitHub Issue Template**: "IDE Compatibility Report"
- **Auto-stale**: Adapters not validated in 90 days get deprecation warning
- **CI Canary**: Nightly job that installs latest IDE versions and validates output

---

## P2: Escaping Hell (JSON/TOML Corruption)

**Risk**: Simple `replace()` corrupts JSON when content contains quotes.

**Example Failure**:
```
Input:  Check if variable is named "foo"
Output: {"instruction": "Check if variable is named "foo""}
        ^^ BROKEN JSON ^^
```

### Mitigation: Context-Aware Escaping

```rust
pub enum OutputFormat {
    Markdown,       // No escaping needed
    Json,           // Escape quotes, backslashes, newlines
    Toml,           // Escape quotes in strings
    Yaml,           // Careful with special chars, multiline
    Raw,            // Pass through unchanged
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

### Rule: Never Generate Structured Data via String Concatenation

For JSON/TOML outputs, prefer:
```rust
// ✅ GOOD: Use serde to serialize
let settings = ClaudeSettings {
    permissions: Permissions {
        deny: vec![".env".into(), "secrets/**".into()],
    },
    instruction: user_content, // Serde handles escaping
};
serde_json::to_string_pretty(&settings)?

// ❌ BAD: String interpolation
format!(r#"{{"instruction": "{}"}}"#, user_content)
```

---

## P3: File Locking & Windows Sync

**Risk**: 
- IDE locks `settings.json` while reading → Calvin write fails/corrupts
- `rsync` unreliable on Windows without MSYS2/Git Bash

### Mitigation: Atomic Writes

```rust
use std::fs;
use std::path::Path;
use tempfile::NamedTempFile;

fn atomic_write(path: &Path, content: &[u8]) -> Result<()> {
    // 1. Write to temp file in same directory (ensures same filesystem)
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut temp = NamedTempFile::new_in(dir)?;
    temp.write_all(content)?;
    
    // 2. Atomic rename (POSIX guarantees atomicity)
    temp.persist(path)?;
    
    Ok(())
}
```

**Windows Considerations**:
- `persist()` on Windows may fail if file is locked → retry with backoff
- Add 3 retries with 100ms, 500ms, 1000ms delays

### Mitigation: Windows Remote Sync Fallback

```rust
fn sync_remote(src: &Path, dest: &str) -> Result<()> {
    if cfg!(windows) {
        // Windows: Prefer scp (built-in since Windows 10)
        sync_via_scp(src, dest)
    } else {
        // Unix: Use rsync for efficiency
        sync_via_rsync(src, dest)
    }
}

fn sync_via_scp(src: &Path, dest: &str) -> Result<()> {
    // scp is available on Windows 10+ without extra install
    Command::new("scp")
        .args(["-r", src.to_str()?, dest])
        .status()?;
    Ok(())
}
```

**Fallback Chain**:
1. Try `rsync` (most efficient)
2. Fall back to `scp -r` (Windows-native)
3. Error with clear message: "Install OpenSSH or rsync"

---

## P4: Idempotency vs User Edits

**Risk**: User manually edits generated file → Calvin watch overwrites their changes

### Mitigation: Lockfile with Checksums

```
.promptpack/
├── .calvin.lock       # Tracks generated file state
├── policies/
└── actions/
```

**Lockfile Format** (`.calvin.lock`):
```toml
# Auto-generated by Calvin. Do not edit.
version = 1

[files.".claude/settings.json"]
hash = "sha256:abc123..."

[files.".cursor/rules/security/RULE.md"]
hash = "sha256:def456..."
```

### Sync Decision Tree

```
For each target file:
  1. Does target file exist?
     NO  → Write file, record hash in lockfile
     YES → Continue
  
  2. Is file in lockfile?
     NO  → User created manually, SKIP (or --force to overwrite)
     YES → Continue
  
  3. Does current file hash match lockfile hash?
     YES → File unchanged since last sync, safe to overwrite
     NO  → User modified it!
           → Default: SKIP + WARNING
           → --force: Overwrite + WARNING
           → --yes: Non-interactive overwrite
```

**Output Example**:
```
$ calvin deploy

⚠ SKIPPED .claude/settings.json
  File was modified since last deploy (hash mismatch)
  Use --force (or --yes) to overwrite

✓ Updated .cursor/rules/security/RULE.md
✓ Updated .agent/workflows/pr-review.md

Summary: 2 updated, 1 skipped, 0 errors
```

---

## P5: Path Resolution Nightmare

**Risk**: Monorepo with `.promptpack/` at root but IDE workspace in subdirectory

### Mitigation: Project-Relative Paths Only

**Rule**: All resource references in source files MUST be relative to `.promptpack/`.

```yaml
---
description: Custom Linter
include: ./scripts/lint.sh  # Relative to .promptpack/
---
```

### Path Canonicalization at Compile Time

```rust
fn resolve_path(
    source_file: &Path,      // .promptpack/actions/lint.md
    reference: &str,         // ./scripts/lint.sh
    target_file: &Path,      // .cursor/commands/lint.md
) -> String {
    // 1. Resolve reference relative to source file's directory
    let source_dir = source_file.parent().unwrap();
    let absolute = source_dir.join(reference).canonicalize()?;
    
    // 2. Verify it's within the project root (security!)
    if !absolute.starts_with(&project_root) {
        return Err("Path escapes project boundary");
    }
    
    // 3. Compute relative path from target file's location
    let target_dir = target_file.parent().unwrap();
    pathdiff::diff_paths(&absolute, target_dir)
        .map(|p| p.to_string_lossy().to_string())
        .ok_or("Cannot compute relative path")
}
```

**Never**:
- Generate absolute paths in output files
- Allow paths that escape project root (`../../../etc/passwd`)

---

## P6: Yolo Mode Backfire

**Risk**: Team uses `yolo`, new dev's Claude reads `.env` and `id_rsa`

### Mitigation: Hardcoded Minimum Deny List

Even in `yolo` mode, Calvin generates a **minimum safety net**:

```rust
const HARDCODED_DENY: &[&str] = &[
    ".env",
    ".env.*",
    "*.pem",
    "*.key",
    "id_rsa",
    "id_ed25519",
    ".git/",
];

fn generate_deny_list(mode: SecurityMode, custom: &[String]) -> Vec<String> {
    match mode {
        SecurityMode::Yolo => {
            // Still include hardcoded minimum!
            HARDCODED_DENY.iter().map(|s| s.to_string()).collect()
        }
        SecurityMode::Balanced | SecurityMode::Strict => {
            let mut list: Vec<_> = HARDCODED_DENY.iter().map(|s| s.to_string()).collect();
            list.extend(custom.iter().cloned());
            list.extend(DEFAULT_SENSITIVE_PATTERNS.iter().map(|s| s.to_string()));
            list
        }
    }
}
```

### Mitigation: Audit Command for CI

```bash
$ calvin check --mode strict --strict-warnings

Checking security posture...
  ✗ FAIL: .claude/settings.json missing permissions.deny
  ✗ FAIL: MCP server "untrusted-tool" not in allowlist
  ⚠ WARN: Antigravity terminal mode is "turbo"

Exit code: 1 (security violations found)
```

**CI Integration** (`.github/workflows/security.yml`):
```yaml
- name: Calvin Security Audit
  run: calvin check --mode strict --strict-warnings
  # Blocks PR if security violations exist
```

### User Override

If someone truly wants to bypass minimum protections:
```toml
[security]
mode = "yolo"
allow_naked = true  # Explicit opt-in, generates scary warning
```

---

## P7: Binary Bloat

**Risk**: 20MB+ binary from clap + serde + tokio + askama

### Mitigation: Cargo Profile Optimization

```toml
# Cargo.toml

[profile.release]
lto = true              # Link-time optimization
codegen-units = 1       # Better optimization, slower compile
strip = true            # Strip debug symbols
panic = "abort"         # Smaller binary, no unwinding
opt-level = "z"         # Optimize for size (or "s" for balanced)

[profile.release.package."*"]
opt-level = "z"
```

### Mitigation: Dependency Audit

```toml
[dependencies]
# Minimize features
clap = { version = "4", default-features = false, features = ["std", "derive", "help"] }
serde = { version = "1", default-features = false, features = ["derive"] }
serde_json = { version = "1", default-features = false, features = ["std"] }
serde_yml = "0.0.12"  # Migrated from deprecated serde_yaml
notify = { version = "8", default-features = false, features = ["macos_kqueue"] }

# Avoid tokio if possible - use blocking sync for simplicity
# notify can work without async runtime
```

### Expected Binary Size

| Configuration | Size |
|--------------|------|
| Default release | ~15-20 MB |
| With optimizations | ~5-8 MB |
| With `upx` compression | ~2-3 MB |

**Note**: Don't use `upx` on macOS (code signing issues) or for distribution (antivirus false positives).

---

## Summary: Required Changes to Tech Decisions

| TD | Issue | Fix |
|----|-------|-----|
| TD-4 | Escaping hell | Add `OutputFormat` enum with context-aware escaping |
| TD-6 | Windows rsync | Add `scp` fallback, detect available tools |
| NEW | Atomic writes | Always write via temp file + rename |
| NEW | Lockfile | Add `.calvin.lock` for change detection |
| NEW | Path resolution | Always project-relative, canonicalize at compile |
| TD-13 | Yolo backfire | Hardcoded minimum deny list, add `audit` command |
| NEW | Binary size | Optimize Cargo profile, minimize features |

---

## Implementation Priority

1. **P0 (Before v0.1)**: Escaping, Atomic Writes, Lockfile
2. **P1 (v0.1)**: Windows scp fallback, Path resolution
3. **P2 (v0.2)**: Version detection, Audit command, Binary optimization
4. **P3 (v1.0)**: Canary testing, Structural merge
