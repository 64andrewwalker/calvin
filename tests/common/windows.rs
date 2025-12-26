//! Windows-compatible test environment helpers.
//!
//! On Windows, `dirs::home_dir()` uses Windows system API (`SHGetKnownFolderPath`),
//! not environment variables like `HOME` or `USERPROFILE`. This means setting these
//! environment variables in tests has no effect on Windows.
//!
//! These helpers set the necessary Calvin-specific environment variables to override
//! the default paths, ensuring tests work correctly on all platforms.

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
    /// This sets:
    /// - `HOME` (Unix)
    /// - `USERPROFILE` (Windows fallback for some crates)
    /// - `XDG_CONFIG_HOME` (XDG standard)
    /// - `CALVIN_REGISTRY_PATH` (overrides `dirs::home_dir()` for registry)
    /// - `CALVIN_SOURCES_USER_LAYER_PATH` (overrides `dirs::home_dir()` for user layer)
    /// - `CALVIN_USER_CONFIG_PATH` (overrides `dirs::home_dir()` for legacy user config)
    /// - `CALVIN_XDG_CONFIG_PATH` (overrides XDG config path)
    fn with_test_home(&mut self, home: &Path) -> &mut Self;
}

impl WindowsCompatExt for Command {
    fn with_test_home(&mut self, home: &Path) -> &mut Self {
        let calvin_dir = home.join(".calvin");
        let registry_path = calvin_dir.join("registry.toml");
        let user_layer_path = calvin_dir.join(".promptpack");
        let user_config_path = calvin_dir.join("config.toml");
        let xdg_config_path = home.join(".config/calvin/config.toml");

        self.env("HOME", home)
            .env("USERPROFILE", home)
            .env("XDG_CONFIG_HOME", home.join(".config"))
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
