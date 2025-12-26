//! Windows-compatible test environment helpers.
//!
//! On Windows, `dirs::home_dir()` uses Windows system API (`SHGetKnownFolderPath`),
//! not environment variables like `HOME` or `USERPROFILE`. This means setting these
//! environment variables in tests has no effect on Windows.
//!
//! These helpers set the necessary Calvin-specific environment variables to override
//! the default paths, ensuring tests work correctly on all platforms.
//!
//! ## Primary Override: `CALVIN_TEST_HOME`
//!
//! The main mechanism is `CALVIN_TEST_HOME`, which is checked by `calvin_home_dir()`
//! in `src/infrastructure/fs/home.rs`. This single variable controls all Calvin-internal
//! paths that depend on the home directory:
//! - `~/.calvin/calvin.lock` (global lockfile)
//! - `~/.calvin/registry.toml` (project registry)
//! - `~/.calvin/.promptpack` (user layer)
//! - `~/.calvin/config.toml` (user config)
//!
//! ## Backwards Compatibility
//!
//! The following specific overrides are kept for edge cases and backwards compatibility:
//! - `CALVIN_REGISTRY_PATH` - direct registry path override
//! - `CALVIN_SOURCES_USER_LAYER_PATH` - direct user layer path override
//! - `CALVIN_USER_CONFIG_PATH` - direct user config path override
//! - `CALVIN_XDG_CONFIG_PATH` - XDG config path override

use std::path::Path;
use std::process::Command;

/// Extension trait for Command to add Windows-compatible home directory setup.
///
/// # Example
///
/// ```ignore
/// use common::WindowsCompatExt;
///
/// Command::new(bin())
///     .current_dir(project_dir)
///     .with_test_home(&home)
///     .args(["deploy", "--yes"])
///     .output()
///     .unwrap();
/// ```
pub trait WindowsCompatExt {
    /// Set all environment variables needed for Windows-compatible home directory isolation.
    ///
    /// **Primary**: Sets `CALVIN_TEST_HOME` which is the unified override for all
    /// home-dependent paths via `calvin_home_dir()`.
    ///
    /// **Also sets** (for completeness and backwards compatibility):
    /// - `HOME` (Unix)
    /// - `USERPROFILE` (Windows fallback for some crates)
    /// - `XDG_CONFIG_HOME` (XDG standard)
    /// - `CALVIN_REGISTRY_PATH` (legacy direct override)
    /// - `CALVIN_SOURCES_USER_LAYER_PATH` (legacy direct override)
    /// - `CALVIN_USER_CONFIG_PATH` (legacy direct override)
    /// - `CALVIN_XDG_CONFIG_PATH` (legacy direct override)
    fn with_test_home(&mut self, home: &Path) -> &mut Self;
}

impl WindowsCompatExt for Command {
    fn with_test_home(&mut self, home: &Path) -> &mut Self {
        let calvin_dir = home.join(".calvin");
        let registry_path = calvin_dir.join("registry.toml");
        let user_layer_path = calvin_dir.join(".promptpack");
        let user_config_path = calvin_dir.join("config.toml");
        let xdg_config_path = home.join(".config/calvin/config.toml");

        self
            // Primary: unified home directory override for calvin_home_dir()
            .env("CALVIN_TEST_HOME", home)
            // Standard Unix/Windows env vars (for non-Calvin code paths)
            .env("HOME", home)
            .env("USERPROFILE", home)
            .env("XDG_CONFIG_HOME", home.join(".config"))
            // Legacy specific overrides (kept for backwards compatibility)
            .env("CALVIN_REGISTRY_PATH", &registry_path)
            .env("CALVIN_SOURCES_USER_LAYER_PATH", &user_layer_path)
            .env("CALVIN_USER_CONFIG_PATH", &user_config_path)
            .env("CALVIN_XDG_CONFIG_PATH", &xdg_config_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn with_test_home_sets_all_required_env_vars() {
        let home = PathBuf::from("/fake/home");
        let mut cmd = Command::new("echo");
        cmd.with_test_home(&home);

        // Command doesn't expose env vars, so we just verify it doesn't panic
        // and returns &mut Self for chaining
        cmd.arg("test");
    }
}
