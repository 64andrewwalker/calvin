//! Target value object - defines which AI platform to compile for

use serde::{Deserialize, Serialize};

/// Target platform for compilation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Target {
    /// Claude Code (Anthropic)
    #[serde(alias = "claude")]
    ClaudeCode,
    /// Cursor IDE
    Cursor,
    /// VS Code with GitHub Copilot
    #[serde(alias = "vscode")]
    VSCode,
    /// Google Antigravity/Gemini
    Antigravity,
    /// OpenAI Codex CLI
    Codex,
    /// OpenCode (SST) terminal agent
    #[serde(rename = "opencode", alias = "open-code")]
    #[value(name = "opencode", alias = "open-code")]
    OpenCode,
    /// All platforms (meta-target, expands to all specific targets)
    All,
}

impl Target {
    /// All concrete targets (excluding `All`)
    pub const ALL_CONCRETE: [Target; 6] = [
        Target::ClaudeCode,
        Target::Cursor,
        Target::VSCode,
        Target::Antigravity,
        Target::Codex,
        Target::OpenCode,
    ];

    /// Returns true if this is the `All` meta-target
    pub fn is_all(&self) -> bool {
        matches!(self, Target::All)
    }

    /// Returns true if this target supports directory-based skills (`SKILL.md` folders).
    pub fn supports_skills(&self) -> bool {
        matches!(
            self,
            Target::ClaudeCode | Target::Cursor | Target::Codex | Target::OpenCode
        )
    }

    /// Expand `All` to concrete targets, or return self if already concrete
    pub fn expand(&self) -> Vec<Target> {
        if self.is_all() {
            Self::ALL_CONCRETE.to_vec()
        } else {
            vec![*self]
        }
    }

    /// Get the directory name for this target
    pub fn directory_name(&self) -> &'static str {
        match self {
            Target::ClaudeCode => ".claude",
            Target::Cursor => ".cursor",
            Target::VSCode => ".vscode",
            Target::Antigravity => ".gemini",
            Target::Codex => ".codex",
            Target::OpenCode => ".opencode",
            Target::All => ".all", // Should not be used directly
        }
    }

    /// Get a human-readable display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Target::ClaudeCode => "Claude Code",
            Target::Cursor => "Cursor",
            Target::VSCode => "VS Code",
            Target::Antigravity => "Antigravity",
            Target::Codex => "Codex",
            Target::OpenCode => "OpenCode",
            Target::All => "All",
        }
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl Target {
    /// All valid target names including aliases (for error messages)
    /// Users can use any of these names in config files
    pub const VALID_NAMES: &'static [&'static str] = &[
        "claude-code",
        "claude", // alias for claude-code
        "cursor",
        "vscode",
        "vs-code", // alias for vscode
        "antigravity",
        "codex",
        "opencode",
        "open-code", // alias for opencode
        "all",
    ];

    /// Canonical names (without aliases, for documentation)
    pub const CANONICAL_NAMES: &'static [&'static str] = &[
        "claude-code",
        "cursor",
        "vscode",
        "antigravity",
        "codex",
        "opencode",
        "all",
    ];

    /// Parse a target name with helpful error message
    ///
    /// Accepts various aliases:
    /// - `claude` or `claude-code` → ClaudeCode
    /// - `vscode` or `vs-code` → VSCode
    pub fn from_str_with_suggestion(s: &str) -> Result<Target, TargetParseError> {
        match s.trim().to_lowercase().as_str() {
            "claude-code" | "claudecode" | "claude" => Ok(Target::ClaudeCode),
            "cursor" => Ok(Target::Cursor),
            "vscode" | "vs-code" => Ok(Target::VSCode),
            "antigravity" => Ok(Target::Antigravity),
            "codex" => Ok(Target::Codex),
            "opencode" | "open-code" | "open_code" => Ok(Target::OpenCode),
            "all" => Ok(Target::All),
            _ => {
                let suggestion = Self::suggest_target(s);
                Err(TargetParseError {
                    invalid: s.to_string(),
                    suggestion,
                })
            }
        }
    }

    /// Suggest a valid target name based on typo
    fn suggest_target(input: &str) -> Option<String> {
        let input_lower = input.to_lowercase();

        // Common typos and shortcuts
        let aliases: &[(&str, &str)] = &[
            ("claude code", "claude-code"),
            ("code", "claude-code"),
            ("vs", "vscode"),
            ("vsc", "vscode"),
            ("anti", "antigravity"),
            ("gravity", "antigravity"),
            ("gemini", "antigravity"),
            ("open code", "opencode"),
            ("open", "opencode"),
        ];

        for (typo, correct) in aliases {
            if input_lower == *typo {
                return Some(correct.to_string());
            }
        }

        // Levenshtein distance for other typos
        let mut best: Option<(&str, usize)> = None;
        for valid in Self::CANONICAL_NAMES {
            let dist = levenshtein(&input_lower, valid);
            match best {
                None => best = Some((valid, dist)),
                Some((_, best_dist)) if dist < best_dist => best = Some((valid, dist)),
                _ => {}
            }
        }

        match best {
            Some((name, dist)) if dist <= 3 => Some(name.to_string()),
            _ => None,
        }
    }
}

