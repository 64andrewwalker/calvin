//! Skill allowed-tools policy
//!
//! Single source of truth for identifying dangerous tools listed in skill frontmatter.

const DANGEROUS_SKILL_TOOLS: &[&str] = &[
    "rm", "sudo", "chmod", "chown", "curl", "wget", "nc", "netcat", "ssh", "scp", "rsync",
];

/// Returns true if the given tool (or its first whitespace-delimited token)
/// is considered dangerous when listed in `allowed-tools`.
pub fn is_dangerous_skill_tool(tool: &str) -> bool {
    let Some(tool) = tool.split_whitespace().next() else {
        return false;
    };

    DANGEROUS_SKILL_TOOLS.contains(&tool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_dangerous_skill_tool_flags_rm() {
        assert!(is_dangerous_skill_tool("rm"));
    }

    #[test]
    #[allow(non_snake_case)] // naming convention: `<original_test_name>__<variant_type>`
    fn is_dangerous_skill_tool_flags_rm__with_args() {
        assert!(is_dangerous_skill_tool("rm -rf"));
    }

    #[test]
    fn is_dangerous_skill_tool_does_not_flag_git() {
        assert!(!is_dangerous_skill_tool("git"));
    }
}
