//! Doctor report and entry functions

use std::path::Path;

use crate::config::{Config, SecurityMode};

use super::checks;
use super::types::{CheckStatus, SecurityCheck};

/// Helper function to create a SecurityCheck
fn make_check(
    platform: &str,
    name: &str,
    status: CheckStatus,
    message: &str,
    recommendation: Option<&str>,
) -> SecurityCheck {
    SecurityCheck {
        name: name.to_string(),
        platform: platform.to_string(),
        status,
        message: message.to_string(),
        recommendation: recommendation.map(String::from),
        details: Vec::new(),
    }
}

pub trait DoctorSink {
    fn add_check(&mut self, check: SecurityCheck);

    fn add_pass(&mut self, platform: &str, name: &str, message: &str) {
        self.add_check(make_check(platform, name, CheckStatus::Pass, message, None));
    }

    fn add_warning(
        &mut self,
        platform: &str,
        name: &str,
        message: &str,
        recommendation: Option<&str>,
    ) {
        self.add_check(make_check(
            platform,
            name,
            CheckStatus::Warning,
            message,
            recommendation,
        ));
    }

    fn add_error(
        &mut self,
        platform: &str,
        name: &str,
        message: &str,
        recommendation: Option<&str>,
    ) {
        self.add_check(make_check(
            platform,
            name,
            CheckStatus::Error,
            message,
            recommendation,
        ));
    }
}

/// Doctor validation results
#[derive(Debug, Clone, Default)]
pub struct DoctorReport {
    pub checks: Vec<SecurityCheck>,
}

impl DoctorReport {
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    pub fn passes(&self) -> usize {
        self.checks
            .iter()
            .filter(|c| c.status == CheckStatus::Pass)
            .count()
    }

    pub fn warnings(&self) -> usize {
        self.checks
            .iter()
            .filter(|c| c.status == CheckStatus::Warning)
            .count()
    }

    pub fn errors(&self) -> usize {
        self.checks
            .iter()
            .filter(|c| c.status == CheckStatus::Error)
            .count()
    }

    pub fn is_success(&self) -> bool {
        self.errors() == 0
    }
}

impl DoctorSink for DoctorReport {
    fn add_check(&mut self, check: SecurityCheck) {
        self.checks.push(check);
    }
}

/// Run doctor checks for all platforms
/// Checks both project directory and user home directory
pub fn run_doctor(project_root: &Path, mode: SecurityMode) -> DoctorReport {
    let mut report = DoctorReport::new();

    // Load project config (used to reflect custom security rules in checks).
    let config_path = project_root.join(".promptpack/config.toml");
    let config = Config::load(&config_path).unwrap_or_default();

    run_doctor_into(project_root, mode, &config, &mut report);
    report
}

pub fn run_doctor_with_callback(
    project_root: &Path,
    mode: SecurityMode,
    mut on_check: impl FnMut(&SecurityCheck),
) -> DoctorReport {
    struct CallbackSink<F> {
        report: DoctorReport,
        on_check: F,
    }

    impl<F> CallbackSink<F> {
        fn new(on_check: F) -> Self {
            Self {
                report: DoctorReport::new(),
                on_check,
            }
        }
    }

    impl<F: FnMut(&SecurityCheck)> DoctorSink for CallbackSink<F> {
        fn add_check(&mut self, check: SecurityCheck) {
            (self.on_check)(&check);
            self.report.checks.push(check);
        }

        fn add_pass(&mut self, platform: &str, name: &str, message: &str) {
            self.add_check(SecurityCheck {
                name: name.to_string(),
                platform: platform.to_string(),
                status: CheckStatus::Pass,
                message: message.to_string(),
                recommendation: None,
                details: Vec::new(),
            });
        }

        fn add_warning(
            &mut self,
            platform: &str,
            name: &str,
            message: &str,
            recommendation: Option<&str>,
        ) {
            self.add_check(SecurityCheck {
                name: name.to_string(),
                platform: platform.to_string(),
                status: CheckStatus::Warning,
                message: message.to_string(),
                recommendation: recommendation.map(String::from),
                details: Vec::new(),
            });
        }

        fn add_error(
            &mut self,
            platform: &str,
            name: &str,
            message: &str,
            recommendation: Option<&str>,
        ) {
            self.add_check(SecurityCheck {
                name: name.to_string(),
                platform: platform.to_string(),
                status: CheckStatus::Error,
                message: message.to_string(),
                recommendation: recommendation.map(String::from),
                details: Vec::new(),
            });
        }
    }

    // Load project config (used to reflect custom security rules in checks).
    let config_path = project_root.join(".promptpack/config.toml");
    let config = Config::load(&config_path).unwrap_or_default();

    let mut sink = CallbackSink::new(|check: &SecurityCheck| on_check(check));
    run_doctor_into(project_root, mode, &config, &mut sink);
    sink.report
}

fn run_doctor_into(
    project_root: &Path,
    mode: SecurityMode,
    config: &Config,
    sink: &mut impl DoctorSink,
) {
    // === Project-scope checks ===
    // Claude Code checks
    checks::check_claude_code(project_root, mode, config, sink);

    // Cursor checks
    checks::check_cursor(project_root, mode, config, sink);

    // VS Code checks
    checks::check_vscode(project_root, mode, sink);

    // Antigravity checks
    checks::check_antigravity(project_root, mode, sink);

    // Codex checks (already handles both project and user scope)
    checks::check_codex(project_root, mode, sink);

    // === User-scope checks (home directory) ===
    if let Some(home) = dirs::home_dir() {
        // Only check user dirs if they exist (user may have deployed --home)
        checks::check_claude_code_user(&home, mode, config, sink);
        checks::check_cursor_user(&home, mode, sink);
        checks::check_antigravity_user(&home, mode, sink);
    }
}
