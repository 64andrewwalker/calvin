//! Test environment builder for isolated Calvin testing.
//!
//! Provides `TestEnv` - an isolated test environment with temp directories
//! for both project and home, plus helpers to run Calvin CLI commands.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

use super::windows::WindowsCompatExt;

/// Result of running a Calvin CLI command
#[derive(Debug)]
pub struct TestResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl TestResult {
    /// Check if command succeeded
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// Combine stdout and stderr
    pub fn combined_output(&self) -> String {
        format!("{}\n{}", self.stdout, self.stderr)
    }
}

/// Isolated test environment with temp directories.
///
/// Provides:
/// - Isolated project directory
/// - Isolated home directory (for user layer)
/// - Environment variable overrides
/// - CLI command execution helpers
pub struct TestEnv {
    /// Temporary directory for the project
    pub project_root: TempDir,
    /// Temporary directory for HOME
    pub home_dir: TempDir,
    /// Backup of original environment variables
    env_backup: HashMap<String, Option<String>>,
    /// Path to the calvin binary
    calvin_bin: PathBuf,
}

impl TestEnv {
    /// Create a new TestEnvBuilder
    pub fn builder() -> TestEnvBuilder {
        TestEnvBuilder::new()
    }

    /// Get path relative to project root
    pub fn project_path(&self, relative: &str) -> PathBuf {
        self.project_root.path().join(relative)
    }

    /// Get path relative to home directory
    pub fn home_path(&self, relative: &str) -> PathBuf {
        self.home_dir.path().join(relative)
    }

    /// Run calvin CLI in this environment from project root
    pub fn run(&self, args: &[&str]) -> TestResult {
        self.run_from(self.project_root.path(), args)
    }

    /// Run calvin CLI in this environment from project root with extra env vars.
    pub fn run_with_env(&self, args: &[&str], env_vars: &[(&str, &str)]) -> TestResult {
        self.run_from_with_env(self.project_root.path(), args, env_vars)
    }

    /// Run calvin CLI from a specific directory
    pub fn run_from(&self, cwd: &Path, args: &[&str]) -> TestResult {
        self.run_from_with_env(cwd, args, &[])
    }

    /// Run calvin CLI from a specific directory with extra env vars.
    pub fn run_from_with_env(
        &self,
        cwd: &Path,
        args: &[&str],
        env_vars: &[(&str, &str)],
    ) -> TestResult {
        let mut cmd = Command::new(&self.calvin_bin);
        cmd.current_dir(cwd)
            .args(args)
            .with_test_home(self.home_dir.path())
            .env("CALVIN_NO_COLOR", "1");

        for (key, value) in env_vars {
            cmd.env(key, value);
        }

        let output = cmd.output().expect("Failed to execute calvin");

        self.output_to_result(output)
    }

