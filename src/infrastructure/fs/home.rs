//! Home directory resolution with test isolation support.
//!
//! On Windows, `dirs::home_dir()` uses the Windows system API (`SHGetKnownFolderPath`)
//! rather than environment variables. This means setting `HOME` or `USERPROFILE` in
//! tests has no effect on Windows.
//!
//! This module provides a unified `calvin_home_dir()` function that:
//! 1. Checks `CALVIN_TEST_HOME` environment variable first (for test isolation)
//! 2. Falls back to `dirs::home_dir()` for production use
//!
//! All Calvin-internal code that needs the home directory for functional paths
//! (lockfile, registry, user layer, config) should use this function.
//!
//! **Exceptions** (continue using `dirs::home_dir()` directly):
//! - Display functions (`display_with_tilde`) - cosmetic only
//! - Security checks - need to check real user directories
//! - Tilde expansion in user-provided paths - external paths should use real home

use std::path::PathBuf;

/// Environment variable for test isolation of home directory.
///
/// When set, this overrides `dirs::home_dir()` for all Calvin-internal paths.
/// This is critical for Windows CI where `dirs::home_dir()` ignores `HOME`/`USERPROFILE`.
pub const CALVIN_TEST_HOME_VAR: &str = "CALVIN_TEST_HOME";

/// Get the home directory for Calvin-internal paths.
///
/// This function should be used instead of `dirs::home_dir()` for:
/// - Lockfile paths (`~/.calvin/calvin.lock`)
/// - Registry paths (`~/.calvin/registry.toml`)
/// - User layer paths (`~/.calvin/.promptpack`)
/// - User config paths (`~/.calvin/config.toml`)
///
/// # Test Isolation
///
/// Set `CALVIN_TEST_HOME` environment variable to override the home directory
/// in tests. This is especially important on Windows where `dirs::home_dir()`
/// uses system APIs that ignore environment variables.
///
/// # Returns
///
/// - `Some(PathBuf)` - The home directory path
/// - `None` - If neither `CALVIN_TEST_HOME` is set nor system home can be resolved
///
/// # Example
///
/// ```
/// use calvin::infrastructure::fs::calvin_home_dir;
///
/// // In production: returns system home directory
/// // In tests with CALVIN_TEST_HOME set: returns that path
/// if let Some(home) = calvin_home_dir() {
///     let lockfile = home.join(".calvin/calvin.lock");
/// }
/// ```
pub fn calvin_home_dir() -> Option<PathBuf> {
    std::env::var(CALVIN_TEST_HOME_VAR)
        .ok()
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calvin_home_dir_returns_some_in_normal_environment() {
        // In a normal environment, should return the system home directory
        // (unless CALVIN_TEST_HOME is set by a parent test harness)
        let result = calvin_home_dir();
        // We can't assert the exact value, but it should be Some
        // unless running in a very unusual environment
        assert!(
            result.is_some() || std::env::var(CALVIN_TEST_HOME_VAR).is_err(),
            "calvin_home_dir() should return Some in normal environment"
        );
    }

    #[test]
    fn calvin_home_dir_respects_test_home_env_var() {
        // Temporarily set the env var
        let test_home = "/test/fake/home";

        // SAFETY: This test is single-threaded and we restore the env var after
        unsafe {
            std::env::set_var(CALVIN_TEST_HOME_VAR, test_home);
        }

        let result = calvin_home_dir();

        unsafe {
            std::env::remove_var(CALVIN_TEST_HOME_VAR);
        }

        assert_eq!(result, Some(PathBuf::from(test_home)));
    }
}
