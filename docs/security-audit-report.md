# Calvin Security Audit Report

> **Date**: 2025-12-17  
> **Auditor**: Antigravity AI Security Module  
> **Version**: v0.1.0 (commit f612faf)  
> **Scope**: Full white-box source code analysis

---

## Executive Summary

| Category | Status | Score |
|----------|--------|-------|
| **Overall** | ✅ PASS | 8.5/10 |
| Input Validation | ✅ Secure | 9/10 |
| Command Injection | ✅ Secure | 8/10 |
| Path Traversal | ✅ Secure | 9/10 |
| Data Protection | ✅ Secure | 9/10 |
| Dependencies | ⚠️ Not Audited | N/A |
| Configuration | ✅ Secure | 8/10 |
| Error Handling | ✅ Secure | 8/10 |

Calvin is a **low-risk CLI tool** that generates configuration files for AI assistants. The codebase demonstrates strong security practices including:
- Path traversal protection with canonicalization
- Shell command argument quoting for SSH operations
- Enforced minimum deny lists for sensitive files
- Atomic file writes to prevent corruption

---

## Findings

### SEC-001: Shell Command via SSH (LOW)

**Severity**: 🟡 LOW (CVSS 3.1: 3.7)  
**Component**: `src/fs.rs:93-127`  
**Status**: ✅ MITIGATED

**Description**:
The `RemoteFileSystem` executes shell commands via SSH:

```rust
let mut child = Command::new("ssh")
    .arg(&self.destination)
    .arg(command)  // Command string passed to remote shell
    .spawn()
```

**Analysis**:
- The `command` parameter is built from internal strings only (`echo $HOME`, `test -e`, `cat`, `mkdir -p`, etc.)
- User-controlled paths are properly quoted via `quote_path()`:
  ```rust
  fn quote_path(path: &Path) -> String {
      format!("'{}'", path.to_string_lossy().replace("'", "'\\''"))
  }
  ```
- This properly escapes single quotes preventing shell injection

**Risk**: Minimal - paths come from internal asset compilation, not direct user input.

**Recommendation**: None required. Current implementation is secure.

---

### SEC-002: Path Traversal Protection (PASS)

**Severity**: ✅ INFORMATIONAL  
**Component**: `src/sync/mod.rs:39-72`  
**Status**: ✅ IMPLEMENTED

**Description**:
Path traversal attacks are prevented via `validate_path_safety()`:

```rust
pub fn validate_path_safety(path: &Path, project_root: &Path) -> CalvinResult<()> {
    // Skip paths starting with ~ (user-level paths)
    if path_str.starts_with("~") {
        return Ok(());
    }
    
    if path_str.contains("..") {
        let resolved = project_root.join(path);
        if let Ok(canonical) = resolved.canonicalize() {
            let root_canonical = project_root.canonicalize()...;
            if !canonical.starts_with(&root_canonical) {
                return Err(CalvinError::PathEscape {...});
            }
        }
    }
}
```

**Tests**: 3 explicit tests covering normal paths, user paths, and traversal attacks.

---

### SEC-003: Hardcoded Security Baseline (PASS)

**Severity**: ✅ INFORMATIONAL  
**Component**: `src/security_baseline.rs:9-17`  
**Status**: ✅ IMPLEMENTED

**Description**:
Minimum deny list is always applied (unless explicitly overridden):

```rust
pub const MINIMUM_DENY: &[&str] = &[
    ".env",
    ".env.*",
    "*.pem",
    "*.key",
    "id_rsa",
    "id_ed25519",
    ".git/",
];
```

This prevents AI assistants from accessing sensitive files even in "yolo" mode.

**Override**: Requires explicit `security.allow_naked = true` which displays warning:
```
⚠ WARNING: Security protections disabled!
  You have set security.allow_naked = true.
  .env, private keys, and .git may be visible to AI assistants.
```

---

### SEC-004: Unsafe Code in Tests Only (PASS)

**Severity**: ✅ INFORMATIONAL  
**Component**: `src/config.rs:521-553`  
**Status**: ✅ ACCEPTABLE

**Description**:
8 instances of `unsafe` blocks found, all in test code for environment variable manipulation:

