//! Security module for doctor validation
//!
//! Implements security checks per the spec:
//! - Claude Code: permissions.deny exists with minimum patterns
//! - Antigravity: not in Turbo mode
//! - Cursor: MCP servers in allowlist

use std::path::Path;

use crate::config::{Config, SecurityMode};

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
    pub details: Vec<String>,
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
            details: Vec::new(),
        });
    }

    pub fn add_warning(&mut self, platform: &str, name: &str, message: &str, recommendation: Option<&str>) {
        self.checks.push(SecurityCheck {
            name: name.to_string(),
            platform: platform.to_string(),
            status: CheckStatus::Warning,
            message: message.to_string(),
            recommendation: recommendation.map(String::from),
            details: Vec::new(),
        });
    }

    pub fn add_error(&mut self, platform: &str, name: &str, message: &str, recommendation: Option<&str>) {
        self.checks.push(SecurityCheck {
            name: name.to_string(),
            platform: platform.to_string(),
            status: CheckStatus::Error,
            message: message.to_string(),
            recommendation: recommendation.map(String::from),
            details: Vec::new(),
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

    // Load project config (used to reflect custom security rules in checks).
    let config_path = project_root.join(".promptpack/config.toml");
    let config = Config::load(&config_path).unwrap_or_default();

    // Claude Code checks
    check_claude_code(project_root, mode, &config, &mut report);

    // Cursor checks
    check_cursor(project_root, mode, &config, &mut report);

    // VS Code checks
    check_vscode(project_root, mode, &mut report);

    // Antigravity checks
    check_antigravity(project_root, mode, &mut report);

    // Codex checks
    check_codex(project_root, mode, &mut report);

    report
}

fn check_claude_code(root: &Path, mode: SecurityMode, config: &Config, report: &mut DoctorReport) {
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
            Some("Run `calvin deploy` to generate commands"));
    }

    // Check .claude/settings.json exists with permissions.deny
    let settings_file = root.join(".claude/settings.json");
    if settings_file.exists() {
        report.add_pass(platform, "settings", ".claude/settings.json exists");

        // Surface explicit opt-out in doctor output.
        if config.security.allow_naked && mode != SecurityMode::Yolo {
            report.add_warning(
                platform,
                "allow_naked",
                "Security protections disabled (security.allow_naked = true)",
                Some("Re-enable protections by setting security.allow_naked = false"),
            );
        }

        let expected_patterns = crate::security_baseline::effective_claude_deny_patterns(config);

        // Parse and validate deny list against expected patterns.
        if let Ok(content) = std::fs::read_to_string(&settings_file) {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                let deny_list = parsed
                    .get("permissions")
                    .and_then(|p| p.get("deny"))
                    .and_then(|d| d.as_array());

                let deny_strings: Vec<String> = match deny_list {
                    Some(arr) => arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect(),
                    None => Vec::new(),
                };

                // If no patterns are expected (e.g. allow_naked=true with no custom patterns),
                // we don't require permissions.deny to exist.
                if expected_patterns.is_empty() {
                    report.add_pass(
                        platform,
                        "deny_list",
                        "permissions.deny not required by configuration",
                    );
                    return;
                }

                if deny_list.is_none() {
                    match mode {
                        SecurityMode::Strict => report.add_error(
                            platform,
                            "deny_list",
                            "permissions.deny not configured",
                            Some("Run `calvin deploy` or add permissions.deny to .claude/settings.json"),
                        ),
                        SecurityMode::Balanced => report.add_warning(
                            platform,
                            "deny_list",
                            "permissions.deny not configured",
                            Some("Consider adding deny list for sensitive files"),
                        ),
                        SecurityMode::Yolo => report.add_pass(
                            platform,
                            "deny_list",
                            "Security checks disabled (yolo mode)",
                        ),
                    }
                    return;
                }

                let deny_set: std::collections::HashSet<&str> =
                    deny_strings.iter().map(|s| s.as_str()).collect();
                let missing: Vec<&str> = expected_patterns
                    .iter()
                    .filter(|pattern| !deny_set.contains(pattern.as_str()))
                    .map(|s| s.as_str())
                    .collect();

                if missing.is_empty() {
                    report.add_pass(
                        platform,
                        "deny_list",
                        &format!("permissions.deny complete ({} patterns)", deny_strings.len()),
                    );
                } else {
                    let missing_str = missing.join(", ");
                    match mode {
                        SecurityMode::Strict => report.add_error(
                            platform,
                            "deny_list_incomplete",
                            &format!("Missing deny patterns: {}", missing_str),
                            Some("Run `calvin deploy` to regenerate baseline or add missing patterns"),
                        ),
                        SecurityMode::Balanced => report.add_warning(
                            platform,
                            "deny_list_incomplete",
                            &format!("Missing deny patterns: {}", missing_str),
                            Some("Consider adding missing patterns for better security"),
                        ),
                        SecurityMode::Yolo => report.add_pass(
                            platform,
                            "deny_list",
                            "Security checks disabled (yolo mode)",
                        ),
                    }
                }
            } else {
                report.add_warning(
                    platform,
                    "deny_list",
                    "Invalid JSON in .claude/settings.json",
                    Some("Fix JSON syntax or regenerate with `calvin deploy`"),
                );
            }
        }
    } else if mode != SecurityMode::Yolo {
        report.add_warning(platform, "settings", "No settings.json found",
            Some("Run `calvin deploy` to generate security baseline"));
    }
}

