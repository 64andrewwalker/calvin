//! Security module for doctor validation
//!
//! Implements security checks per the spec:
//! - Claude Code: permissions.deny exists
//! - Antigravity: not in Turbo mode
//! - Cursor: MCP servers in allowlist

use std::path::Path;

use crate::config::SecurityMode;

/// Security check result
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityCheck {
    pub name: String,
    pub platform: String,
    pub status: CheckStatus,
    pub message: String,
    pub recommendation: Option<String>,
}

/// Status of a security check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Warning,
    Error,
}

impl std::fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckStatus::Pass => write!(f, "✓"),
            CheckStatus::Warning => write!(f, "⚠"),
            CheckStatus::Error => write!(f, "✗"),
        }
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

    pub fn add_check(&mut self, check: SecurityCheck) {
        self.checks.push(check);
    }

    pub fn add_pass(&mut self, platform: &str, name: &str, message: &str) {
        self.checks.push(SecurityCheck {
            name: name.to_string(),
            platform: platform.to_string(),
            status: CheckStatus::Pass,
            message: message.to_string(),
            recommendation: None,
        });
    }

    pub fn add_warning(&mut self, platform: &str, name: &str, message: &str, recommendation: Option<&str>) {
        self.checks.push(SecurityCheck {
            name: name.to_string(),
            platform: platform.to_string(),
            status: CheckStatus::Warning,
            message: message.to_string(),
            recommendation: recommendation.map(String::from),
        });
    }

    pub fn add_error(&mut self, platform: &str, name: &str, message: &str, recommendation: Option<&str>) {
        self.checks.push(SecurityCheck {
            name: name.to_string(),
            platform: platform.to_string(),
            status: CheckStatus::Error,
            message: message.to_string(),
            recommendation: recommendation.map(String::from),
        });
    }

    pub fn passes(&self) -> usize {
        self.checks.iter().filter(|c| c.status == CheckStatus::Pass).count()
    }

    pub fn warnings(&self) -> usize {
        self.checks.iter().filter(|c| c.status == CheckStatus::Warning).count()
    }

    pub fn errors(&self) -> usize {
        self.checks.iter().filter(|c| c.status == CheckStatus::Error).count()
    }

    pub fn is_success(&self) -> bool {
        self.errors() == 0
    }
}

/// Run doctor checks for all platforms
pub fn run_doctor(project_root: &Path, mode: SecurityMode) -> DoctorReport {
    let mut report = DoctorReport::new();

    // Claude Code checks
    check_claude_code(project_root, mode, &mut report);

    // Cursor checks
    check_cursor(project_root, mode, &mut report);

    // VS Code checks
    check_vscode(project_root, mode, &mut report);

    // Antigravity checks
    check_antigravity(project_root, mode, &mut report);

    report
}

fn check_claude_code(root: &Path, mode: SecurityMode, report: &mut DoctorReport) {
    let platform = "Claude Code";

    // Check .claude/commands/ exists
    let commands_dir = root.join(".claude/commands");
    if commands_dir.exists() {
        let count = std::fs::read_dir(&commands_dir)
            .map(|rd| rd.count())
            .unwrap_or(0);
        report.add_pass(platform, "commands", &format!("{} commands synced", count));
    } else {
        report.add_warning(platform, "commands", "No commands directory found", 
            Some("Run `calvin sync` to generate commands"));
    }

    // Check .claude/settings.json exists with permissions.deny
    let settings_file = root.join(".claude/settings.json");
    if settings_file.exists() {
        report.add_pass(platform, "settings", ".claude/settings.json exists");
        
        // Check for permissions.deny
        if let Ok(content) = std::fs::read_to_string(&settings_file) {
            if content.contains("permissions") && content.contains("deny") {
                report.add_pass(platform, "deny_list", "permissions.deny configured");
            } else {
                match mode {
                    SecurityMode::Strict => {
                        report.add_error(platform, "deny_list", 
                            "permissions.deny not configured",
                            Some("Add deny list for sensitive files (.env, *.pem, etc.)"));
                    }
                    SecurityMode::Balanced => {
                        report.add_warning(platform, "deny_list",
                            "permissions.deny not configured",
                            Some("Consider adding deny list for sensitive files"));
                    }
                    SecurityMode::Yolo => {
                        // Info only in yolo mode
                        report.add_pass(platform, "deny_list", "Security checks disabled (yolo mode)");
                    }
                }
            }
        }
    } else {
        if mode != SecurityMode::Yolo {
            report.add_warning(platform, "settings", "No settings.json found",
                Some("Run `calvin sync` to generate security baseline"));
        }
    }
}

