# Calvinignore Support PRD

> **Status**: Proposed  
> **Author**: Arona (AI Assistant)  
> **Created**: 2025-12-28  
> **Updated**: 2025-12-28 (Self-Critique v2)  
> **Target Version**: 0.7.0

---

## Executive Summary

This PRD proposes adding **`.calvinignore`** support to Calvin. A `.calvinignore` file allows users to specify patterns for files and directories that Calvin should skip during asset loading.

**Key Decisions** (confirmed by Sensei):

1. Ignore rules **apply to skills and supplemental files**
2. `.calvinignore` takes **priority over frontmatter** (ignored files are never loaded)
3. **Each layer has its own `.calvinignore`** file

---

## Self-Critique: Issues in v1

Before proceeding, Arona identified these gaps in the initial draft:

| Issue | Problem | Resolution |
|-------|---------|------------|
| **Wrong crate choice** | `glob` crate doesn't support gitignore semantics (negation, directory hints) | Use `ignore` crate (by BurntSushi, same author as ripgrep) |
| **No performance analysis** | Large `.promptpack/` could be slow | Add caching, skip directory traversal for ignored dirs |
| **Windows path handling** | `/` vs `\` not addressed | Normalize to forward slash before matching |
| **No file size limit** | Huge `.calvinignore` could DoS | Limit to 1000 lines, 64KB |
| **Missing edge cases** | Already-tracked files, symlinks, encoding | Add explicit handling |
| **Incomplete error model** | Only "invalid pattern" error | Add line numbers, specific error types |
| **No debug command** | Users can't verify patterns | Add `calvin check --show-ignored` |
| **No caching** | Re-parsing on every deploy | Cache parsed patterns |

---

## 1. Problem Statement

### Current State

Calvin loads **all** Markdown files under `.promptpack/` directories:

- `policies/*.md`
- `actions/*.md`  
- `agents/*.md`
- `skills/<id>/SKILL.md` + supplementals

There's no way to exclude files without moving them outside `.promptpack/`.

### Pain Points

| Issue | Example | Impact |
|-------|---------|--------|
| Draft files loaded | `drafts/wip-feature.md` | Broken config deployed |
| Template sources compiled | `templates/*.tmpl` | Template syntax errors |
| Test fixtures loaded | `fixtures/test-policy.md` | Invalid test data in prod |
| README compiled as asset | `README.md` | Documentation treated as action |

---

## 2. Goals

### Primary Goals

1. Add `.calvinignore` file support in `.promptpack/`
2. Use gitignore-compatible pattern syntax (via `ignore` crate)
3. Apply ignore rules during asset loading (before compilation)
4. Support ignore files in each layer (user, team, project)
5. Apply to all asset types including skill supplementals

### Non-Goals

- **NOT** implementing `.calvinignore` merging across layers
- **NOT** adding CLI override for ignore patterns (use frontmatter `targets: []` instead)
- **NOT** supporting git-style per-directory `.calvinignore` nesting

### Success Criteria

1. User creates `.promptpack/.calvinignore` with pattern `drafts/`
2. Files in `.promptpack/drafts/` are not loaded
3. `calvin deploy -v` shows ignored file count
4. `calvin check --show-ignored` reveals what's being ignored
5. Skill supplemental files matching ignore patterns are skipped

---

## 3. Design Philosophy

### Principle 1: Use Battle-Tested Crate

**Decision**: Use the [`ignore`](https://docs.rs/ignore) crate instead of `glob`.

| Crate | Pros | Cons |
|-------|------|------|
| `glob` | Simple, minimal | No gitignore semantics, no negation |
| `globset` | Fast multi-pattern | No gitignore directory handling |
| **`ignore`** ✓ | Full gitignore semantics, fast, proven in ripgrep | Larger dependency |
| `gitignore` | Small | Unmaintained |

**Rationale**: `ignore` is used by ripgrep, fd, and delta. It handles:

- Negation patterns (`!important.md`)
- Directory-specific matching (`logs/` vs `logs`)
- Nested `.gitignore` files (we won't use this, but it's robust)
- Windows path normalization

### Principle 2: Load-Time Filtering with Early Directory Skip

```
load_layer(path)
  → load_calvinignore(path)
  → walk_directory(path)
      ↳ on_dir: if dir matches ignore → SKIP ENTIRE SUBTREE
      ↳ on_file: if file matches ignore → skip
  → parse_remaining_files
  → return assets
```

**Performance**: By skipping ignored directories during traversal (not after), we avoid walking large ignored trees like `node_modules/` (if someone has it in `.promptpack/` for some reason).

### Principle 3: Layered Isolation

Each layer has its own `.calvinignore`:

```
~/.calvin/.promptpack/.calvinignore     # User layer
/team/shared/.promptpack/.calvinignore  # Team layer
./project/.promptpack/.calvinignore     # Project layer
```

**Semantics**:

- No inheritance between layers
- Each layer's ignore rules only affect that layer's files
- Override behavior unchanged: higher-priority layer's assets replace lower-priority

### Principle 4: Priority Over Frontmatter

If a file matches `.calvinignore`, it is **never loaded**:

| File | Frontmatter | .calvinignore | Result |
|------|-------------|---------------|--------|
| `policy.md` | `targets: all` | (no match) | ✓ Loaded |
| `policy.md` | `targets: all` | `policy.md` | ✗ Ignored |
| `policy.md` | `targets: []` | (no match) | ✓ Loaded (empty targets) |

**Implementation Note**: Since ignored files are never parsed, frontmatter validation errors in ignored files don't surface.

### Principle 5: Fail Loudly with Context

| Situation | Behavior |
|-----------|----------|
| Invalid pattern syntax | **Error** with line number + suggestion |
| `.calvinignore` too large (>64KB) | **Error**: "File exceeds 64KB limit" |
| Too many patterns (>1000) | **Error**: "Exceeds 1000 pattern limit" |
| Empty `.calvinignore` | Silent (no-op) |
| `.calvinignore` not found | Silent (no ignore rules) |
| All assets in layer ignored | **Warning**: "No assets loaded from layer" |
| Ignored directory contains SKILL.md | **Info** (verbose): "Skipped skill: ..." |

---

## 4. Detailed Design

### 4.1 File Format Specification

**Location**: `.promptpack/.calvinignore`  
**Encoding**: UTF-8 (BOM ignored if present)  
**Max Size**: 64 KB  
**Max Lines**: 1000 patterns

**Syntax** (gitignore-compatible subset):

| Pattern | Meaning | Example Match |
|---------|---------|---------------|
| `foo` | File or dir named `foo` anywhere | `foo`, `a/foo`, `a/b/foo` |
| `/foo` | Only at root of `.promptpack/` | `foo` (not `a/foo`) |
| `foo/` | Directory named `foo` (skip subtree) | `foo/**` |
| `*.md` | All `.md` files | `README.md`, `a/b/test.md` |
| `**/test` | `test` at any depth | `test`, `a/test`, `a/b/test` |
| `a/**/b` | Any path from `a` to `b` | `a/b`, `a/x/b`, `a/x/y/b` |
| `!important.md` | Negate: DO include this file | (re-include previously ignored) |
| `# comment` | Comment line | (ignored) |
| (blank line) | Ignored | (ignored) |