fn check_cursor(root: &Path, mode: SecurityMode, config: &Config, report: &mut DoctorReport) {
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
            Some("Run `calvin deploy` to generate rules"));
    }

    // Check for MCP config and validate servers against allowlist
    let mcp_file = root.join(".cursor/mcp.json");
    if mcp_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&mcp_file) {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                let servers = parsed
                    .get("servers")
                    .or_else(|| parsed.get("mcpServers"))
                    .and_then(|s| s.as_object());

                if let Some(servers) = servers {
                    let mut unknown_servers = Vec::new();
                    let custom_allowlist: Vec<&str> = config
                        .security
                        .mcp
                        .allowlist
                        .iter()
                        .chain(config.security.mcp.additional_allowlist.iter())
                        .map(|s| s.as_str())
                        .collect();

                    let mut details = Vec::new();
                    
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

                        let is_custom_allowed = !custom_allowlist.is_empty()
                            && custom_allowlist.iter().any(|pattern| {
                                name.contains(pattern)
                                    || command.contains(*pattern)
                                    || args.contains(*pattern)
                                    || full_cmd.contains(*pattern)
                            });

                        if is_allowed {
                            details.push(format!("{}: built-in allowlist", name));
                        } else if is_custom_allowed {
                            details.push(format!("{}: custom allowlist", name));
                        } else {
                            details.push(format!("{}: unknown", name));
                        }
                        
                        if !is_allowed && !is_custom_allowed {
                            unknown_servers.push(name.clone());
                        }
                    }
                    
                    if unknown_servers.is_empty() {
                        report.add_check(SecurityCheck {
                            name: "mcp".to_string(),
                            platform: platform.to_string(),
                            status: CheckStatus::Pass,
                            message: format!("MCP servers validated ({} servers)", servers.len()),
                            recommendation: None,
                            details,
                        });
                    } else {
                        let unknown_str = unknown_servers.join(", ");
                        match mode {
                            SecurityMode::Strict => {
                                report.add_check(SecurityCheck {
                                    name: "mcp_unknown".to_string(),
                                    platform: platform.to_string(),
                                    status: CheckStatus::Warning,
                                    message: format!("Unknown MCP servers: {}", unknown_str),
                                    recommendation: Some(
                                        "Review these servers or add them to your project's config.toml allowlist"
                                            .to_string(),
                                    ),
                                    details,
                                });
                            }
                            SecurityMode::Balanced => {
                                report.add_check(SecurityCheck {
                                    name: "mcp_unknown".to_string(),
                                    platform: platform.to_string(),
                                    status: CheckStatus::Warning,
                                    message: format!("Unknown MCP servers: {}", unknown_str),
                                    recommendation: Some(
                                        "Consider reviewing MCP server configurations".to_string(),
                                    ),
                                    details,
                                });
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
            Some("Run `calvin deploy` to generate instructions"));
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
            Some("Run `calvin deploy` to generate rules"));
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

fn check_codex(root: &Path, _mode: SecurityMode, report: &mut DoctorReport) {
    let platform = "Codex";

    // Project-scope prompts
    let project_prompts = root.join(".codex/prompts");
    if project_prompts.exists() {
        let count = std::fs::read_dir(&project_prompts)
            .map(|rd| {
                rd.filter_map(Result::ok)
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
                    .count()
            })
            .unwrap_or(0);
        report.add_pass(platform, "prompts", &format!("{} prompts synced", count));
        return;
    }

    // User-scope prompts (most common for Codex CLI)
    if let Some(home) = dirs::home_dir() {
        let user_prompts = home.join(".codex/prompts");
        if user_prompts.exists() {
            let count = std::fs::read_dir(&user_prompts)
                .map(|rd| {
                    rd.filter_map(Result::ok)
                        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
                        .count()
                })
                .unwrap_or(0);
            report.add_pass(platform, "prompts", &format!("User prompts installed ({} prompts)", count));
        } else {
            report.add_warning(
                platform,
                "prompts",
                "No prompts directory found",
                Some("Run `calvin deploy --home --targets codex` to install prompts"),
            );
        }
    } else {
        report.add_warning(
            platform,
            "prompts",
            "Cannot determine home directory for Codex prompts",
            Some("Run `calvin deploy --home --targets codex` to install prompts"),
        );
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

    // === TDD: US-1 Configurable deny list (Sprint 1 / P0) ===

    #[test]
    fn test_deny_list_completeness_requires_env_dot_star_by_default() {
        let dir = tempdir().unwrap();

        // Missing `.env.*` should be flagged in strict mode by default.
        fs::create_dir_all(dir.path().join(".claude")).unwrap();
        fs::write(
            dir.path().join(".claude/settings.json"),
            r#"{"permissions": {"deny": [".env", "*.pem", "*.key", "id_rsa", "id_ed25519", ".git/"]}}"#,
        )
        .unwrap();

        let report = run_doctor(dir.path(), SecurityMode::Strict);

        let deny_issues: Vec<_> = report
            .checks
            .iter()
            .filter(|c| c.name.contains("deny") && c.status != CheckStatus::Pass)
            .collect();

        assert!(
            deny_issues.iter().any(|c| c.message.contains(".env.*")),
            "Strict mode should report missing `.env.*`"
        );
    }

    #[test]
    fn test_deny_list_exclude_can_remove_minimum_patterns() {
        let dir = tempdir().unwrap();

        // Project config excludes `.git/` from minimum deny list.
        fs::create_dir_all(dir.path().join(".promptpack")).unwrap();
        fs::write(
            dir.path().join(".promptpack/config.toml"),
            r#"
[security]
mode = "strict"

[security.deny]
exclude = [".git/", ".env.example"]
"#,
        )
        .unwrap();

        // Settings intentionally omit `.git/`.
        fs::create_dir_all(dir.path().join(".claude")).unwrap();
        fs::write(
            dir.path().join(".claude/settings.json"),
            r#"{"permissions": {"deny": [".env", ".env.*", "*.pem", "*.key", "id_rsa", "id_ed25519"]}}"#,
        )
        .unwrap();

        let report = run_doctor(dir.path(), SecurityMode::Strict);

        let deny_errors: Vec<_> = report
            .checks
            .iter()
            .filter(|c| c.name.contains("deny") && c.status == CheckStatus::Error)
            .collect();

        assert!(
            deny_errors.is_empty(),
            "Excluded minimum deny patterns should not produce strict-mode errors"
        );
    }

    #[test]
    fn test_allow_naked_disables_minimum_deny_requirements_in_doctor() {
        let dir = tempdir().unwrap();

        fs::create_dir_all(dir.path().join(".promptpack")).unwrap();
        fs::write(
            dir.path().join(".promptpack/config.toml"),
            r#"
[security]
allow_naked = true
"#,
        )
        .unwrap();

        // No deny list at all.
        fs::create_dir_all(dir.path().join(".claude")).unwrap();
        fs::write(dir.path().join(".claude/settings.json"), r#"{"permissions": {}}"#).unwrap();

        let report = run_doctor(dir.path(), SecurityMode::Strict);

        let deny_errors: Vec<_> = report
            .checks
            .iter()
            .filter(|c| c.name.contains("deny") && c.status == CheckStatus::Error)
            .collect();

        assert!(
            deny_errors.is_empty(),
            "allow_naked=true should prevent strict-mode deny list failures"
        );
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

    // === TDD: US-2 MCP allowlist extension (Sprint 2 / P1) ===

    #[test]
    fn test_mcp_allowlist_additional_allowlist_allows_unknown_server() {
        let dir = tempdir().unwrap();

        // Project config extends allowlist with internal server name.
        fs::create_dir_all(dir.path().join(".promptpack")).unwrap();
        fs::write(
            dir.path().join(".promptpack/config.toml"),
            r#"
[security.mcp]
additional_allowlist = ["internal-code-server"]
"#,
        )
        .unwrap();

        // Cursor MCP config with unknown server name/command.
        fs::create_dir_all(dir.path().join(".cursor")).unwrap();
        fs::write(
            dir.path().join(".cursor/mcp.json"),
            r#"{"servers": {"internal-code-server": {"command": "/usr/local/bin/internal-mcp"}}}"#,
        )
        .unwrap();

        let report = run_doctor(dir.path(), SecurityMode::Strict);

        // Should PASS MCP validation when server is in additional allowlist.
        let mcp_warnings: Vec<_> = report
            .checks
            .iter()
            .filter(|c| c.platform == "Cursor" && c.name.contains("mcp") && c.status != CheckStatus::Pass)
            .collect();
        assert!(
            mcp_warnings.is_empty(),
            "additional_allowlist should suppress MCP warnings for allowed servers"
        );
    }
}
