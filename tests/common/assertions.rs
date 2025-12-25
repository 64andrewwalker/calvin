//! Custom assertion macros for contract and scenario tests.
//!
//! These macros provide descriptive failure messages to aid debugging.

use std::path::Path;

/// List all files in a directory recursively (for debugging)
pub fn list_all_files(dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                for sub in list_all_files(&path) {
                    files.push(sub);
                }
            } else {
                files.push(path.display().to_string());
            }
        }
    }
    files
}

/// Assert that a file was deployed to the expected location relative to project root.
///
/// # Example
/// ```ignore
/// assert_deployed!(env, ".cursor/rules/test.mdc");
/// ```
#[macro_export]
macro_rules! assert_deployed {
    ($env:expr, $path:expr) => {
        let full_path = $env.project_path($path);
        assert!(
            full_path.exists(),
            "Expected file at '{}', but it doesn't exist.\n\
             Project root: {:?}\n\
             Files found:\n  {}",
            $path,
            $env.project_root.path(),
            $crate::common::list_all_files($env.project_root.path()).join("\n  ")
        );
    };
}

/// Assert that a file was NOT deployed (should not exist).
///
/// # Example
/// ```ignore
/// assert_not_deployed!(env, ".claude/");
/// ```
#[macro_export]
macro_rules! assert_not_deployed {
    ($env:expr, $path:expr) => {
        let full_path = $env.project_path($path);
        assert!(
            !full_path.exists(),
            "Expected '{}' to NOT exist, but it does.\n\
             Project root: {:?}",
            $path,
            $env.project_root.path()
        );
    };
}

/// Assert that output (stdout or stderr) contains expected pattern.
///
/// # Example
/// ```ignore
/// assert_output_contains!(result, "Deploy Complete");
/// ```
#[macro_export]
macro_rules! assert_output_contains {
    ($result:expr, $pattern:expr) => {
        assert!(
            $result.stdout.contains($pattern) || $result.stderr.contains($pattern),
            "Expected output to contain '{}'\n\
             stdout:\n{}\n\
             stderr:\n{}",
            $pattern,
            $result.stdout,
            $result.stderr
        );
    };
}

/// Assert that output does NOT contain a pattern.
///
/// # Example
/// ```ignore
/// assert_output_not_contains!(result, "error");
/// ```
#[macro_export]
macro_rules! assert_output_not_contains {
    ($result:expr, $pattern:expr) => {
        assert!(
            !$result.stdout.contains($pattern) && !$result.stderr.contains($pattern),
            "Expected output to NOT contain '{}'\n\
             stdout:\n{}\n\
             stderr:\n{}",
            $pattern,
            $result.stdout,
            $result.stderr
        );
    };
}

/// Assert that no raw HOME path appears in output (should use tilde notation).
///
/// # Example
/// ```ignore
/// assert_no_raw_home_path!(result, env.home_dir.path());
/// ```
#[macro_export]
macro_rules! assert_no_raw_home_path {
    ($result:expr, $home:expr) => {
        let home_str = $home.display().to_string();
        assert!(
            !$result.stdout.contains(&home_str),
            "Raw HOME path leaked to stdout. Should use ~ notation.\n\
             HOME: {}\n\
             stdout:\n{}",
            home_str,
            $result.stdout
        );
    };
}

/// Assert that deployed file contains expected content.
///
/// # Example
/// ```ignore
/// assert_deployed_contains!(env, ".cursor/rules/test.mdc", "FROM_PROJECT");
/// ```
#[macro_export]
macro_rules! assert_deployed_contains {
    ($env:expr, $path:expr, $content:expr) => {
        let full_path = $env.project_path($path);
        assert!(
            full_path.exists(),
            "Cannot check content: file '{}' doesn't exist",
            $path
        );
        let file_content = std::fs::read_to_string(&full_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", $path, e));
        assert!(
            file_content.contains($content),
            "File '{}' does not contain expected content '{}'.\n\
             Actual content:\n{}",
            $path,
            $content,
            file_content
        );
    };
}