**Not Supported** (by design):

| Feature | Reason |
|---------|--------|
| Per-directory `.calvinignore` | Adds complexity, rarely needed |
| `[abc]` character classes | Rarely used in this context |

### 4.2 Implementation Using `ignore` Crate

```rust
use ignore::gitignore::{Gitignore, GitignoreBuilder};

pub struct IgnorePatterns {
    matcher: Gitignore,
    pattern_count: usize,
    ignored_files: Vec<PathBuf>,  // For verbose output
}

impl IgnorePatterns {
    pub fn load(promptpack_path: &Path) -> Result<Self, IgnoreError> {
        let ignore_path = promptpack_path.join(".calvinignore");
        
        if !ignore_path.exists() {
            return Ok(Self::empty());
        }
        
        // Size check
        let metadata = fs::metadata(&ignore_path)?;
        if metadata.len() > 65536 {
            return Err(IgnoreError::FileTooLarge(ignore_path));
        }
        
        // Parse
        let mut builder = GitignoreBuilder::new(promptpack_path);
        let content = fs::read_to_string(&ignore_path)?;
        let mut pattern_count = 0;
        
        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            pattern_count += 1;
            if pattern_count > 1000 {
                return Err(IgnoreError::TooManyPatterns(ignore_path));
            }
            
            if let Err(e) = builder.add_line(Some(ignore_path.clone()), line) {
                return Err(IgnoreError::InvalidPattern {
                    path: ignore_path,
                    line: line_num + 1,
                    pattern: line.to_string(),
                    message: e.to_string(),
                });
            }
        }
        
        let matcher = builder.build()?;
        
        Ok(Self {
            matcher,
            pattern_count,
            ignored_files: Vec::new(),
        })
    }
    
    pub fn is_ignored(&self, rel_path: &Path, is_dir: bool) -> bool {
        self.matcher
            .matched_path_or_any_parents(rel_path, is_dir)
            .is_ignore()
    }
}
```

