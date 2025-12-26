# Windows CI Lessons Learned

> **Created**: 2025-12-26
> **Status**: Active guideline

This document captures lessons learned from Windows CI failures and provides guidelines to prevent similar issues in the future.

## Problem 1: `dirs::home_dir()` Ignores Environment Variables

### Issue

On Windows, `dirs::home_dir()` uses the Windows system API (`SHGetKnownFolderPath`) instead of environment variables like `HOME` or `USERPROFILE`. This means:

- Setting `HOME` or `USERPROFILE` in tests has no effect on Windows
- Tests that rely on isolated home directories fail because paths resolve to the real user home

### Solution

We created a unified `calvin_home_dir()` function in `src/infrastructure/fs/home.rs`:

```rust
pub fn calvin_home_dir() -> Option<PathBuf> {
    std::env::var("CALVIN_TEST_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
}
```

### Rules for New Code

1. **NEVER** use `dirs::home_dir()` directly for functional paths (lockfile, registry, config, user layer)
2. **ALWAYS** use `calvin_home_dir()` or `crate::infrastructure::calvin_home_dir()` instead
3. **EXCEPTION**: Security checks and display functions that need the real home directory can use `dirs::home_dir()` directly (document why)

### For Domain Layer

Since `domain/` cannot depend on `infrastructure/`, use direct env var check:

```rust
let home = std::env::var("CALVIN_TEST_HOME")
    .ok()
    .map(PathBuf::from)
    .or_else(dirs::home_dir);
```

## Problem 2: Windows Path Escaping in TOML

### Issue

When writing TOML files in tests with paths that include the temp directory:

```rust
fs::write(
    config_path,
    format!(r#"user_layer_path = "{}""#, path.display()),
)
```

On Windows, `path.display()` produces `C:\Users\...\path`, but backslashes `\` are escape characters in TOML strings. This causes parsing errors or incorrect paths.

### Solution

Replace backslashes with forward slashes before writing to TOML:

```rust
let toml_safe_path = path.display().to_string().replace('\\', "/");
fs::write(
    config_path,
    format!(r#"user_layer_path = "{}""#, toml_safe_path),
)
```

### Rules for New Tests

1. **ALWAYS** escape paths before writing to TOML files
2. Consider creating a helper function:

```rust
fn toml_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}
```

## CI Verification Checklist

Before adding new features that involve:

- [ ] Home directory paths → Use `calvin_home_dir()`
- [ ] User config files → Use `calvin_home_dir()` for base path
- [ ] Writing paths to TOML → Escape backslashes
- [ ] Test isolation → Use `TestEnv` builder or `WindowsCompatExt`

## Quick Reference

| Scenario | Solution |
|----------|----------|
| Need home dir in production code | `crate::infrastructure::calvin_home_dir()` |
| Need home dir in domain layer | Direct `CALVIN_TEST_HOME` env var check |
| Writing path to TOML in test | `path.display().to_string().replace('\\', "/")` |
| Running CLI in test | Use `WindowsCompatExt::with_test_home()` |
| Creating isolated test env | Use `TestEnv::builder()` |

## Related Files

- `src/infrastructure/fs/home.rs` - `calvin_home_dir()` definition
- `tests/common/windows.rs` - `WindowsCompatExt` trait
- `tests/common/env.rs` - `TestEnv` builder
- `docs/archive/windows-home-dir-unification-plan.md` - Implementation details