    /// Convert Command output to TestResult
    fn output_to_result(&self, output: Output) -> TestResult {
        TestResult {
            success: output.status.success(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    }

    /// Read the lockfile content from the project
    pub fn read_lockfile(&self) -> String {
        let lockfile_path = self.project_path("calvin.lock");
        if lockfile_path.exists() {
            return std::fs::read_to_string(&lockfile_path).unwrap_or_else(|_| String::new());
        }
        // Try alternate location
        let alt_path = self.project_path(".promptpack/calvin.lock");
        if alt_path.exists() {
            return std::fs::read_to_string(&alt_path).unwrap_or_else(|_| String::new());
        }
        String::new()
    }

    /// Read a deployed file's content
    pub fn read_deployed_file(&self, relative_path: &str) -> String {
        let full_path = self.project_path(relative_path);
        std::fs::read_to_string(&full_path)
            .unwrap_or_else(|e| panic!("Failed to read deployed file {}: {}", relative_path, e))
    }

    /// Write a file to the project directory
    pub fn write_project_file(&self, relative_path: &str, content: &str) {
        let full_path = self.project_path(relative_path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create directories");
        }
        std::fs::write(&full_path, content).expect("Failed to write file");
    }

    /// Write a file to the home directory
    pub fn write_home_file(&self, relative_path: &str, content: &str) {
        let full_path = self.home_path(relative_path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create directories");
        }
        std::fs::write(&full_path, content).expect("Failed to write file");
    }

    /// Remove a project asset
    pub fn remove_project_asset(&self, name: &str) {
        let asset_path = self.project_path(&format!(".promptpack/{}", name));
        if asset_path.exists() {
            std::fs::remove_file(&asset_path).expect("Failed to remove asset");
        }
    }

    /// Create subdirectories in the project
    pub fn create_subdirectories(&self, dirs: &[&str]) {
        for dir in dirs {
            let full_path = self.project_path(dir);
            std::fs::create_dir_all(&full_path).expect("Failed to create subdirectory");
        }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        // Restore original environment variables
        for (key, original) in &self.env_backup {
            match original {
                Some(val) => std::env::set_var(key, val),
                None => std::env::remove_var(key),
            }
        }
    }
}

/// Builder for TestEnv with fluent API
pub struct TestEnvBuilder {
    project_assets: Vec<(String, String)>,
    user_layer_assets: Vec<(String, String)>,
    additional_layer_assets: Vec<(String, Vec<(String, String)>)>,
    project_config: Option<String>,
    write_project_config: bool,
    home_config: Option<String>,
    user_promptpack_config: Option<String>,
    subdirectories: Vec<String>,
    create_project_promptpack: bool,
    create_user_layer: bool,
    init_git: bool,
    calvinignore: Option<String>,
}

impl TestEnvBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self {
            project_assets: Vec::new(),
            user_layer_assets: Vec::new(),
            additional_layer_assets: Vec::new(),
            project_config: None,
            write_project_config: true,
            home_config: None,
            user_promptpack_config: None,
            subdirectories: Vec::new(),
            create_project_promptpack: true,
            create_user_layer: false,
            init_git: true,
            calvinignore: None,
        }
    }

    /// Set .calvinignore content for the project layer
    pub fn with_calvinignore(mut self, content: &str) -> Self {
        self.calvinignore = Some(content.to_string());
        self
    }

    /// Add an asset to the project's .promptpack directory
    pub fn with_project_asset(mut self, name: &str, content: &str) -> Self {
        self.project_assets
            .push((name.to_string(), content.to_string()));
        self
    }

    /// Add an asset to the user layer (~/.calvin/.promptpack)
    pub fn with_user_asset(mut self, name: &str, content: &str) -> Self {
        self.create_user_layer = true;
        self.user_layer_assets
            .push((name.to_string(), content.to_string()));
        self
    }

    /// Alias for with_user_asset for compatibility with test plan
    pub fn with_user_layer_asset(self, name: &str, content: &str) -> Self {
        self.with_user_asset(name, content)
    }

    /// Set project config.toml content
    pub fn with_project_config(mut self, toml: &str) -> Self {
        self.project_config = Some(toml.to_string());
        self
    }

    /// Do not write `.promptpack/config.toml` for this project.
    pub fn without_project_config_file(mut self) -> Self {
        self.write_project_config = false;
        self
    }

    /// Set user config.toml content (~/.calvin/config.toml)
    pub fn with_home_config(mut self, toml: &str) -> Self {
        self.home_config = Some(toml.to_string());
        self
    }

    /// Set user-layer promptpack config.toml content (~/.calvin/.promptpack/config.toml)
    pub fn with_user_promptpack_config(mut self, toml: &str) -> Self {
        self.create_user_layer = true;
        self.user_promptpack_config = Some(toml.to_string());
        self
    }

    /// Create subdirectories in the project
    pub fn with_subdirectories(mut self, dirs: &[&str]) -> Self {
        self.subdirectories
            .extend(dirs.iter().map(|s| s.to_string()));
        self
    }

    /// Skip creating .promptpack in project
    pub fn without_project_promptpack(mut self) -> Self {
        self.create_project_promptpack = false;
        self
    }

    /// Skip creating user layer
    pub fn without_user_layer(mut self) -> Self {
        self.create_user_layer = false;
        self
    }

    /// Initialize as a fresh environment (no user layer, no project promptpack)
    pub fn fresh_environment(self) -> Self {
        self.without_user_layer().without_project_promptpack()
    }

    /// Skip git initialization
    pub fn without_git(mut self) -> Self {
        self.init_git = false;
        self
    }