### 4.3 Directory Traversal with Early Skip

```rust
fn walk_with_ignore(
    root: &Path,
    ignore: &IgnorePatterns,
) -> Vec<PathBuf> {
    let mut files = Vec::new();
    
    for entry in WalkDir::new(root)
        .follow_links(false)  // Security: don't follow symlinks
        .into_iter()
        .filter_entry(|e| {
            let rel_path = e.path().strip_prefix(root).unwrap_or(e.path());
            let is_dir = e.file_type().is_dir();
            
            // Early skip: if directory matches, skip entire subtree
            if is_dir && ignore.is_ignored(rel_path, true) {
                return false;  // Don't descend
            }
            
            true
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,  // Permission error, etc.
        };
        
        if entry.file_type().is_file() {
            let rel_path = entry.path().strip_prefix(root).unwrap();
            if !ignore.is_ignored(rel_path, false) {
                files.push(entry.path().to_path_buf());
            }
        }
    }
    
    files
}
```

### 4.4 Windows Path Normalization

The `ignore` crate handles this internally, but we ensure consistent behavior:

```rust
fn normalize_path(path: &Path) -> String {
    // Convert Windows backslashes to forward slashes
    path.to_string_lossy().replace('\\', "/")
}
```

### 4.5 Caching Strategy

Ignore patterns are parsed once per layer per operation:

```rust
pub struct LayerWithIgnore {
    pub path: PathBuf,
    pub ignore_patterns: IgnorePatterns,  // Cached
    pub assets: Vec<Asset>,
    pub ignored_count: usize,
}

impl LayerLoader {
    fn load_layer(&self, path: &Path) -> Result<LayerWithIgnore, LayerError> {
        // Parse once
        let ignore_patterns = IgnorePatterns::load(path)?;
        
        // Use for all file scanning
        let files = walk_with_ignore(path, &ignore_patterns);
        
        // ... parse assets ...
        
        Ok(LayerWithIgnore {
            path: path.to_path_buf(),
            ignore_patterns,
            assets,
            ignored_count,
        })
    }
}
```

### 4.6 CLI Changes

> **UX Principle**: Never spam the terminal. `node_modules/` could match 20,000+ files.

#### `calvin deploy -v` (Verbose)

Shows **counts only**, never lists individual files:

```text
$ calvin deploy -v

Layer Stack:
  project  ./.promptpack  (5 assets, 2 skills)
           .calvinignore: 3 patterns, 47 files skipped
  user     ~/.calvin/.promptpack  (3 assets, 1 skill)
           .calvinignore: 1 pattern, 2 files skipped
```

#### `calvin check --show-ignored`

Shows **patterns** (not files):

```text
$ calvin check --show-ignored

.calvinignore Patterns:
  project (./.promptpack/.calvinignore):
    1. drafts/
    2. *.bak
    3. README.md
    → 47 files matched
    
  user (~/.calvin/.promptpack/.calvinignore):
    1. experiments/
    → 2 files matched
```

#### `calvin check --debug-ignore` (New, for debugging)

Shows **individual files** — use sparingly:

```text
$ calvin check --debug-ignore

Ignored Files (project layer):
  - drafts/wip-feature.md           (pattern: drafts/)
  - drafts/idea.md                  (pattern: drafts/)
  - README.md                       (pattern: README.md)
  - old-policy.bak                  (pattern: *.bak)
  ... (47 files total)
```

