//! Security Policy
//!
//! Domain policy for security-related decisions.
//! This policy encapsulates security rules without I/O operations.

use crate::domain::value_objects::SecurityMode;

/// Security policy for evaluating security rules
///
/// This policy determines how strict security checks should be
/// and what constitutes a violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecurityPolicy {
    mode: SecurityMode,
}

impl SecurityPolicy {
    /// Create a new security policy with the given mode
    pub fn new(mode: SecurityMode) -> Self {
        Self { mode }
    }

    /// Get the security mode
    pub fn mode(&self) -> SecurityMode {
        self.mode
    }

    /// Check if this is strict mode
    pub fn is_strict(&self) -> bool {
        matches!(self.mode, SecurityMode::Strict)
    }

    /// Check if this is yolo (permissive) mode
    pub fn is_yolo(&self) -> bool {
        matches!(self.mode, SecurityMode::Yolo)
    }

    /// Check if warnings should be treated as errors
    pub fn warnings_as_errors(&self) -> bool {
        self.is_strict()
    }

    /// Get the minimum required deny patterns for Claude Code
    pub fn required_deny_patterns(&self) -> Vec<&'static str> {
        match self.mode {
            SecurityMode::Yolo => vec![],
            SecurityMode::Balanced => vec!["**/.env", "**/.env.*", "**/secrets.*"],
            SecurityMode::Strict => vec![
                "**/.env",
                "**/.env.*",
                "**/secrets.*",
                "**/*.key",
                "**/*.pem",
                "**/id_rsa*",
                "**/credentials*",
            ],
        }
    }

    /// Check if an MCP server command is allowed
    pub fn is_mcp_allowed(&self, command: &str) -> bool {
        if self.is_yolo() {
            return true;
        }

        // Allowlist of known safe patterns
        const ALLOWLIST: &[&str] = &[
            "npx",
            "uvx",
            "node",
            "@anthropic/",
            "@modelcontextprotocol/",
            "mcp-server-",
        ];

        ALLOWLIST.iter().any(|pattern| command.contains(pattern))
    }

    /// Check if a file path should be denied access
    pub fn should_deny_file(&self, path: &str) -> bool {
        if self.is_yolo() {
            return false;
        }

        let patterns = self.required_deny_patterns();
        for pattern in patterns {
            if Self::glob_matches(pattern, path) {
                return true;
            }
        }
        false
    }

    /// Simple glob matching (supports ** and *)
    fn glob_matches(pattern: &str, path: &str) -> bool {
        // Simplified glob matching - full implementation would use glob crate
        if let Some(suffix) = pattern.strip_prefix("**/") {
            if let Some(ext) = suffix.strip_prefix("*.") {
                // Match by extension (e.g., *.key)
                return path.ends_with(&format!(".{}", ext));
            }
            if suffix.contains(".*") {
                // Match base name with any extension (e.g., secrets.*)
                let base = suffix.split(".*").next().unwrap_or("");
                let filename = path.rsplit('/').next().unwrap_or(path);
                return filename.starts_with(base) && filename.len() > base.len();
            }
            // Match by filename part
            return path.contains(suffix);
        }
        path.contains(pattern)
    }
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self::new(SecurityMode::Balanced)
    }
}

impl From<SecurityMode> for SecurityPolicy {
    fn from(mode: SecurityMode) -> Self {
        Self::new(mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_balanced() {
        let policy = SecurityPolicy::default();
        assert_eq!(policy.mode(), SecurityMode::Balanced);
    }

    #[test]
    fn strict_mode() {
        let policy = SecurityPolicy::new(SecurityMode::Strict);
        assert!(policy.is_strict());
        assert!(!policy.is_yolo());
    }

    #[test]
    fn yolo_mode() {
        let policy = SecurityPolicy::new(SecurityMode::Yolo);
        assert!(policy.is_yolo());
        assert!(!policy.is_strict());
    }

    #[test]
    fn warnings_as_errors_in_strict() {
        let strict = SecurityPolicy::new(SecurityMode::Strict);
        let balanced = SecurityPolicy::new(SecurityMode::Balanced);

        assert!(strict.warnings_as_errors());
        assert!(!balanced.warnings_as_errors());
    }

    #[test]
    fn required_patterns_yolo_empty() {
        let policy = SecurityPolicy::new(SecurityMode::Yolo);
        assert!(policy.required_deny_patterns().is_empty());
    }

    #[test]
    fn required_patterns_balanced_has_env() {
        let policy = SecurityPolicy::new(SecurityMode::Balanced);
        let patterns = policy.required_deny_patterns();
        assert!(patterns.iter().any(|p| p.contains(".env")));
    }

    #[test]
    fn required_patterns_strict_has_more() {
        let balanced = SecurityPolicy::new(SecurityMode::Balanced);
        let strict = SecurityPolicy::new(SecurityMode::Strict);

        assert!(strict.required_deny_patterns().len() > balanced.required_deny_patterns().len());
    }

    #[test]
    fn mcp_allowed_npx() {
        let policy = SecurityPolicy::new(SecurityMode::Balanced);
        assert!(policy.is_mcp_allowed("npx @anthropic/mcp-server"));
    }

    #[test]
    fn mcp_disallowed_unknown() {
        let policy = SecurityPolicy::new(SecurityMode::Balanced);
        assert!(!policy.is_mcp_allowed("some-random-command"));
    }

    #[test]
    fn mcp_allowed_in_yolo() {
        let policy = SecurityPolicy::new(SecurityMode::Yolo);
        assert!(policy.is_mcp_allowed("any-command"));
    }

    #[test]
    fn deny_env_file() {
        let policy = SecurityPolicy::new(SecurityMode::Balanced);
        assert!(policy.should_deny_file("project/.env"));
        assert!(policy.should_deny_file("project/.env.local"));
    }

    #[test]
    fn deny_secrets_file() {
        let policy = SecurityPolicy::new(SecurityMode::Balanced);
        assert!(policy.should_deny_file("config/secrets.json"));
    }

    #[test]
    fn deny_key_files_only_in_strict() {
        let balanced = SecurityPolicy::new(SecurityMode::Balanced);
        let strict = SecurityPolicy::new(SecurityMode::Strict);

        assert!(!balanced.should_deny_file("server.key"));
        assert!(strict.should_deny_file("server.key"));
    }

    #[test]
    fn yolo_denies_nothing() {
        let policy = SecurityPolicy::new(SecurityMode::Yolo);
        assert!(!policy.should_deny_file(".env"));
        assert!(!policy.should_deny_file("secrets.json"));
    }

    #[test]
    fn from_security_mode() {
        let policy: SecurityPolicy = SecurityMode::Strict.into();
        assert!(policy.is_strict());
    }
}
