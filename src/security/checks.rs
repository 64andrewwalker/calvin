//! Platform-specific security checks
//!
//! This module contains security validation logic for each supported platform.
//!
//! # Size Justification
//!
//! calvin-no-split: This file is intentionally kept as a single unit because:
//! - All 8 check functions follow the same pattern (check_xxx)
//! - They share common imports and helper constants
//! - Splitting by platform would create many tiny files (20-150 lines each)
//! - The current structure allows easy comparison between platforms

use std::path::Path;

use crate::config::{Config, SecurityMode};

use super::report::DoctorSink;
use super::types::{CheckStatus, SecurityCheck};
use super::{EXPECTED_PROMPT_COUNT, MCP_ALLOWLIST};

const SKILL_DANGEROUS_TOOLS: &[&str] = &[
    "rm", "sudo", "chmod", "chown", "curl", "wget", "nc", "netcat", "ssh", "scp", "rsync",
];

fn check_skills_dir(skills_dir: &Path, platform: &str, sink: &mut impl DoctorSink) {
    if !skills_dir.exists() {
        sink.add_pass(platform, "skills", "OK");
        return;
    }

    if !skills_dir.is_dir() {
        sink.add_error(
            platform,
            "skills",
            "Skills path exists but is not a directory",
            Some("Remove the file and redeploy skills"),
        );
        return;
    }

    let mut skill_count = 0usize;
    let mut missing = Vec::new();
    let mut dangerous = Vec::new();

    let entries = match std::fs::read_dir(skills_dir) {
        Ok(e) => e,
        Err(_) => {
            sink.add_warning(
                platform,
                "skills",
                "Cannot read skills directory",
                Some("Check file permissions"),
            );
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(id) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if id.starts_with('.') {
            continue;
        }

        skill_count += 1;
        let skill_md = path.join("SKILL.md");
        if !skill_md.exists() {
            missing.push(id.to_string());
            continue;
        }

        // Best-effort: parse allowed-tools from frontmatter.
        if let Ok(content) = std::fs::read_to_string(&skill_md) {
            if let Ok(extracted) = crate::parser::extract_frontmatter(&content, &skill_md) {
                if let Ok(fm) = crate::parser::parse_frontmatter(&extracted.yaml, &skill_md) {
                    for tool in fm.allowed_tools {
                        if SKILL_DANGEROUS_TOOLS.contains(&tool.as_str()) {
                            dangerous.push((id.to_string(), tool));
                        }
                    }
                }
            }
        }
    }

    if !missing.is_empty() {
        sink.add_error(
            platform,
            "skills",
            &format!("Missing SKILL.md in: {}", missing.join(", ")),
            Some("Re-run `calvin deploy` or fix the skill folders"),
        );
        return;
    }

    sink.add_pass(platform, "skills", &format!("{} skills", skill_count));

    for (id, tool) in dangerous {
        sink.add_warning(
            platform,
            "allowed_tools",
            &format!(
                "Skill '{}' uses '{}' in allowed-tools (security warning)",
                id, tool
            ),
            Some("Ensure this is intentional and safe"),
        );
    }
}

pub fn check_claude_code(
    root: &Path,
    mode: SecurityMode,
    config: &Config,
    sink: &mut impl DoctorSink,
) {
    let platform = "Claude Code";

    // Check .claude/commands/ exists
    let commands_dir = root.join(".claude/commands");
    if commands_dir.exists() {
        let count = std::fs::read_dir(&commands_dir)
            .map(|rd| rd.count())
            .unwrap_or(0);
        let msg = if count == 0 {
            "OK".to_string()
        } else {
            format!("{} synced", count)
        };
        sink.add_pass(platform, "commands", &msg);
    } else {
        sink.add_warning(
            platform,
            "commands",
            "No commands directory found",
            Some("Run `calvin deploy` to generate commands"),
        );
    }

    // Check .claude/settings.json exists with permissions.deny
    let settings_file = root.join(".claude/settings.json");
    if settings_file.exists() {
        sink.add_pass(platform, "settings", ".claude/settings.json exists");

        // Surface explicit opt-out in doctor output.
        if config.security.allow_naked && mode != SecurityMode::Yolo {
            sink.add_warning(
                platform,
                "allow_naked",
                "Security protections disabled (security.allow_naked = true)",
                Some("Re-enable protections by setting security.allow_naked = false"),
            );
        }

        let expected_patterns = crate::domain::policies::effective_claude_deny_patterns(config);

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
                    sink.add_pass(
                        platform,
                        "deny_list",
                        "permissions.deny not required by configuration",
                    );
                    return;
                }

                if deny_list.is_none() {
                    match mode {
                        SecurityMode::Strict => sink.add_error(
                            platform,
                            "deny_list",
                            "permissions.deny not configured",
                            Some(
                                "Run `calvin deploy` or add permissions.deny to .claude/settings.json",
                            ),
                        ),
                        SecurityMode::Balanced => sink.add_warning(
                            platform,
                            "deny_list",
                            "permissions.deny not configured",
                            Some("Consider adding deny list for sensitive files"),
                        ),
                        SecurityMode::Yolo => sink.add_pass(
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
                    sink.add_pass(
                        platform,
                        "deny_list",
                        &format!(
                            "permissions.deny complete ({} patterns)",
                            deny_strings.len()
                        ),
                    );
                } else {
                    let missing_str = missing.join(", ");
                    match mode {
                        SecurityMode::Strict => sink.add_error(
                            platform,
                            "deny_list_incomplete",
                            &format!("Missing deny patterns: {}", missing_str),
                            Some(
                                "Run `calvin deploy` to regenerate baseline or add missing patterns",
                            ),
                        ),
                        SecurityMode::Balanced => sink.add_warning(
                            platform,
                            "deny_list_incomplete",
                            &format!("Missing deny patterns: {}", missing_str),
                            Some("Consider adding missing patterns for better security"),
                        ),
                        SecurityMode::Yolo => sink.add_pass(
                            platform,
                            "deny_list",
                            "Security checks disabled (yolo mode)",
                        ),
                    }
                }
            } else {
                sink.add_warning(
                    platform,
                    "deny_list",
                    "Invalid JSON in .claude/settings.json",
                    Some("Fix JSON syntax or regenerate with `calvin deploy`"),
                );
            }
        }
    } else if mode != SecurityMode::Yolo {
        sink.add_warning(
            platform,
            "settings",
            "No settings.json found",
            Some("Run `calvin deploy` to generate security baseline"),
        );
    }

    // Skills (project-scope)
    check_skills_dir(&root.join(".claude/skills"), platform, sink);
}