> ⚠️ **Warning**: This can produce very long output. Use `--debug-ignore 2>/dev/null | head -50` for large projects.

### 4.7 Error Types

```rust
pub enum IgnoreError {
    FileTooLarge(PathBuf),
    TooManyPatterns(PathBuf),
    InvalidPattern {
        path: PathBuf,
        line: usize,
        pattern: String,
        message: String,
    },
    IoError(std::io::Error),
}

impl Display for IgnoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileTooLarge(path) => 
                write!(f, ".calvinignore exceeds 64KB limit: {}", path.display()),
            Self::TooManyPatterns(path) => 
                write!(f, ".calvinignore exceeds 1000 pattern limit: {}", path.display()),
            Self::InvalidPattern { path, line, pattern, message } => 
                write!(f, "Invalid pattern at {}:{}: '{}' - {}", 
                    path.display(), line, pattern, message),
            Self::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}
```

---

## 5. Edge Cases

### 5.1 Symlinks

**Behavior**: Symlinks are NOT followed (security).

```
.promptpack/
├── policies/
│   └── real.md
└── external -> /etc/passwd  # ← NOT followed, NOT loaded
```

**Implementation**: `WalkDir::new().follow_links(false)`

### 5.2 File Encoding

**Behavior**: `.calvinignore` must be valid UTF-8.

```rust
let content = fs::read_to_string(&ignore_path)
    .map_err(|e| IgnoreError::IoError(e))?;
```

Non-UTF-8 files produce an error (not silent failure).

### 5.3 Negation with Directory Ignore

**Behavior**: Negation cannot re-include files if parent is ignored (gitignore semantics).

```gitignore
drafts/           # Ignore entire directory
!drafts/keep.md   # ❌ WON'T WORK - parent is ignored
```

**Solution**: Don't ignore the directory:

```gitignore
drafts/*.md       # Ignore files in drafts
!drafts/keep.md   # ✓ Re-include keep.md
```

### 5.4 Hidden Files

Calvin already ignores hidden files (`.`-prefixed). `.calvinignore` patterns can explicitly include them:

```gitignore
!.hidden-but-important.md
```

### 5.5 SKILL.md Ignored

If a skill's `SKILL.md` is ignored, the entire skill is ignored (no warning about missing SKILL.md):

```gitignore
skills/draft-skill/  # Entire skill ignored
```

---

## 6. Implementation Plan (TDD)

### Phase 1: Domain Layer

| Contract | Test | Status |
|----------|------|--------|
| IGNORE-001 | Load patterns from file | [ ] |
| IGNORE-002 | Empty file = empty patterns | [ ] |
| IGNORE-003 | Missing file = empty patterns | [ ] |
| IGNORE-004 | File too large = error | [ ] |
| IGNORE-005 | Too many patterns = error | [ ] |
| IGNORE-006 | Invalid pattern = error with line number | [ ] |
| IGNORE-007 | Pattern matches file | [ ] |
| IGNORE-008 | Pattern matches directory recursively | [ ] |
| IGNORE-009 | Negation re-includes file | [ ] |
| IGNORE-010 | `**` matches any depth | [ ] |

**Files**:

- `src/domain/value_objects/ignore_patterns.rs`

### Phase 2: Repository Integration

| Contract | Test | Status |
|----------|------|--------|
| REPO-001 | Ignored files not loaded | [ ] |
| REPO-002 | Ignored directories skipped (performance) | [ ] |
| REPO-003 | Skill supplementals respect ignore | [ ] |
| REPO-004 | Layer tracks ignored count | [ ] |
| REPO-005 | Symlinks not followed | [ ] |

**Files**:

- `src/infrastructure/layer/loader.rs`
- `src/infrastructure/repositories/asset.rs`

### Phase 3: CLI & UI

| Contract | Test | Status |
|----------|------|--------|
| CLI-001 | `deploy -v` shows ignored count per layer | [ ] |
| CLI-002 | `check --show-ignored` lists patterns | [ ] |
| CLI-003 | `check --debug-ignore` lists individual files | [ ] |
| CLI-004 | JSON output includes `ignored_count` | [ ] |
| CLI-005 | Errors include line numbers | [ ] |

**Files**:

- `src/commands/deploy/cmd.rs`
- `src/commands/check/engine.rs`

