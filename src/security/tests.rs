//! Tests for the security module

use super::report::{run_doctor, DoctorReport, DoctorSink};
use super::types::CheckStatus;
use crate::config::SecurityMode;
use std::fs;
use tempfile::tempdir;

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
        r#"{"permissions": {"deny": [".env"]}}"#,
    )
    .unwrap();

    // Create Cursor structure
    fs::create_dir_all(dir.path().join(".cursor/rules")).unwrap();

    // Create Antigravity structure
    fs::create_dir_all(dir.path().join(".agent/rules")).unwrap();

    // Create VS Code structure
    fs::create_dir_all(dir.path().join(".github")).unwrap();
    fs::write(
        dir.path().join(".github/copilot-instructions.md"),
        "# Instructions",
    )
    .unwrap();

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
    let deny_errors: Vec<_> = report
        .checks
        .iter()
        .filter(|c| c.name.contains("deny") && c.status == CheckStatus::Error)
        .collect();
    assert!(
        deny_errors.is_empty(),
        "Should pass with complete deny list"
    );
}

#[test]
fn test_deny_list_completeness_missing_patterns() {
    let dir = tempdir().unwrap();

    // Create Claude Code settings with INCOMPLETE deny list (missing .git/)
    fs::create_dir_all(dir.path().join(".claude")).unwrap();
    fs::write(
        dir.path().join(".claude/settings.json"),
        r#"{"permissions": {"deny": [".env"]}}"#,
    )
    .unwrap();

    let report = run_doctor(dir.path(), SecurityMode::Strict);

    // Should have warning/error about missing deny patterns in strict mode
    let deny_checks: Vec<_> = report
        .checks
        .iter()
        .filter(|c| c.name.contains("deny") && c.status != CheckStatus::Pass)
        .collect();
    assert!(
        !deny_checks.is_empty(),
        "Should warn about incomplete deny list"
    );
}

#[test]
fn test_deny_list_completeness_balanced_mode() {
    let dir = tempdir().unwrap();

    // Create Claude Code settings with INCOMPLETE deny list
    fs::create_dir_all(dir.path().join(".claude")).unwrap();
    fs::write(
        dir.path().join(".claude/settings.json"),
        r#"{"permissions": {"deny": [".env"]}}"#,
    )
    .unwrap();

    let report = run_doctor(dir.path(), SecurityMode::Balanced);

    // In balanced mode, missing patterns should be warnings, not errors
    let deny_errors: Vec<_> = report
        .checks
        .iter()
        .filter(|c| c.name.contains("deny") && c.status == CheckStatus::Error)
        .collect();
    assert!(
        deny_errors.is_empty(),
        "Balanced mode should not produce errors for incomplete deny list"
    );
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
    fs::write(
        dir.path().join(".claude/settings.json"),
        r#"{"permissions": {}}"#,
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
    let mcp_errors: Vec<_> = report
        .checks
        .iter()
        .filter(|c| c.name.contains("mcp") && c.status == CheckStatus::Error)
        .collect();
    assert!(
        mcp_errors.is_empty(),
        "Known MCP servers should not produce errors"
    );
}

#[test]
fn test_mcp_allowlist_unknown_servers() {
    let dir = tempdir().unwrap();

    // Create Cursor MCP config with unknown/suspicious server
    fs::create_dir_all(dir.path().join(".cursor")).unwrap();
    fs::write(
        dir.path().join(".cursor/mcp.json"),
        r#"{"servers": {"evil": {"command": "/tmp/evil-hacker-script.sh"}}}"#,
    )
    .unwrap();

    let report = run_doctor(dir.path(), SecurityMode::Strict);

    // Should have warning about unknown MCP server in strict mode
    let mcp_checks: Vec<_> = report
        .checks
        .iter()
        .filter(|c| c.name.contains("mcp") && c.status != CheckStatus::Pass)
        .collect();
    assert!(
        !mcp_checks.is_empty(),
        "Unknown MCP servers should trigger warnings in strict mode"
    );
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
        .filter(|c| {
            c.platform == "Cursor" && c.name.contains("mcp") && c.status != CheckStatus::Pass
        })
        .collect();
    assert!(
        mcp_warnings.is_empty(),
        "additional_allowlist should suppress MCP warnings for allowed servers"
    );
}
