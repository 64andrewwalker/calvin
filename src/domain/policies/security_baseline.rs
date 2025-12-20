//! Security baseline generation helpers.
//!
//! This module centralizes the "minimum security" rules so both adapters and
//! `doctor/audit` can stay consistent.
//!
//! This is a pure domain policy - no I/O operations.

use crate::config::Config;

/// Minimum deny list (applied unless `security.allow_naked = true`).
pub const MINIMUM_DENY: &[&str] = &[
    ".env",
    ".env.*",
    "*.pem",
    "*.key",
    "id_rsa",
    "id_ed25519",
    ".git/",
];

/// Compute the effective deny patterns for Claude Code.
///
/// Rules:
/// - If `allow_naked = true`, do not inject `MINIMUM_DENY`.
/// - Always append `security.deny.patterns`.
/// - Apply `security.deny.exclude` by removing any deny pattern that would match
///   an excluded path (e.g. excluding `.env.example` removes `.env.*`).
pub fn effective_claude_deny_patterns(config: &Config) -> Vec<String> {
    let mut patterns: Vec<String> = Vec::new();

    if !config.security.allow_naked {
        patterns.extend(MINIMUM_DENY.iter().map(|s| s.to_string()));
    }

    patterns.extend(config.security.deny.patterns.iter().cloned());

    if !config.security.deny.exclude.is_empty() {
        patterns.retain(|deny_pattern| {
            !config
                .security
                .deny
                .exclude
                .iter()
                .any(|excluded_path| glob_matches(deny_pattern, excluded_path))
        });
    }

    patterns.sort();
    patterns.dedup();
    patterns
}

// Minimal glob matcher supporting `*` and `?`.
// We intentionally match over the whole string and treat `*` as matching any
// sequence (including path separators).
fn glob_matches(pattern: &str, text: &str) -> bool {
    let p = pattern.as_bytes();
    let t = text.as_bytes();

    let (mut pi, mut ti) = (0usize, 0usize);
    let (mut star_pi, mut star_ti) = (None::<usize>, 0usize);

    while ti < t.len() {
        if pi < p.len() && (p[pi] == t[ti] || p[pi] == b'?') {
            pi += 1;
            ti += 1;
            continue;
        }

        if pi < p.len() && p[pi] == b'*' {
            star_pi = Some(pi);
            star_ti = ti;
            pi += 1;
            continue;
        }

        if let Some(sp) = star_pi {
            pi = sp + 1;
            star_ti += 1;
            ti = star_ti;
            continue;
        }

        return false;
    }

    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }

    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effective_claude_deny_patterns_default_includes_minimum() {
        let config = Config::default();
        let patterns = effective_claude_deny_patterns(&config);
        assert!(patterns.contains(&".env".to_string()));
        assert!(patterns.contains(&".env.*".to_string()));
        assert!(patterns.contains(&".git/".to_string()));
    }

    #[test]
    fn test_effective_claude_deny_patterns_allow_naked_removes_minimum() {
        let mut config = Config::default();
        config.security.allow_naked = true;
        let patterns = effective_claude_deny_patterns(&config);
        assert!(!patterns.contains(&".env".to_string()));
        assert!(!patterns.contains(&".env.*".to_string()));
    }

    #[test]
    fn test_effective_claude_deny_patterns_exclude_removes_matching_pattern() {
        let mut config = Config::default();
        config.security.deny.exclude = vec![".env.example".to_string()];
        let patterns = effective_claude_deny_patterns(&config);
        assert!(!patterns.contains(&".env.*".to_string()));
        assert!(patterns.contains(&".env".to_string()));
    }

    // Additional tests for glob_matches
    #[test]
    fn test_glob_matches_exact() {
        assert!(glob_matches(".env", ".env"));
        assert!(!glob_matches(".env", ".envrc"));
    }

    #[test]
    fn test_glob_matches_wildcard() {
        assert!(glob_matches(".env.*", ".env.local"));
        assert!(glob_matches(".env.*", ".env.production"));
        assert!(!glob_matches(".env.*", ".env"));
    }

    #[test]
    fn test_glob_matches_question_mark() {
        assert!(glob_matches("id_?sa", "id_rsa"));
        assert!(glob_matches("id_?d25519", "id_ed25519"));
    }

    #[test]
    fn test_glob_matches_star_extension() {
        assert!(glob_matches("*.pem", "server.pem"));
        assert!(glob_matches("*.key", "private.key"));
    }
}