### Phase 4: Documentation

- [ ] `docs/guides/calvinignore.mdx`
- [ ] `docs-content` user guide
- [ ] Update `calvin init` templates
- [ ] CHANGELOG entry

---

## 7. Security Considerations

| Risk | Mitigation |
|------|------------|
| Path traversal (`../`) | Paths normalized relative to `.promptpack/` |
| Symlink escape | `follow_links(false)` |
| DoS via huge file | 64KB limit |
| DoS via pattern explosion | 1000 pattern limit |
| Timing attacks | Not applicable (local files) |

---

## 8. Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `ignore` | `0.4` | Gitignore pattern matching |
| (existing) `walkdir` | - | Directory traversal |

**Binary size impact**: `ignore` adds ~50KB to release binary (acceptable).

---

## 9. Testing Strategy

### Unit Tests

```rust
#[test]
fn test_ignore_file_pattern() {
    let patterns = IgnorePatterns::from_lines(&["README.md"]);
    assert!(patterns.is_ignored(Path::new("README.md"), false));
    assert!(!patterns.is_ignored(Path::new("other.md"), false));
}

#[test]
fn test_ignore_directory_skips_subtree() {
    let patterns = IgnorePatterns::from_lines(&["drafts/"]);
    assert!(patterns.is_ignored(Path::new("drafts"), true));
    assert!(patterns.is_ignored(Path::new("drafts/a.md"), false));
    assert!(patterns.is_ignored(Path::new("drafts/nested/b.md"), false));
}

#[test]
fn test_negation_re_includes() {
    let patterns = IgnorePatterns::from_lines(&["*.md", "!important.md"]);
    assert!(patterns.is_ignored(Path::new("test.md"), false));
    assert!(!patterns.is_ignored(Path::new("important.md"), false));
}

#[test]
fn test_file_too_large_error() {
    let env = TestEnv::new();
    env.write(".promptpack/.calvinignore", &"x\n".repeat(100_000));
    
    let result = IgnorePatterns::load(&env.promptpack_path());
    assert!(matches!(result, Err(IgnoreError::FileTooLarge(_))));
}
```

### Integration Tests

```rust
#[test]
fn deploy_respects_calvinignore() {
    let env = TestEnv::new();
    env.write(".promptpack/.calvinignore", "drafts/\n");
    env.write(".promptpack/drafts/wip.md", "---\ndescription: WIP\n---\n");
    env.write(".promptpack/policies/real.md", "---\ndescription: Real\n---\n");
    
    let result = env.run(["calvin", "deploy", "-v"]);
    
    assert!(result.stdout.contains("1 asset"));
    assert!(result.stdout.contains("1 ignored"));
    assert!(!result.stdout.contains("wip"));
}

#[test]
fn check_show_ignored_lists_files() {
    let env = TestEnv::new();
    env.write(".promptpack/.calvinignore", "drafts/\n");
    env.write(".promptpack/drafts/wip.md", "---\ndescription: WIP\n---\n");
    
    let result = env.run(["calvin", "check", "--show-ignored"]);
    
    assert!(result.stdout.contains("drafts/wip.md"));
    assert!(result.stdout.contains("matched: drafts/"));
}
```

---

## 10. Open Questions

### Resolved

| Question | Decision | Rationale |
|----------|----------|-----------|
| Which crate? | `ignore` | Battle-tested, gitignore-compatible |
| Apply to skills? | Yes | Consistent behavior |
| Priority vs frontmatter? | Ignore wins | Load-time filtering |
| Per-layer or global? | Per-layer | Layer isolation principle |
| Support negation? | Yes | Users expect gitignore semantics |

### Future Considerations

| Feature | Status |
|---------|--------|
| Global `~/.config/calvin/.calvinignore` | Deferred to v0.8+ |
| Per-directory nested `.calvinignore` | Not planned |
| `calvin init` template with examples | Phase 4 |

---

## 11. References

- [`ignore` crate docs](https://docs.rs/ignore) — Implementation reference
- [gitignore specification](https://git-scm.com/docs/gitignore) — Syntax reference
- [Skills Support PRD](./skills-support-prd.md) — Similar directory-based feature
- [Multi-Layer PRD](./multi-layer/) — Layer isolation principles