pub fn check_cursor(root: &Path, mode: SecurityMode, config: &Config, sink: &mut impl DoctorSink) {
    let platform = "Cursor";

    // Check .cursor/rules/ exists
    let rules_dir = root.join(".cursor/rules");
    if rules_dir.exists() {
        let count = std::fs::read_dir(&rules_dir)
            .map(|rd| rd.count())
            .unwrap_or(0);
        let msg = if count == 0 {
            "OK".to_string()
        } else {
            format!("{} synced", count)
        };
        sink.add_pass(platform, "rules", &msg);
    } else {
        sink.add_warning(
            platform,
            "rules",
            "No rules directory found",
            Some("Run `calvin deploy` to generate rules"),
        );
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

                    for (name, server_config) in servers {
                        // Check command against allowlist
                        let command = server_config
                            .get("command")
                            .and_then(|c| c.as_str())
                            .unwrap_or("");
                        let args = server_config
                            .get("args")
                            .and_then(|a| a.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str())
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            })
                            .unwrap_or_default();

                        let full_cmd = format!("{} {}", command, args);

                        // Check if any allowlist pattern matches
                        let is_allowed = MCP_ALLOWLIST.iter().any(|pattern| {
                            command.contains(pattern)
                                || args.contains(pattern)
                                || full_cmd.contains(pattern)
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
                        sink.add_check(SecurityCheck {
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
                                sink.add_check(SecurityCheck {
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
                                sink.add_check(SecurityCheck {
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
                                sink.add_pass(platform, "mcp", "MCP checks disabled (yolo mode)");
                            }
                        }
                    }
                } else {
                    sink.add_pass(platform, "mcp", "MCP configuration found (no servers)");
                }
            } else {
                sink.add_warning(
                    platform,
                    "mcp",
                    "Invalid mcp.json format",
                    Some("Check mcp.json for JSON syntax errors"),
                );
            }
        }
    }
}

pub fn check_vscode(root: &Path, _mode: SecurityMode, sink: &mut impl DoctorSink) {
    let platform = "VS Code";

    // Check .github/copilot-instructions.md exists
    let instructions = root.join(".github/copilot-instructions.md");
    if instructions.exists() {
        sink.add_pass(
            platform,
            "instructions",
            ".github/copilot-instructions.md exists",
        );
    } else {
        sink.add_warning(
            platform,
            "instructions",
            "No copilot-instructions.md found",
            Some("Run `calvin deploy` to generate instructions"),
        );
    }

    // Check AGENTS.md
    let agents = root.join("AGENTS.md");
    if agents.exists() {
        sink.add_pass(platform, "agents_md", "AGENTS.md exists");
    }
}

pub fn check_antigravity(root: &Path, mode: SecurityMode, sink: &mut impl DoctorSink) {
    let platform = "Antigravity";

    // Check .agent/rules/ exists
    let rules_dir = root.join(".agent/rules");
    if rules_dir.exists() {
        let count = std::fs::read_dir(&rules_dir)
            .map(|rd| rd.count())
            .unwrap_or(0);
        let msg = if count == 0 {
            "OK".to_string()
        } else {
            format!("{} synced", count)
        };
        sink.add_pass(platform, "rules", &msg);
    } else {
        sink.add_warning(
            platform,
            "rules",
            "No rules directory found",
            Some("Run `calvin deploy` to generate rules"),
        );
    }

    // Check .agent/workflows/ exists
    let workflows_dir = root.join(".agent/workflows");
    if workflows_dir.exists() {
        let count = std::fs::read_dir(&workflows_dir)
            .map(|rd| rd.count())
            .unwrap_or(0);
        let msg = if count == 0 {
            "OK".to_string()
        } else {
            format!("{} synced", count)
        };
        sink.add_pass(platform, "workflows", &msg);
    }

    // Turbo mode warning (would need to check user settings)
    if mode == SecurityMode::Strict {
        sink.add_warning(
            platform,
            "terminal_mode",
            "Cannot detect terminal mode from project",
            Some("Ensure Terminal mode is set to 'Auto' (not 'Turbo') in Antigravity settings"),
        );
    }
}

pub fn check_codex(root: &Path, _mode: SecurityMode, sink: &mut impl DoctorSink) {
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
        let msg = if count == 0 {
            "OK".to_string()
        } else {
            format!("{} synced", count)
        };
        sink.add_pass(platform, "prompts", &msg);
    } else {
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
                sink.add_pass(
                    platform,
                    "prompts",
                    &format!("User prompts installed ({} prompts)", count),
                );
            } else {
                sink.add_warning(
                    platform,
                    "prompts",
                    "No prompts directory found",
                    Some("Run `calvin deploy --home --targets codex` to install prompts"),
                );
            }
        } else {
            sink.add_warning(
                platform,
                "prompts",
                "Cannot determine home directory for Codex prompts",
                Some("Run `calvin deploy --home --targets codex` to install prompts"),
            );
        }
    }

    // Skills (prefer project-scope, fall back to user-scope)
    let project_skills = root.join(".codex/skills");
    if project_skills.exists() {
        check_skills_dir(&project_skills, platform, sink);
    } else if let Some(home) = dirs::home_dir() {
        check_skills_dir(&home.join(".codex/skills"), platform, sink);
    } else {
        sink.add_pass(platform, "skills", "OK");
    }
}