    /// Build the TestEnv
    pub fn build(self) -> TestEnv {
        let project_root = TempDir::new().expect("Failed to create project temp dir");
        let home_dir = TempDir::new().expect("Failed to create home temp dir");

        // Find calvin binary
        let calvin_bin = Self::find_calvin_binary();

        // Initialize git repo if requested
        if self.init_git {
            Command::new("git")
                .current_dir(project_root.path())
                .args(["init", "-q"])
                .output()
                .ok();
        }

        // Create project .promptpack structure
        if self.create_project_promptpack || !self.project_assets.is_empty() {
            let promptpack_dir = project_root.path().join(".promptpack");
            std::fs::create_dir_all(&promptpack_dir).expect("Failed to create .promptpack");

            if self.write_project_config {
                // Write default config if not specified
                let config_content = self
                    .project_config
                    .as_deref()
                    .unwrap_or("[targets]\nenabled = [\"cursor\"]\n");
                std::fs::write(promptpack_dir.join("config.toml"), config_content)
                    .expect("Failed to write config.toml");
            }
        }

        // Write project assets
        for (name, content) in &self.project_assets {
            let asset_path = project_root.path().join(".promptpack").join(name);
            if let Some(parent) = asset_path.parent() {
                std::fs::create_dir_all(parent).expect("Failed to create asset directory");
            }
            std::fs::write(&asset_path, content).expect("Failed to write asset");
        }

        // Write .calvinignore if specified
        if let Some(calvinignore) = &self.calvinignore {
            let calvinignore_path = project_root.path().join(".promptpack/.calvinignore");
            std::fs::write(&calvinignore_path, calvinignore)
                .expect("Failed to write .calvinignore");
        }

        // Create user layer if needed
        if self.create_user_layer || !self.user_layer_assets.is_empty() {
            let user_promptpack = home_dir.path().join(".calvin/.promptpack");
            std::fs::create_dir_all(&user_promptpack).expect("Failed to create user layer");

            // Write user layer config
            let user_config_content = self
                .user_promptpack_config
                .as_deref()
                .unwrap_or("[targets]\nenabled = [\"cursor\"]\n");
            std::fs::write(user_promptpack.join("config.toml"), user_config_content)
                .expect("Failed to write user config.toml");
        }

        // Write user layer assets
        for (name, content) in &self.user_layer_assets {
            let asset_path = home_dir.path().join(".calvin/.promptpack").join(name);
            if let Some(parent) = asset_path.parent() {
                std::fs::create_dir_all(parent).expect("Failed to create user asset directory");
            }
            std::fs::write(&asset_path, content).expect("Failed to write user asset");
        }

        // Write home config if specified
        if let Some(config) = &self.home_config {
            let config_path = home_dir.path().join(".calvin/config.toml");
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent).expect("Failed to create .calvin directory");
            }
            std::fs::write(&config_path, config).expect("Failed to write home config");
        }

        // Create subdirectories
        for dir in &self.subdirectories {
            let full_path = project_root.path().join(dir);
            std::fs::create_dir_all(&full_path).expect("Failed to create subdirectory");
        }

        TestEnv {
            project_root,
            home_dir,
            env_backup: HashMap::new(),
            calvin_bin,
        }
    }

    /// Find the calvin binary to use for testing
    fn find_calvin_binary() -> PathBuf {
        // First try the target/debug path
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());

        let debug_bin = PathBuf::from(&manifest_dir).join("target/debug/calvin");
        if debug_bin.exists() {
            return debug_bin;
        }

        // Try release build
        let release_bin = PathBuf::from(&manifest_dir).join("target/release/calvin");
        if release_bin.exists() {
            return release_bin;
        }

        // Use assert_cmd's cargo_bin approach
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("debug")
            .join("calvin")
    }
}

impl Default for TestEnvBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_creates_project_promptpack() {
        let env = TestEnv::builder()
            .with_project_asset("test.md", "# Test")
            .build();

        assert!(env.project_path(".promptpack").exists());
        assert!(env.project_path(".promptpack/test.md").exists());
    }

    #[test]
    fn test_builder_creates_user_layer() {
        let env = TestEnv::builder()
            .with_user_asset("global.md", "# Global")
            .build();

        assert!(env.home_path(".calvin/.promptpack").exists());
        assert!(env.home_path(".calvin/.promptpack/global.md").exists());
    }

    #[test]
    fn test_fresh_environment_has_no_promptpack() {
        let env = TestEnv::builder().fresh_environment().build();

        assert!(!env.project_path(".promptpack").exists());
        assert!(!env.home_path(".calvin/.promptpack").exists());
    }
}