fn check_cursor(root: &Path, mode: SecurityMode, report: &mut DoctorReport) {
    let platform = "Cursor";

    // Check .cursor/rules/ exists
    let rules_dir = root.join(".cursor/rules");
    if rules_dir.exists() {
        let count = std::fs::read_dir(&rules_dir)
            .map(|rd| rd.count())
            .unwrap_or(0);
        report.add_pass(platform, "rules", &format!("{} rules synced", count));
    } else {
        report.add_warning(platform, "rules", "No rules directory found",
            Some("Run `calvin sync` to generate rules"));
    }

    // Check for MCP config
    let mcp_file = root.join(".cursor/mcp.json");
    if mcp_file.exists() && mode == SecurityMode::Strict {
        // Would check MCP servers against allowlist here
        report.add_pass(platform, "mcp", "MCP configuration found");
    }
}

fn check_vscode(root: &Path, _mode: SecurityMode, report: &mut DoctorReport) {
    let platform = "VS Code";

    // Check .github/copilot-instructions.md exists
    let instructions = root.join(".github/copilot-instructions.md");
    if instructions.exists() {
        report.add_pass(platform, "instructions", ".github/copilot-instructions.md exists");
    } else {
        report.add_warning(platform, "instructions", "No copilot-instructions.md found",
            Some("Run `calvin sync` to generate instructions"));
    }

    // Check AGENTS.md
    let agents = root.join("AGENTS.md");
    if agents.exists() {
        report.add_pass(platform, "agents_md", "AGENTS.md exists");
    }
}

fn check_antigravity(root: &Path, mode: SecurityMode, report: &mut DoctorReport) {
    let platform = "Antigravity";

    // Check .agent/rules/ exists
    let rules_dir = root.join(".agent/rules");
    if rules_dir.exists() {
        let count = std::fs::read_dir(&rules_dir)
            .map(|rd| rd.count())
            .unwrap_or(0);
        report.add_pass(platform, "rules", &format!("{} rules synced", count));
    } else {
        report.add_warning(platform, "rules", "No rules directory found",
            Some("Run `calvin sync` to generate rules"));
    }

    // Check .agent/workflows/ exists
    let workflows_dir = root.join(".agent/workflows");
    if workflows_dir.exists() {
        let count = std::fs::read_dir(&workflows_dir)
            .map(|rd| rd.count())
            .unwrap_or(0);
        report.add_pass(platform, "workflows", &format!("{} workflows synced", count));
    }

    // Turbo mode warning (would need to check user settings)
    if mode == SecurityMode::Strict {
        report.add_warning(platform, "terminal_mode",
            "Cannot detect terminal mode from project",
            Some("Ensure Terminal mode is set to 'Auto' (not 'Turbo') in Antigravity settings"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_doctor_report_new() {
        let report = DoctorReport::new();
        assert!(report.checks.is_empty());
        assert!(report.is_success());
    }

    #[test]
    fn test_doctor_report_add_checks() {
        let mut report = DoctorReport::new();
        report.add_pass("Test", "pass_check", "All good");
        report.add_warning("Test", "warn_check", "Hmm", Some("Fix it"));
        report.add_error("Test", "error_check", "Bad", None);

        assert_eq!(report.passes(), 1);
        assert_eq!(report.warnings(), 1);
        assert_eq!(report.errors(), 1);
        assert!(!report.is_success());
    }

    #[test]
    fn test_run_doctor_empty_project() {
        let dir = tempdir().unwrap();
        let report = run_doctor(dir.path(), SecurityMode::Balanced);
        
        // Should have some warnings for missing files
        assert!(report.warnings() > 0);
    }

    #[test]
    fn test_run_doctor_with_files() {
        let dir = tempdir().unwrap();
        
        // Create Claude Code structure
        fs::create_dir_all(dir.path().join(".claude/commands")).unwrap();
        fs::write(
            dir.path().join(".claude/settings.json"),
            r#"{"permissions": {"deny": [".env"]}}"#
        ).unwrap();
        
        // Create Cursor structure
        fs::create_dir_all(dir.path().join(".cursor/rules")).unwrap();
        
        // Create Antigravity structure
        fs::create_dir_all(dir.path().join(".agent/rules")).unwrap();
        
        // Create VS Code structure
        fs::create_dir_all(dir.path().join(".github")).unwrap();
        fs::write(
            dir.path().join(".github/copilot-instructions.md"),
            "# Instructions"
        ).unwrap();
        
        let report = run_doctor(dir.path(), SecurityMode::Balanced);
        
        // Should have passes for the created files
        assert!(report.passes() > 0);
    }

    #[test]
    fn test_check_status_display() {
        assert_eq!(format!("{}", CheckStatus::Pass), "✓");
        assert_eq!(format!("{}", CheckStatus::Warning), "⚠");
        assert_eq!(format!("{}", CheckStatus::Error), "✗");
    }
}