// === User-scope check functions ===

pub fn check_claude_code_user(
    home: &Path,
    _mode: SecurityMode,
    _config: &Config,
    sink: &mut impl DoctorSink,
) {
    let platform = "Claude Code (User)";

    // Check ~/.claude/commands/ exists
    let commands_dir = home.join(".claude/commands");
    if commands_dir.exists() {
        let count = std::fs::read_dir(&commands_dir)
            .map(|rd| {
                rd.filter_map(Result::ok)
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
                    .count()
            })
            .unwrap_or(0);
        if count > 0 {
            let msg = if count > EXPECTED_PROMPT_COUNT {
                format!(
                    "{} installed (+{} manual)",
                    count,
                    count - EXPECTED_PROMPT_COUNT
                )
            } else {
                format!("{} installed", count)
            };
            sink.add_pass(platform, "commands", &msg);
        }
    }
    // No warning for missing user dirs - they're optional
}

pub fn check_cursor_user(home: &Path, _mode: SecurityMode, sink: &mut impl DoctorSink) {
    let platform = "Cursor (User)";

    // Check ~/.cursor/rules/ exists
    let rules_dir = home.join(".cursor/rules");
    if rules_dir.exists() {
        let count = std::fs::read_dir(&rules_dir)
            .map(|rd| {
                rd.filter_map(Result::ok)
                    .filter(|e| {
                        e.path()
                            .extension()
                            .is_some_and(|ext| ext == "mdc" || ext == "md")
                    })
                    .count()
            })
            .unwrap_or(0);
        if count > 0 {
            let msg = if count > EXPECTED_PROMPT_COUNT {
                format!(
                    "{} installed (+{} manual)",
                    count,
                    count - EXPECTED_PROMPT_COUNT
                )
            } else {
                format!("{} installed", count)
            };
            sink.add_pass(platform, "rules", &msg);
        }
    }
    // No warning for missing user dirs - they're optional
}

pub fn check_antigravity_user(home: &Path, _mode: SecurityMode, sink: &mut impl DoctorSink) {
    let platform = "Antigravity (User)";

    // Check ~/.gemini/antigravity/global_workflows/ exists
    let workflows_dir = home.join(".gemini/antigravity/global_workflows");
    if workflows_dir.exists() {
        let count = std::fs::read_dir(&workflows_dir)
            .map(|rd| {
                rd.filter_map(Result::ok)
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
                    .count()
            })
            .unwrap_or(0);
        if count > 0 {
            let msg = if count > EXPECTED_PROMPT_COUNT {
                format!(
                    "{} installed (+{} manual)",
                    count,
                    count - EXPECTED_PROMPT_COUNT
                )
            } else {
                format!("{} installed", count)
            };
            sink.add_pass(platform, "workflows", &msg);
        }
    }
    // No warning for missing user dirs - they're optional
}