/// Error when parsing an invalid target name
#[derive(Debug, Clone)]
pub struct TargetParseError {
    /// The invalid value provided
    pub invalid: String,
    /// A suggested valid target name, if applicable
    pub suggestion: Option<String>,
}

impl std::fmt::Display for TargetParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid target '{}'. ", self.invalid)?;
        if let Some(ref suggestion) = self.suggestion {
            write!(f, "Did you mean '{}'? ", suggestion)?;
        }
        write!(f, "Valid targets: {}", Target::VALID_NAMES.join(", "))
    }
}

impl std::error::Error for TargetParseError {}

/// Simple Levenshtein distance for typo detection
fn levenshtein(a: &str, b: &str) -> usize {
    if a == b {
        return 0;
    }

    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();

    let mut prev: Vec<usize> = (0..=b_bytes.len()).collect();
    let mut curr = vec![0usize; b_bytes.len() + 1];

    for (i, &ac) in a_bytes.iter().enumerate() {
        curr[0] = i + 1;
        for (j, &bc) in b_bytes.iter().enumerate() {
            let cost = if ac == bc { 0 } else { 1 };
            curr[j + 1] =
                std::cmp::min(std::cmp::min(prev[j + 1] + 1, curr[j] + 1), prev[j] + cost);
        }
        prev.clone_from_slice(&curr);
    }

    prev[b_bytes.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_all_concrete_has_6_targets() {
        assert_eq!(Target::ALL_CONCRETE.len(), 6);
    }

    #[test]
    fn target_is_all() {
        assert!(Target::All.is_all());
        assert!(!Target::ClaudeCode.is_all());
    }

    #[test]
    fn target_expand_all() {
        let expanded = Target::All.expand();
        assert_eq!(expanded.len(), 6);
    }

    #[test]
    fn target_expand_concrete() {
        let expanded = Target::Cursor.expand();
        assert_eq!(expanded, vec![Target::Cursor]);
    }

    #[test]
    fn target_directory_names() {
        assert_eq!(Target::ClaudeCode.directory_name(), ".claude");
        assert_eq!(Target::Cursor.directory_name(), ".cursor");
        assert_eq!(Target::VSCode.directory_name(), ".vscode");
    }

    #[test]
    fn target_display_names() {
        assert_eq!(Target::ClaudeCode.display_name(), "Claude Code");
        assert_eq!(Target::Cursor.display_name(), "Cursor");
    }

    #[test]
    fn target_serde_kebab_case() {
        let json = "\"claude-code\"";
        let target: Target = serde_json::from_str(json).unwrap();
        assert_eq!(target, Target::ClaudeCode);
    }

    #[test]
    fn target_serde_vscode_alias() {
        let json = "\"vscode\"";
        let target: Target = serde_json::from_str(json).unwrap();
        assert_eq!(target, Target::VSCode);
    }

    #[test]
    fn target_from_str_with_suggestion_valid() {
        assert_eq!(
            Target::from_str_with_suggestion("claude-code").unwrap(),
            Target::ClaudeCode
        );
        assert_eq!(
            Target::from_str_with_suggestion("cursor").unwrap(),
            Target::Cursor
        );
        assert_eq!(
            Target::from_str_with_suggestion("vscode").unwrap(),
            Target::VSCode
        );
        assert_eq!(
            Target::from_str_with_suggestion("vs-code").unwrap(),
            Target::VSCode
        );
        assert_eq!(
            Target::from_str_with_suggestion("antigravity").unwrap(),
            Target::Antigravity
        );
        assert_eq!(
            Target::from_str_with_suggestion("codex").unwrap(),
            Target::Codex
        );
        assert_eq!(
            Target::from_str_with_suggestion("opencode").unwrap(),
            Target::OpenCode
        );
        assert_eq!(
            Target::from_str_with_suggestion("all").unwrap(),
            Target::All
        );
    }

    #[test]
    fn target_from_str_with_suggestion_claude_alias() {
        // "claude" is now a valid alias for "claude-code"
        let target = Target::from_str_with_suggestion("claude").unwrap();
        assert_eq!(target, Target::ClaudeCode);
    }

    #[test]
    fn target_from_str_with_suggestion_typo() {
        // "cursr" (typo) should suggest "cursor"
        let err = Target::from_str_with_suggestion("cursr").unwrap_err();
        assert_eq!(err.invalid, "cursr");
        assert_eq!(err.suggestion, Some("cursor".to_string()));
    }

    #[test]
    fn target_from_str_with_suggestion_invalid() {
        let err = Target::from_str_with_suggestion("foobar").unwrap_err();
        assert_eq!(err.invalid, "foobar");
        // Error message should list valid targets
        let msg = err.to_string();
        assert!(
            msg.contains("Valid targets:"),
            "error should list valid targets: {}",
            msg
        );
    }

    #[test]
    fn target_supports_skills() {
        assert!(Target::ClaudeCode.supports_skills());
        assert!(Target::Cursor.supports_skills());
        assert!(Target::Codex.supports_skills());
        assert!(Target::OpenCode.supports_skills());
        assert!(!Target::VSCode.supports_skills());
        assert!(!Target::Antigravity.supports_skills());
    }

    #[test]
    #[allow(non_snake_case)] // naming convention: `<original_test_name>__<variant_type>`
    fn target_supports_skills__all_is_false() {
        assert!(!Target::All.supports_skills());
    }
}
