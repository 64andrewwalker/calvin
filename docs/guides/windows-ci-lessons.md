# Windows CI & Testing Lessons

> Documenting hard-earned lessons from stabilizing Windows CI tests.

## The `dirs::home_dir()` Trap

### Problem
On Linux/macOS, the `dirs` crate respects the `HOME` environment variable. This makes it easy to mock the home directory in tests:

```rust
Command::new(bin)
    .env("HOME", temp_dir) // Works on Unix!
    // ...
```

**However**, on Windows, `dirs::home_dir()` uses the Windows system API (`SHGetKnownFolderPath`) to locate the profile directory. It **completely ignores** `HOME` and `USERPROFILE` environment variables.

This leads to tests that:
1. Fail to find mocked config files.
2. Accidentally read/write to the *actual* user's home directory on the CI runner (or local dev machine!).
3. Fail with permission errors or inconsistent state.

### Solution: Explicit Overrides

We cannot rely on OS-level overrides for the home directory on Windows. Instead, we must implement application-level overrides for critical paths.

**Implementation Pattern:**

In your application code (e.g., `src/infrastructure/repositories/registry.rs`):

```rust
fn default_registry_path() -> PathBuf {
    // 1. Check for a specific test/override env var first
    if let Ok(path) = std::env::var("CALVIN_REGISTRY_PATH") {
        return PathBuf::from(path);
    }
    
    // 2. Fall back to standard logic
    dirs::home_dir()
        .map(|h| h.join(".calvin/registry.toml"))
        .unwrap_or_else(|| PathBuf::from("~/.calvin/registry.toml"))
}
```

In your test code (e.g., `tests/common/env.rs`):

```rust
cmd.env("CALVIN_REGISTRY_PATH", temp_home.join(".calvin/registry.toml"));
```

### Checklist for Windows Compatibility

- [ ] Do not assume `HOME` is respected.
- [ ] Do not assume `USERPROFILE` is respected by `dirs` crate (though some other tools might).
- [ ] Always add specific env var overrides for paths derived from the home directory.
- [ ] Ensure `TestEnv` sets these overrides automatically for all tests.
- [ ] Use `std::path::PathBuf` for all path manipulation (avoid string concatenation which breaks on `\` vs `/`).

## Other Notes

- **Line Endings**: Windows uses `\r\n`. When comparing command output, ensure you normalize newlines or use loose matching.
- **Path Separators**: Use `.join()` instead of hardcoding `/`.
- **File Locking**: Windows is stricter about file locking. Ensure files are closed before trying to delete the directory containing them.

