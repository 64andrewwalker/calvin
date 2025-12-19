//! Check Use Case
//!
//! This module defines the `CheckUseCase` which orchestrates security checks
//! across all deployed targets.

use crate::config::{Config, SecurityMode};
use anyhow::Result;
use std::path::Path;

/// Options for the check operation
#[derive(Debug, Clone, Default)]
pub struct CheckOptions {
    /// Security mode (yolo, balanced, strict)
    pub mode: SecurityMode,
    /// Treat warnings as errors
    pub strict_warnings: bool,
}

/// Result of a single security check
#[derive(Debug, Clone)]
pub struct CheckItem {
    /// Platform being checked (e.g., "claude-code", "cursor")
    pub platform: String,
    /// Name of the check
    pub name: String,
    /// Status of the check
    pub status: CheckStatus,
    /// Human-readable message
    pub message: String,
    /// Recommendation for fixing issues
    pub recommendation: Option<String>,
    /// Additional details
    pub details: Vec<String>,
}

/// Status of a check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Warning,
    Error,
}

/// Result of the check operation
#[derive(Debug, Clone, Default)]
pub struct CheckResult {
    /// All check items
    pub items: Vec<CheckItem>,
    /// Number of passed checks
    pub passed: usize,
    /// Number of warnings
    pub warnings: usize,
    /// Number of errors
    pub errors: usize,
}

impl CheckResult {
    /// Check if all checks passed (no errors)
    pub fn is_success(&self) -> bool {
        self.errors == 0
    }

    /// Check if all checks passed with no warnings
    pub fn is_clean(&self) -> bool {
        self.errors == 0 && self.warnings == 0
    }
}

/// Check Use Case
///
/// Orchestrates security checks for all deployed targets.
pub struct CheckUseCase {
    #[allow(dead_code)] // Will be used for custom security rules
    config: Config,
}

impl CheckUseCase {
    /// Create a new CheckUseCase
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Execute the check operation
    pub fn execute(&self, project_root: &Path, options: CheckOptions) -> Result<CheckResult> {
        use crate::security::{run_doctor, CheckStatus as LegacyStatus};

        let report = run_doctor(project_root, options.mode);

        let mut result = CheckResult::default();

        for check in &report.checks {
            let status = match check.status {
                LegacyStatus::Pass => CheckStatus::Pass,
                LegacyStatus::Warning => CheckStatus::Warning,
                LegacyStatus::Error => CheckStatus::Error,
            };

            match status {
                CheckStatus::Pass => result.passed += 1,
                CheckStatus::Warning => result.warnings += 1,
                CheckStatus::Error => result.errors += 1,
            }

            result.items.push(CheckItem {
                platform: check.platform.to_string(),
                name: check.name.to_string(),
                status,
                message: check.message.to_string(),
                recommendation: check.recommendation.clone(),
                details: check.details.clone(),
            });
        }

        Ok(result)
    }

    /// Execute with a callback for each check (for streaming UI)
    pub fn execute_with_callback<F>(
        &self,
        project_root: &Path,
        options: CheckOptions,
        mut on_check: F,
    ) -> Result<CheckResult>
    where
        F: FnMut(&CheckItem),
    {
        use crate::security::{run_doctor_with_callback, CheckStatus as LegacyStatus};

        let mut result = CheckResult::default();

        run_doctor_with_callback(project_root, options.mode, |check| {
            let status = match check.status {
                LegacyStatus::Pass => CheckStatus::Pass,
                LegacyStatus::Warning => CheckStatus::Warning,
                LegacyStatus::Error => CheckStatus::Error,
            };

            match status {
                CheckStatus::Pass => result.passed += 1,
                CheckStatus::Warning => result.warnings += 1,
                CheckStatus::Error => result.errors += 1,
            }

            let item = CheckItem {
                platform: check.platform.to_string(),
                name: check.name.to_string(),
                status,
                message: check.message.to_string(),
                recommendation: check.recommendation.clone(),
                details: check.details.clone(),
            };

            on_check(&item);
            result.items.push(item);
        });

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn check_options_default_is_balanced() {
        let options = CheckOptions::default();
        assert_eq!(options.mode, SecurityMode::Balanced);
        assert!(!options.strict_warnings);
    }

    #[test]
    fn check_result_default_is_empty() {
        let result = CheckResult::default();
        assert!(result.items.is_empty());
        assert_eq!(result.passed, 0);
        assert_eq!(result.warnings, 0);
        assert_eq!(result.errors, 0);
    }

    #[test]
    fn check_result_is_success_when_no_errors() {
        let mut result = CheckResult::default();
        result.passed = 5;
        result.warnings = 2;
        assert!(result.is_success());
    }

    #[test]
    fn check_result_not_success_when_errors() {
        let mut result = CheckResult::default();
        result.passed = 5;
        result.errors = 1;
        assert!(!result.is_success());
    }

    #[test]
    fn check_result_is_clean_when_no_warnings_or_errors() {
        let mut result = CheckResult::default();
        result.passed = 5;
        assert!(result.is_clean());
    }

    #[test]
    fn check_result_not_clean_when_warnings() {
        let mut result = CheckResult::default();
        result.passed = 5;
        result.warnings = 1;
        assert!(!result.is_clean());
    }

    #[test]
    fn check_use_case_executes() {
        let dir = tempdir().unwrap();
        let config = Config::default();
        let use_case = CheckUseCase::new(config);

        let options = CheckOptions::default();
        let result = use_case.execute(dir.path(), options).unwrap();

        // Should have some checks (at least the basic ones)
        assert!(result.items.len() > 0);
    }

    #[test]
    fn check_use_case_callback_receives_items() {
        let dir = tempdir().unwrap();
        let config = Config::default();
        let use_case = CheckUseCase::new(config);

        let options = CheckOptions::default();
        let mut callback_count = 0;

        let result = use_case
            .execute_with_callback(dir.path(), options, |_item| {
                callback_count += 1;
            })
            .unwrap();

        assert_eq!(callback_count, result.items.len());
    }
}