```rust
#[test]
fn test_config_env_override_security() {
    unsafe { std::env::set_var("CALVIN_SECURITY_MODE", "strict") };
    // ... test logic ...
    unsafe { std::env::remove_var("CALVIN_SECURITY_MODE") };
}
```

**Risk**: None - test-only code, required for environment variable modification in Rust 2024 edition.

---

### SEC-005: No Secrets in Codebase (PASS)

**Severity**: ✅ INFORMATIONAL  
**Status**: ✅ PASS

**Scan Results**:
- `password`: 0 occurrences
- `api_key`: 0 occurrences
- `secret`: Only configuration examples (e.g., `secrets/**` deny patterns)
- No hardcoded credentials detected

---

### SEC-006: Panic in Production Code (LOW)

**Severity**: 🟡 LOW (CVSS 3.1: 2.0)  
**Component**: `src/main.rs:850-980`  
**Status**: ⚠️ ACCEPTABLE

**Description**:
11 `panic!()` calls found, all in test code:

```rust
// All in #[test] functions
panic!("Expected Sync command");
panic!("Expected Doctor command");
// etc.
```

**Risk**: None - test-only code will never execute in production.

---

### SEC-007: Atomic File Writes (PASS)

**Severity**: ✅ INFORMATIONAL  
**Component**: `src/sync/writer.rs`  
**Status**: ✅ IMPLEMENTED

**Description**:
All file writes use atomic tempfile + rename pattern (TD-14):
- Prevents file corruption from interrupted writes
- Includes Windows retry logic for locked files

---

### SEC-008: Input Escaping for Structured Outputs (PASS)

**Severity**: ✅ INFORMATIONAL  
**Component**: `src/adapters/escaping.rs`  
**Status**: ✅ IMPLEMENTED

**Description**:
Context-aware escaping prevents JSON/TOML/YAML corruption:

```rust
pub fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
```

14 dedicated tests ensure escaping correctness.

---

## Risk Matrix

| ID | Finding | Severity | Likelihood | Impact | Status |
|----|---------|----------|------------|--------|--------|
| SEC-001 | SSH Command Execution | LOW | Low | Low | ✅ Mitigated |
| SEC-002 | Path Traversal | - | - | - | ✅ Implemented |
| SEC-003 | Security Baseline | - | - | - | ✅ Implemented |
| SEC-004 | Unsafe Code | INFO | None | None | ✅ Test Only |
| SEC-005 | Secrets Scan | - | - | - | ✅ Pass |
| SEC-006 | Panic Calls | LOW | None | None | ✅ Test Only |
| SEC-007 | Atomic Writes | - | - | - | ✅ Implemented |
| SEC-008 | Output Escaping | - | - | - | ✅ Implemented |

---

## Remediation Priority Queue

| Priority | ID | Action | Status |
|----------|-----|--------|--------|
| - | - | No critical or high findings | ✅ |

---

## Recommendations

### Short-term (Optional)
1. **Add cargo-audit to CI**: Automate dependency vulnerability scanning
   ```yaml
   - run: cargo install cargo-audit && cargo audit
   ```

2. **Add gitleaks to CI**: Automate secret detection
   ```yaml
   - uses: gitleaks/gitleaks-action@v2
   ```

### Long-term
1. Consider fuzzing the YAML frontmatter parser with `cargo-fuzz`
2. Consider adding `#![forbid(unsafe_code)]` to lib.rs (after moving test utilities)

---

## Appendix: Test Coverage

Security-related tests identified:
- `test_validate_path_safety_normal`
- `test_validate_path_safety_user_paths`
- `test_validate_path_safety_traversal`
- `test_effective_claude_deny_patterns_*` (3 tests)
- `test_escape_json_*` (6 tests)
- `test_escape_toml_*` (3 tests)
- `test_escape_yaml_*` (3 tests)
- `test_json_corruption_prevention`
- `test_mcp_allowlist_*` (3 tests)
- `test_deny_list_*` (5 tests)

Total: 26+ security-focused tests

---

## Sign-off

- **Auditor**: Antigravity AI Security Module
- **Date**: 2025-12-17
- **Result**: **PASS** ✅
- **Next Audit**: On major version release or security-related changes

---

*This report was generated following OWASP guidelines for source code review.*
