//! Security module for doctor validation
//!
//! Implements security checks per the spec:
//! - Claude Code: permissions.deny exists with minimum patterns
//! - Antigravity: not in Turbo mode
//! - Cursor: MCP servers in allowlist

use std::path::Path;

use crate::config::SecurityMode;

/// Minimum deny patterns that must be present (mirrors claude_code.rs)
const MINIMUM_DENY_PATTERNS: &[&str] = &[
    ".env",
    "*.pem",
    "*.key",
    "id_rsa",
    ".git/",
];

/// Known safe MCP server command patterns (allowlist)
const MCP_ALLOWLIST: &[&str] = &[
    "npx",           // npm package executor (common for official MCP servers)
    "uvx",           // Python uv package executor
    "node",          // Node.js
    "@anthropic/",   // Official Anthropic MCP servers
    "@modelcontextprotocol/", // Official MCP servers
    "mcp-server-",   // Common MCP server naming pattern
];

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
        
        // Check for permissions.deny and completeness
        if let Ok(content) = std::fs::read_to_string(&settings_file) {
            if content.contains("permissions") && content.contains("deny") {
                // Parse and check completeness
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(deny_list) = parsed
                        .get("permissions")
                        .and_then(|p| p.get("deny"))
                        .and_then(|d| d.as_array())
                    {
                        let deny_strings: Vec<&str> = deny_list
                            .iter()
                            .filter_map(|v| v.as_str())
                            .collect();
                        
                        // Check for missing minimum patterns
                        let missing: Vec<&str> = MINIMUM_DENY_PATTERNS
                            .iter()
                            .filter(|pattern| !deny_strings.iter().any(|d| d.contains(*pattern)))
                            .copied()
                            .collect();
                        
                        if missing.is_empty() {
                            report.add_pass(platform, "deny_list", 
                                &format!("permissions.deny complete ({} patterns)", deny_strings.len()));
                        } else {
                            let missing_str = missing.join(", ");
                            match mode {
                                SecurityMode::Strict => {
                                    report.add_error(platform, "deny_list_incomplete",
                                        &format!("Missing deny patterns: {}", missing_str),
                                        Some("Add missing patterns to permissions.deny"));
                                }
                                SecurityMode::Balanced => {
                                    report.add_warning(platform, "deny_list_incomplete",
                                        &format!("Missing deny patterns: {}", missing_str),
                                        Some("Consider adding missing patterns for better security"));
                                }
                                SecurityMode::Yolo => {
                                    report.add_pass(platform, "deny_list", 
                                        "Security checks disabled (yolo mode)");
                                }
                            }
                        }
                    } else {
                        report.add_pass(platform, "deny_list", "permissions.deny configured");
                    }
                } else {
                    report.add_pass(platform, "deny_list", "permissions.deny configured");
                }
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

    // Check for MCP config and validate servers against allowlist
    let mcp_file = root.join(".cursor/mcp.json");
    if mcp_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&mcp_file) {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(servers) = parsed.get("servers").and_then(|s| s.as_object()) {
                    let mut unknown_servers = Vec::new();
                    
                    for (name, config) in servers {
                        // Check command against allowlist
                        let command = config.get("command").and_then(|c| c.as_str()).unwrap_or("");
                        let args = config.get("args")
                            .and_then(|a| a.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" "))
                            .unwrap_or_default();
                        
                        let full_cmd = format!("{} {}", command, args);
                        
                        // Check if any allowlist pattern matches
                        let is_allowed = MCP_ALLOWLIST.iter().any(|pattern| {
                            command.contains(pattern) || args.contains(pattern) || full_cmd.contains(pattern)
                        });
                        
                        if !is_allowed {
                            unknown_servers.push(name.clone());
                        }
                    }
                    
                    if unknown_servers.is_empty() {
                        report.add_pass(platform, "mcp", 
                            &format!("MCP servers validated ({} servers)", servers.len()));
                    } else {
                        let unknown_str = unknown_servers.join(", ");
                        match mode {
                            SecurityMode::Strict => {
                                report.add_warning(platform, "mcp_unknown",
                                    &format!("Unknown MCP servers: {}", unknown_str),
                                    Some("Review these servers or add them to your project's config.toml allowlist"));
                            }
                            SecurityMode::Balanced => {
                                report.add_warning(platform, "mcp_unknown",
                                    &format!("Unknown MCP servers: {}", unknown_str),
                                    Some("Consider reviewing MCP server configurations"));
                            }
                            SecurityMode::Yolo => {
                                report.add_pass(platform, "mcp", "MCP checks disabled (yolo mode)");
                            }
                        }
                    }
                } else {
                    report.add_pass(platform, "mcp", "MCP configuration found (no servers)");
                }
            } else {
                report.add_warning(platform, "mcp", "Invalid mcp.json format",
                    Some("Check mcp.json for JSON syntax errors"));
            }
        }
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

    // === TDD: Deny List Completeness Tests (P2 Fix) ===

    #[test]
    fn test_deny_list_completeness_full() {
        let dir = tempdir().unwrap();
        
        // Create Claude Code settings with COMPLETE deny list
        fs::create_dir_all(dir.path().join(".claude")).unwrap();
        fs::write(
            dir.path().join(".claude/settings.json"),
            r#"{"permissions": {"deny": [".env", ".env.*", "*.pem", "*.key", "id_rsa", "id_ed25519", ".git/"]}}"#
        ).unwrap();
        
        let report = run_doctor(dir.path(), SecurityMode::Strict);
        
        // Should NOT have any error about missing deny patterns
        let deny_errors: Vec<_> = report.checks.iter()
            .filter(|c| c.name.contains("deny") && c.status == CheckStatus::Error)
            .collect();
        assert!(deny_errors.is_empty(), "Should pass with complete deny list");
    }

    #[test]
    fn test_deny_list_completeness_missing_patterns() {
        let dir = tempdir().unwrap();
        
        // Create Claude Code settings with INCOMPLETE deny list (missing .git/)
        fs::create_dir_all(dir.path().join(".claude")).unwrap();
        fs::write(
            dir.path().join(".claude/settings.json"),
            r#"{"permissions": {"deny": [".env"]}}"#
        ).unwrap();
        
        let report = run_doctor(dir.path(), SecurityMode::Strict);
        
        // Should have warning/error about missing deny patterns in strict mode
        let deny_checks: Vec<_> = report.checks.iter()
            .filter(|c| c.name.contains("deny") && c.status != CheckStatus::Pass)
            .collect();
        assert!(!deny_checks.is_empty(), "Should warn about incomplete deny list");
    }

    #[test]
    fn test_deny_list_completeness_balanced_mode() {
        let dir = tempdir().unwrap();
        
        // Create Claude Code settings with INCOMPLETE deny list
        fs::create_dir_all(dir.path().join(".claude")).unwrap();
        fs::write(
            dir.path().join(".claude/settings.json"),
            r#"{"permissions": {"deny": [".env"]}}"#
        ).unwrap();
        
        let report = run_doctor(dir.path(), SecurityMode::Balanced);
        
        // In balanced mode, missing patterns should be warnings, not errors
        let deny_errors: Vec<_> = report.checks.iter()
            .filter(|c| c.name.contains("deny") && c.status == CheckStatus::Error)
            .collect();
        assert!(deny_errors.is_empty(), "Balanced mode should not produce errors for incomplete deny list");
    }

    // === TDD: MCP Allowlist Validation Tests (P2 Fix) ===

    #[test]
    fn test_mcp_allowlist_valid_servers() {
        let dir = tempdir().unwrap();
        
        // Create Cursor MCP config with known safe servers
        fs::create_dir_all(dir.path().join(".cursor")).unwrap();
        fs::write(
            dir.path().join(".cursor/mcp.json"),
            r#"{"servers": {"filesystem": {"command": "npx", "args": ["-y", "@anthropic/mcp-server-filesystem"]}}}"#
        ).unwrap();
        
        let report = run_doctor(dir.path(), SecurityMode::Strict);
        
        // Should pass or warn, not error for known servers
        let mcp_errors: Vec<_> = report.checks.iter()
            .filter(|c| c.name.contains("mcp") && c.status == CheckStatus::Error)
            .collect();
        assert!(mcp_errors.is_empty(), "Known MCP servers should not produce errors");
    }

    #[test]
    fn test_mcp_allowlist_unknown_servers() {
        let dir = tempdir().unwrap();
        
        // Create Cursor MCP config with unknown/suspicious server
        fs::create_dir_all(dir.path().join(".cursor")).unwrap();
        fs::write(
            dir.path().join(".cursor/mcp.json"),
            r#"{"servers": {"evil": {"command": "/tmp/evil-hacker-script.sh"}}}"#
        ).unwrap();
        
        let report = run_doctor(dir.path(), SecurityMode::Strict);
        
        // Should have warning about unknown MCP server in strict mode
        let mcp_checks: Vec<_> = report.checks.iter()
            .filter(|c| c.name.contains("mcp") && c.status != CheckStatus::Pass)
            .collect();
        assert!(!mcp_checks.is_empty(), "Unknown MCP servers should trigger warnings in strict mode");
    }
}
