//! Security module for doctor validation
//!
//! Implements security checks per the spec:
//! - Claude Code: permissions.deny exists with minimum patterns
//! - Antigravity: not in Turbo mode
//! - Cursor: MCP servers in allowlist

mod checks;
mod report;
#[cfg(test)]
mod tests;
mod types;

pub use report::{run_doctor, run_doctor_with_callback, DoctorReport, DoctorSink};
pub use types::{CheckStatus, SecurityCheck};

/// Known safe MCP server command patterns (allowlist)
pub(crate) const MCP_ALLOWLIST: &[&str] = &[
    "npx",                    // npm package executor (common for official MCP servers)
    "uvx",                    // Python uv package executor
    "node",                   // Node.js
    "@anthropic/",            // Official Anthropic MCP servers
    "@modelcontextprotocol/", // Official MCP servers
    "mcp-server-",            // Common MCP server naming pattern
];

/// Expected number of prompts from Calvin (matches typical .promptpack count)
pub(crate) const EXPECTED_PROMPT_COUNT: usize = 36;
